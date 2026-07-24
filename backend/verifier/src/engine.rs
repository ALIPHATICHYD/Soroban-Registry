//! Concurrent verification engine.
//!
//! Wraps the core `verify_contract` function with a worker-pool pattern backed
//! by a `tokio::sync::Semaphore` so that at most `max_concurrent` verifications
//! run simultaneously. Progress is tracked via an `Arc<AtomicUsize>`.

use crate::{verify_contract, VerificationResult};
use serde_json::Value;
use shared::RegistryError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A bounded concurrent verification engine.
///
/// # Example
/// ```rust,no_run
/// # use verifier::engine::VerificationEngine;
/// # tokio_test::block_on(async {
/// let engine = VerificationEngine::new(5);
/// let result = engine.verify("wasm_base64:...", "aabbcc...", None, None).await.unwrap();
/// # });
/// ```
#[derive(Clone)]
pub struct VerificationEngine {
    semaphore: Arc<Semaphore>,
    active: Arc<AtomicUsize>,
    queued: Arc<AtomicUsize>,
    completed: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
}

impl VerificationEngine {
    /// Create a new engine that allows at most `max_concurrent` simultaneous verifications.
    ///
    /// # Panics
    /// Panics if `max_concurrent` is 0.
    pub fn new(max_concurrent: usize) -> Self {
        assert!(max_concurrent > 0, "max_concurrent must be > 0");
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active: Arc::new(AtomicUsize::new(0)),
            queued: Arc::new(AtomicUsize::new(0)),
            completed: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Submit a single verification job.
    ///
    /// Blocks until a worker slot is available, then runs the verification.
    /// Progress counters are updated atomically throughout.
    pub async fn verify(
        &self,
        source_code: &str,
        deployed_wasm_hash: &str,
        compiler_version: Option<&str>,
        build_params: Option<&Value>,
    ) -> Result<VerificationResult, RegistryError> {
        self.queued.fetch_add(1, Ordering::Relaxed);

        // Acquire a permit — blocks if max_concurrent slots are all taken.
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| RegistryError::internal("semaphore closed".to_string()))?;

        self.queued.fetch_sub(1, Ordering::Relaxed);
        self.active.fetch_add(1, Ordering::Relaxed);

        let result = verify_contract(
            source_code,
            deployed_wasm_hash,
            compiler_version,
            build_params,
        )
        .await;

        self.active.fetch_sub(1, Ordering::Relaxed);
        match &result {
            Ok(_) => {
                self.completed.fetch_add(1, Ordering::Relaxed);
            }
            Err(_) => {
                self.failed.fetch_add(1, Ordering::Relaxed);
            }
        }

        result
    }

    /// Submit multiple verifications concurrently and collect all results.
    ///
    /// All jobs are spawned immediately; the semaphore bounds how many run at once.
    /// Results are returned in submission order.
    pub async fn verify_batch(
        &self,
        jobs: Vec<BatchJob>,
    ) -> Vec<Result<VerificationResult, RegistryError>> {
        let mut handles = Vec::with_capacity(jobs.len());

        for job in jobs {
            let engine = self.clone();
            let handle = tokio::spawn(async move {
                engine
                    .verify(
                        &job.source_code,
                        &job.deployed_wasm_hash,
                        job.compiler_version.as_deref(),
                        job.build_params.as_ref(),
                    )
                    .await
            });
            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(r) => results.push(r),
                Err(e) => results.push(Err(RegistryError::internal(format!("task panicked: {e}")))),
            }
        }
        results
    }

    /// Snapshot of current engine progress.
    pub fn progress(&self) -> EngineProgress {
        EngineProgress {
            active: self.active.load(Ordering::Relaxed),
            queued: self.queued.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
        }
    }
}

/// A single job for `verify_batch`.
pub struct BatchJob {
    pub source_code: String,
    pub deployed_wasm_hash: String,
    pub compiler_version: Option<String>,
    pub build_params: Option<Value>,
}

/// Point-in-time snapshot of engine progress counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineProgress {
    /// Jobs currently executing.
    pub active: usize,
    /// Jobs waiting for a worker slot.
    pub queued: usize,
    /// Jobs that completed successfully.
    pub completed: usize,
    /// Jobs that returned an error.
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_wasm;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    fn wasm_source(bytes: &[u8]) -> String {
        format!("wasm_base64:{}", BASE64.encode(bytes))
    }

    #[tokio::test]
    async fn single_verify_succeeds() {
        let engine = VerificationEngine::new(5);
        let wasm = b"test-wasm-bytes";
        let hash = hash_wasm(wasm);
        let result = engine
            .verify(&wasm_source(wasm), &hash, None, None)
            .await
            .unwrap();
        assert!(result.verified);
    }

    #[tokio::test]
    async fn progress_counters_update_correctly() {
        let engine = VerificationEngine::new(5);
        let wasm = b"counter-test";
        let hash = hash_wasm(wasm);
        engine
            .verify(&wasm_source(wasm), &hash, None, None)
            .await
            .unwrap();
        let p = engine.progress();
        assert_eq!(p.active, 0);
        assert_eq!(p.queued, 0);
        assert_eq!(p.completed, 1);
        assert_eq!(p.failed, 0);
    }

    #[tokio::test]
    async fn failed_job_increments_failed_counter() {
        let engine = VerificationEngine::new(5);
        // invalid hash triggers an error
        let _ = engine
            .verify("fn main(){}", "not-a-valid-hash", None, None)
            .await;
        let p = engine.progress();
        assert_eq!(p.failed, 1);
        assert_eq!(p.completed, 0);
    }

    #[tokio::test]
    async fn five_concurrent_verifications_no_corruption() {
        let engine = VerificationEngine::new(5);
        let jobs: Vec<BatchJob> = (0..5u8)
            .map(|i| {
                let wasm = vec![i; 32];
                let hash = hash_wasm(&wasm);
                BatchJob {
                    source_code: wasm_source(&wasm),
                    deployed_wasm_hash: hash,
                    compiler_version: None,
                    build_params: None,
                }
            })
            .collect();

        let results = engine.verify_batch(jobs).await;
        assert_eq!(results.len(), 5);
        for r in &results {
            assert!(
                r.as_ref().unwrap().verified,
                "each job should verify correctly"
            );
        }
        let p = engine.progress();
        assert_eq!(p.completed, 5);
        assert_eq!(p.failed, 0);
        assert_eq!(p.active, 0);
    }

    #[tokio::test]
    async fn semaphore_bounds_concurrency() {
        // Engine with max 2 slots; submit 6 jobs and ensure all complete.
        let engine = VerificationEngine::new(2);
        let jobs: Vec<BatchJob> = (0..6u8)
            .map(|i| {
                let wasm = vec![i + 10; 16];
                let hash = hash_wasm(&wasm);
                BatchJob {
                    source_code: wasm_source(&wasm),
                    deployed_wasm_hash: hash,
                    compiler_version: None,
                    build_params: None,
                }
            })
            .collect();

        let results = engine.verify_batch(jobs).await;
        assert_eq!(results.len(), 6);
        assert!(results.iter().all(|r| r.as_ref().unwrap().verified));
        assert_eq!(engine.progress().completed, 6);
    }
}
