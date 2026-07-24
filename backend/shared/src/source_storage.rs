use crate::error::RegistryError;
use sha2::{Digest, Sha256};
use std::env;
use std::fmt;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use crate::models::{SourceFormat, StorageBackend};

#[derive(Debug, Clone)]
pub struct SourceStorageConfig {
    pub backend: StorageBackend,
    pub local_root: PathBuf,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_prefix: Option<String>,
    pub s3_endpoint: Option<String>,
}

impl SourceStorageConfig {
    pub fn from_env() -> Result<Self, RegistryError> {
        let backend = match env::var("SOURCE_STORAGE_BACKEND")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase()
            .as_str()
        {
            "s3" => StorageBackend::S3,
            "gcs" => StorageBackend::Gcs,
            _ => StorageBackend::Local,
        };

        let local_root = env::var("SOURCE_STORAGE_LOCAL_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/source_storage"));

        let s3_bucket = env::var("SOURCE_STORAGE_BUCKET").ok();
        let s3_region = env::var("SOURCE_STORAGE_REGION").ok();
        let s3_prefix = env::var("SOURCE_STORAGE_PREFIX").ok();
        let s3_endpoint = env::var("SOURCE_STORAGE_ENDPOINT").ok();

        if matches!(backend, StorageBackend::S3 | StorageBackend::Gcs) && s3_bucket.is_none() {
            return Err(RegistryError::invalid_input(
                "SOURCE_STORAGE_BUCKET is required for S3/GCS backend".to_string(),
            ));
        }

        Ok(Self {
            backend,
            local_root,
            s3_bucket,
            s3_region,
            s3_prefix,
            s3_endpoint,
        })
    }
}

#[derive(Clone)]
pub struct SourceStorage {
    config: SourceStorageConfig,
    s3_bucket_client: Option<s3::bucket::Bucket>,
}

impl SourceStorage {
    pub async fn new() -> Result<Self, RegistryError> {
        let config = SourceStorageConfig::from_env()?;

        let s3_bucket_client = if matches!(config.backend, StorageBackend::S3 | StorageBackend::Gcs)
        {
            let region = config
                .s3_region
                .as_deref()
                .unwrap_or("us-east-1")
                .to_string();
            let bucket = config.s3_bucket.clone().ok_or_else(|| {
                RegistryError::invalid_input("SOURCE_STORAGE_BUCKET is required".to_string())
            })?;

            let credentials = s3::creds::Credentials::default()
                .map_err(|e| RegistryError::internal(format!("S3 credentials error: {}", e)))?;
            let s3_region = if let Some(ep) = config.s3_endpoint.clone() {
                s3::Region::Custom {
                    region: region.clone(),
                    endpoint: ep,
                }
            } else {
                s3::Region::Custom {
                    region: region.clone(),
                    endpoint: format!("https://s3.{}.amazonaws.com", region),
                }
            };
            let b = s3::Bucket::new(&bucket, s3_region, credentials)
                .map_err(|e| RegistryError::internal(format!("S3 bucket init error: {}", e)))?
                .with_path_style();
            Some(*b)
        } else {
            None
        };

        Ok(Self {
            config,
            s3_bucket_client,
        })
    }

    /// stores source, returns (storage_backend, storage_key)
    pub async fn store_source(
        &self,
        contract_id: &str,
        version: &str,
        format: SourceFormat,
        source_bytes: &[u8],
    ) -> Result<(String, String, String), RegistryError> {
        let source_hash = compute_sha256(source_bytes);
        let _source_size = source_bytes.len() as i64;

        let key = format!(
            "{}/{}/{}/{}.{}",
            contract_id,
            version,
            format,
            Uuid::new_v4(),
            "bin"
        );

        match self.config.backend {
            StorageBackend::Local => {
                let path = self
                    .config
                    .local_root
                    .join(contract_id)
                    .join(version)
                    .join(format.to_string());
                fs::create_dir_all(&path).await?;
                let file_path = path.join(format!("{}.bin", Uuid::new_v4()));
                fs::write(&file_path, source_bytes).await?;
                let key = file_path.to_string_lossy().into_owned();
                Ok(("local".to_string(), key, source_hash))
            }
            StorageBackend::S3 | StorageBackend::Gcs => {
                let bucket = self.s3_bucket_client.as_ref().ok_or_else(|| {
                    RegistryError::internal("S3 client not initialized".to_string())
                })?;

                let prefix = self
                    .config
                    .s3_prefix
                    .clone()
                    .unwrap_or_else(|| "contract_sources".to_string());
                let object_key = format!("{}/{}", prefix.trim_end_matches('/'), key);

                bucket
                    .put_object(&object_key, source_bytes)
                    .await
                    .map_err(|e| {
                        RegistryError::internal(format!(
                            "Failed to upload source artifact to S3/GCS: {}",
                            e
                        ))
                    })?;
                Ok((self.config.backend.to_string(), object_key, source_hash))
            }
        }
    }

    pub async fn retrieve_source(
        &self,
        storage_backend: &str,
        storage_key: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        match storage_backend {
            "local" => {
                let bytes = fs::read(storage_key).await?;
                Ok(bytes)
            }
            "s3" | "gcs" => {
                let bucket = self.s3_bucket_client.as_ref().ok_or_else(|| {
                    RegistryError::internal("S3/GCS bucket not initialized".to_string())
                })?;

                let data = bucket.get_object(storage_key).await.map_err(|e| {
                    RegistryError::internal(format!("S3/GCS get_object failed: {}", e))
                })?;
                Ok(data.to_vec())
            }
            other => Err(RegistryError::invalid_input(format!(
                "Unknown storage backend {}",
                other
            ))),
        }
    }
}

impl fmt::Display for SourceFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceFormat::Rust => write!(f, "rust"),
            SourceFormat::Wasm => write!(f, "wasm"),
        }
    }
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_source_storage_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SOURCE_STORAGE_BACKEND", "local");
        env::set_var("SOURCE_STORAGE_LOCAL_ROOT", temp_dir.path());

        let storage = SourceStorage::new().await.expect("init storage");
        let contract_id = "test_contract";
        let version = "1.0.0";

        let src = b"fn hello() { println!(\"hello\"); }";
        let (_backend, key, hash) = storage
            .store_source(contract_id, version, SourceFormat::Rust, src)
            .await
            .expect("store source");

        assert_eq!(hash, compute_sha256(src));
        let loaded = storage
            .retrieve_source("local", &key)
            .await
            .expect("read source");
        assert_eq!(loaded, src);
    }
}
