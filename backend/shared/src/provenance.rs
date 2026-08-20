//! Build-provenance metadata for published contracts (#1140).
//!
//! Provenance records *how* a WASM artifact was produced (source repo,
//! commit, toolchain versions, build environment) so an independent party
//! can attempt to reproduce it. This module only models and validates that
//! metadata; it never builds anything and never treats metadata alone as
//! proof of reproducibility. Whether an artifact is actually reproducible is
//! determined solely by an independent rebuild whose WASM hash matches the
//! recorded artifact hash, never by matching metadata.

use serde::{Deserialize, Serialize};

/// Field length ceilings, enforced defensively since this metadata may
/// originate from an untrusted publisher. These are generous for legitimate
/// values (a repo URL, a container image reference) while still bounding
/// storage/display cost.
const MAX_URL_LEN: usize = 2048;
const MAX_SHORT_FIELD_LEN: usize = 256;
const MAX_IMAGE_REF_LEN: usize = 512;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct SourceInfo {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub commit: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct ToolchainInfo {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rustc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub soroban_sdk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stellar_cli: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct DependencyInfo {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lockfile_sha256: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct BuildEnvironmentInfo {
    /// A pinned container image reference, e.g.
    /// `ghcr.io/example/soroban-builder@sha256:...`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub image: Option<String>,
}

/// Reproducibility is a *result*, not an input: it is only ever set by an
/// actual rebuild attempt (see `contract verify-build` in the CLI), never
/// inferred from the presence of the other provenance fields.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReproducibilityStatus {
    #[default]
    NotChecked,
    /// An independent rebuild produced a WASM hash matching the recorded
    /// artifact hash.
    Reproduced,
    /// An independent rebuild completed but its WASM hash did not match the
    /// recorded artifact hash.
    Mismatched,
    /// An independent rebuild was attempted but failed to produce a WASM
    /// artifact at all (compile error, missing toolchain, timeout).
    BuildFailed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct ReproducibilityInfo {
    #[serde(default)]
    pub status: ReproducibilityStatus,
}

/// Optional, structured build-provenance metadata for a published contract.
/// Every field is optional: contracts published without provenance remain
/// valid, and partial provenance (e.g. source repo but no pinned build
/// environment) is expected to be common.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct BuildProvenance {
    #[serde(default)]
    pub source: SourceInfo,
    #[serde(default)]
    pub toolchain: ToolchainInfo,
    #[serde(default)]
    pub dependencies: DependencyInfo,
    #[serde(default)]
    pub build_environment: BuildEnvironmentInfo,
    #[serde(default)]
    pub reproducibility: ReproducibilityInfo,
}

fn is_valid_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && value.len() <= MAX_URL_LEN
        && !value.chars().any(|c| c.is_control() || c.is_whitespace())
}

fn is_valid_commit_sha(value: &str) -> bool {
    let len = value.len();
    (7..=40).contains(&len) && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Coarse container-image-reference validation: bounded length, no
/// whitespace/control characters, and restricted to the charset image
/// references actually use (`[a-zA-Z0-9./:_@-]`). This is deliberately not a
/// full OCI reference grammar; it exists to reject obviously malformed or
/// abusive input, not to validate registry semantics.
fn is_valid_image_ref(value: &str) -> bool {
    value.len() <= MAX_IMAGE_REF_LEN
        && !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "./:_@-".contains(c))
}

fn is_valid_short_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SHORT_FIELD_LEN
        && !value.chars().any(|c| c.is_control())
}

/// Validate a [`BuildProvenance`] record. Returns a list of human-readable
/// problems; an empty list means the record is well-formed (this says
/// nothing about whether the artifact is actually reproducible).
pub fn validate_provenance(provenance: &BuildProvenance) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(repo) = &provenance.source.repository {
        if !is_valid_url(repo) {
            errors.push(format!(
                "source.repository must be an http(s) URL of at most {MAX_URL_LEN} bytes with no whitespace"
            ));
        }
    }
    if let Some(commit) = &provenance.source.commit {
        if !is_valid_commit_sha(commit) {
            errors.push(
                "source.commit must be a 7-40 character hexadecimal git commit SHA".to_string(),
            );
        }
    }

    for (label, value) in [
        ("toolchain.rustc", &provenance.toolchain.rustc),
        ("toolchain.soroban_sdk", &provenance.toolchain.soroban_sdk),
        ("toolchain.stellar_cli", &provenance.toolchain.stellar_cli),
        ("toolchain.target", &provenance.toolchain.target),
    ] {
        if let Some(v) = value {
            if !is_valid_short_field(v) {
                errors.push(format!(
                    "{label} must be a non-empty string of at most {MAX_SHORT_FIELD_LEN} bytes with no control characters"
                ));
            }
        }
    }

    if let Some(lockfile_sha256) = &provenance.dependencies.lockfile_sha256 {
        if !is_valid_sha256_hex(lockfile_sha256) {
            errors.push(
                "dependencies.lockfile_sha256 must be a 64-character hexadecimal SHA-256 hash"
                    .to_string(),
            );
        }
    }

    if let Some(image) = &provenance.build_environment.image {
        if !is_valid_image_ref(image) {
            errors.push(format!(
                "build_environment.image must be a valid, at most {MAX_IMAGE_REF_LEN}-byte container image reference"
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_provenance_is_valid() {
        assert!(validate_provenance(&BuildProvenance::default()).is_empty());
    }

    #[test]
    fn default_reproducibility_is_not_checked() {
        assert_eq!(
            BuildProvenance::default().reproducibility.status,
            ReproducibilityStatus::NotChecked
        );
    }

    #[test]
    fn valid_full_provenance_passes() {
        let p = BuildProvenance {
            source: SourceInfo {
                repository: Some("https://github.com/example/contract".to_string()),
                commit: Some("abc123def456".to_string()),
            },
            toolchain: ToolchainInfo {
                rustc: Some("1.79.0".to_string()),
                soroban_sdk: Some("21.7.7".to_string()),
                stellar_cli: Some("21.0.0".to_string()),
                target: Some("wasm32v1-none".to_string()),
            },
            dependencies: DependencyInfo {
                lockfile_sha256: Some("a".repeat(64)),
            },
            build_environment: BuildEnvironmentInfo {
                image: Some("ghcr.io/example/soroban-builder@sha256:deadbeef".to_string()),
            },
            reproducibility: ReproducibilityInfo::default(),
        };
        assert!(validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_non_http_repository_url() {
        let p = BuildProvenance {
            source: SourceInfo {
                repository: Some("git@github.com:example/contract.git".to_string()),
                commit: None,
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_oversized_repository_url() {
        let p = BuildProvenance {
            source: SourceInfo {
                repository: Some(format!("https://example.com/{}", "a".repeat(3000))),
                commit: None,
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_non_hex_commit() {
        let p = BuildProvenance {
            source: SourceInfo {
                repository: None,
                commit: Some("not-a-sha!!".to_string()),
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_too_short_commit() {
        let p = BuildProvenance {
            source: SourceInfo {
                repository: None,
                commit: Some("abc12".to_string()),
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_malformed_lockfile_hash() {
        let p = BuildProvenance {
            dependencies: DependencyInfo {
                lockfile_sha256: Some("not-hex".to_string()),
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_image_ref_with_whitespace() {
        let p = BuildProvenance {
            build_environment: BuildEnvironmentInfo {
                image: Some("ghcr.io/example/builder rm -rf /".to_string()),
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn rejects_oversized_toolchain_field() {
        let p = BuildProvenance {
            toolchain: ToolchainInfo {
                rustc: Some("x".repeat(1000)),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!validate_provenance(&p).is_empty());
    }

    #[test]
    fn deserializes_the_documented_example_shape() {
        let json = r#"{
            "source": { "repository": "https://github.com/example/contract", "commit": "abc123" },
            "toolchain": { "rustc": "1.79.0", "soroban_sdk": "21.7.7", "stellar_cli": "21.0.0", "target": "wasm32v1-none" },
            "dependencies": { "lockfile_sha256": null },
            "build_environment": { "image": null },
            "reproducibility": { "status": "not_checked" }
        }"#;
        let p: BuildProvenance = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(p.source.repository.as_deref(), Some("https://github.com/example/contract"));
        assert_eq!(p.reproducibility.status, ReproducibilityStatus::NotChecked);
    }

    #[test]
    fn missing_provenance_fields_deserialize_to_defaults() {
        let p: BuildProvenance = serde_json::from_str("{}").expect("should deserialize");
        assert_eq!(p, BuildProvenance::default());
    }
}
