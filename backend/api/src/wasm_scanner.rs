//! Lightweight, deterministic validation for uploaded contract WASM.
//! This is deliberately fail-closed: an artifact that cannot be parsed, does
//! not match its advertised hash, or contains an executable-payload marker is
//! quarantined until a stronger scanner can clear it.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmScanResult {
    pub status: &'static str,
    pub findings: Vec<String>,
}

pub fn scan(bytes: &[u8], expected_hash: &str) -> WasmScanResult {
    let mut findings = Vec::new();
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" || &bytes[4..8] != [1, 0, 0, 0] {
        findings.push("malformed_wasm_header".to_string());
    } else if let Err(error) = wasmparser::Validator::new().validate_all(bytes) {
        findings.push(format!("malformed_wasm: {}", error));
    }

    let mut digest = Sha256::new();
    digest.update(bytes);
    let actual_hash = hex::encode(digest.finalize());
    if !expected_hash.trim().eq_ignore_ascii_case(&actual_hash) {
        findings.push("wasm_hash_mismatch".to_string());
    }

    let lower = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    for marker in ["malware", "virus", "shellcode", "powershell", "execve"] {
        if lower.contains(marker) {
            findings.push(format!("suspicious_payload_marker:{marker}"));
        }
    }

    WasmScanResult {
        status: if findings.is_empty() {
            "passed"
        } else {
            "quarantined"
        },
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(bytes: &[u8]) -> String {
        let mut digest = Sha256::new();
        digest.update(bytes);
        hex::encode(digest.finalize())
    }

    #[test]
    fn malformed_wasm_is_quarantined() {
        let result = scan(b"not wasm", &hash(b"not wasm"));
        assert_eq!(result.status, "quarantined");
        assert!(result.findings.iter().any(|f| f == "malformed_wasm_header"));
    }

    #[test]
    fn known_bad_payload_is_quarantined() {
        let mut wasm = b"\0asm\x01\0\0\0".to_vec();
        wasm.extend_from_slice(b"malware");
        let result = scan(&wasm, &hash(&wasm));
        assert_eq!(result.status, "quarantined");
        assert!(result.findings.iter().any(|f| f.contains("malware")));
    }

    #[test]
    fn artifact_hash_mismatch_is_quarantined() {
        let wasm = b"\0asm\x01\0\0\0";
        let result = scan(wasm, &hash(b"a different artifact"));
        assert_eq!(result.status, "quarantined");
        assert!(result.findings.iter().any(|f| f == "wasm_hash_mismatch"));
    }
}
