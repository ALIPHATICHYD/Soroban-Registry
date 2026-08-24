//! Structural validation of contract WASM binaries.
//!
//! This is the single source of truth for "is this a valid (Soroban) contract
//! WASM?" — used by the backend simulation/verification path and by the CLI's
//! local `contract verify` command, so both apply identical checks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasmparser::{Parser, Validator};

/// Soroban embeds its contract spec (the ABI) in a custom WASM section with
/// this name. Its presence is what distinguishes a deployable Soroban contract
/// from an arbitrary WASM module.
pub const CONTRACT_SPEC_SECTION: &str = "contractspecv0";
/// Custom section carrying the environment/SDK metadata (interface version).
pub const CONTRACT_ENV_META_SECTION: &str = "contractenvmetav0";

/// Identifier for the first canonical contract-artifact hashing scheme.
///
/// V1 removes only the standard debug/toolchain custom sections `name` and
/// `producers`. All executable sections, Soroban metadata, and unknown custom
/// sections remain byte-for-byte unchanged.
pub const CANONICAL_WASM_HASH_V1: &str = "soroban-registry-wasm-canonical-v1";

const CANONICAL_V1_IGNORED_CUSTOM_SECTIONS: [&str; 2] = ["name", "producers"];

/// Error returned when an artifact cannot be safely canonicalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalWasmError {
    InvalidWasm(String),
    InvalidSectionEncoding,
    InvalidCustomSectionName,
}

impl std::fmt::Display for CanonicalWasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWasm(detail) => write!(f, "invalid wasm: {detail}"),
            Self::InvalidSectionEncoding => write!(f, "invalid wasm section encoding"),
            Self::InvalidCustomSectionName => write!(f, "invalid custom section name"),
        }
    }
}

impl std::error::Error for CanonicalWasmError {}

/// Produce the V1 canonical representation used only as a secondary,
/// metadata-tolerant comparison.
///
/// The original artifact hash remains the authority for identifying deployed
/// code. This function deliberately does not modify code, data, imports,
/// exports, `contractspecv0`, `contractenvmetav0`, or unknown custom sections.
pub fn canonicalize_wasm_v1(wasm_bytes: &[u8]) -> Result<Vec<u8>, CanonicalWasmError> {
    Validator::new()
        .validate_all(wasm_bytes)
        .map_err(|err| CanonicalWasmError::InvalidWasm(err.to_string()))?;

    const WASM_V1_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";
    if !wasm_bytes.starts_with(WASM_V1_HEADER) {
        return Err(CanonicalWasmError::InvalidWasm(
            "expected a core WebAssembly v1 module".to_string(),
        ));
    }

    let mut canonical = Vec::with_capacity(wasm_bytes.len());
    canonical.extend_from_slice(WASM_V1_HEADER);
    let mut cursor = WASM_V1_HEADER.len();

    while cursor < wasm_bytes.len() {
        let section_start = cursor;
        let section_id = *wasm_bytes
            .get(cursor)
            .ok_or(CanonicalWasmError::InvalidSectionEncoding)?;
        cursor += 1;

        let section_len = read_u32_leb(wasm_bytes, &mut cursor)? as usize;
        let payload_start = cursor;
        let section_end = payload_start
            .checked_add(section_len)
            .filter(|end| *end <= wasm_bytes.len())
            .ok_or(CanonicalWasmError::InvalidSectionEncoding)?;

        let ignored = if section_id == 0 {
            let mut name_cursor = payload_start;
            let name_len = read_u32_leb(wasm_bytes, &mut name_cursor)? as usize;
            let name_end = name_cursor
                .checked_add(name_len)
                .filter(|end| *end <= section_end)
                .ok_or(CanonicalWasmError::InvalidSectionEncoding)?;
            let name = std::str::from_utf8(&wasm_bytes[name_cursor..name_end])
                .map_err(|_| CanonicalWasmError::InvalidCustomSectionName)?;
            CANONICAL_V1_IGNORED_CUSTOM_SECTIONS.contains(&name)
        } else {
            false
        };

        if !ignored {
            canonical.extend_from_slice(&wasm_bytes[section_start..section_end]);
        }
        cursor = section_end;
    }

    Ok(canonical)
}

/// SHA-256 of [`canonicalize_wasm_v1`]. The algorithm identifier must be
/// stored or transmitted alongside this value so future schemes cannot be
/// confused with V1.
pub fn canonical_wasm_hash_v1(wasm_bytes: &[u8]) -> Result<String, CanonicalWasmError> {
    let canonical = canonicalize_wasm_v1(wasm_bytes)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    Ok(hex::encode(hasher.finalize()))
}

fn read_u32_leb(bytes: &[u8], cursor: &mut usize) -> Result<u32, CanonicalWasmError> {
    let mut value = 0u32;
    for shift in (0..=28).step_by(7) {
        let byte = *bytes
            .get(*cursor)
            .ok_or(CanonicalWasmError::InvalidSectionEncoding)?;
        *cursor += 1;

        if shift == 28 && byte & 0xf0 != 0 {
            return Err(CanonicalWasmError::InvalidSectionEncoding);
        }
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(CanonicalWasmError::InvalidSectionEncoding)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub function_count: u32,
    pub table_count: u32,
    pub data_section_size: u32,
    pub memory_pages: u64,
    pub export_functions: Vec<String>,
    pub import_functions: Vec<String>,
    /// Names of all custom sections found in the module.
    pub custom_sections: Vec<String>,
    /// Whether the Soroban contract spec (ABI) section is present.
    pub has_contract_spec: bool,
    /// Whether the Soroban environment metadata section is present.
    pub has_env_meta: bool,
}

impl WasmValidationResult {
    /// True when the module is structurally valid WASM *and* carries the Soroban
    /// contract spec — i.e. it is a publishable Soroban contract.
    pub fn is_soroban_contract(&self) -> bool {
        self.valid && self.has_contract_spec
    }
}

/// Extract the raw bytes of the `contractspecv0` custom section, if present.
/// Concatenates the payloads of every section with that name, matching how
/// Soroban's toolchain may split them, before use with
/// [`crate::contract_spec::parse_contract_spec`].
pub fn extract_contract_spec_bytes(wasm_bytes: &[u8]) -> Option<Vec<u8>> {
    let parser = Parser::new(0);
    let mut out: Option<Vec<u8>> = None;

    for payload in parser.parse_all(wasm_bytes) {
        if let Ok(wasmparser::Payload::CustomSection(c)) = payload {
            if c.name() == CONTRACT_SPEC_SECTION {
                out.get_or_insert_with(Vec::new)
                    .extend_from_slice(c.data());
            }
        }
    }

    out
}

/// Parse and structurally validate a WASM byte slice.
///
/// Returns a populated [`WasmValidationResult`] even on failure; inspect
/// `valid`, `errors`, and `has_contract_spec` to decide how to report.
pub fn validate_wasm(wasm_bytes: &[u8]) -> WasmValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut function_count = 0u32;
    let mut table_count = 0u32;
    let mut data_section_size = 0u32;
    let mut memory_pages = 0u64;
    let mut export_functions = Vec::new();
    let mut import_functions = Vec::new();
    let mut custom_sections = Vec::new();

    let parser = Parser::new(0);

    for payload in parser.parse_all(wasm_bytes) {
        match payload {
            Ok(wasmparser::Payload::Version { num, .. }) => {
                if num != 1 {
                    warnings.push(format!("Unusual WASM version: {}", num));
                }
            }
            Ok(wasmparser::Payload::FunctionSection(f)) => {
                function_count = f.count();
            }
            Ok(wasmparser::Payload::TableSection(t)) => {
                table_count = t.count();
            }
            Ok(wasmparser::Payload::MemorySection(m)) => {
                for mem in m.into_iter().flatten() {
                    memory_pages = mem.initial;
                }
            }
            Ok(wasmparser::Payload::DataSection(d)) => {
                data_section_size = d.count();
            }
            Ok(wasmparser::Payload::ExportSection(e)) => {
                for exp in e.into_iter().flatten() {
                    export_functions.push(exp.name.to_string());
                }
            }
            Ok(wasmparser::Payload::ImportSection(i)) => {
                for imp in i.into_iter().flatten() {
                    let name = format!("{}::{}", imp.module, imp.name);
                    import_functions.push(name);
                }
            }
            Ok(wasmparser::Payload::CustomSection(c)) => {
                custom_sections.push(c.name().to_string());
            }
            Ok(wasmparser::Payload::CodeSectionStart { count, .. }) => {
                if count == 0 {
                    warnings.push("No code section found - contract may be empty".to_string());
                }
            }
            Err(e) => {
                errors.push(format!("WASM parsing error: {}", e));
            }
            _ => {}
        }
    }

    if function_count == 0 {
        errors.push("No functions found in WASM binary".to_string());
    }

    if export_functions.is_empty() {
        warnings.push("No exported functions found".to_string());
    }

    let has_contract_spec = custom_sections.iter().any(|s| s == CONTRACT_SPEC_SECTION);
    let has_env_meta = custom_sections
        .iter()
        .any(|s| s == CONTRACT_ENV_META_SECTION);

    // Valid = structurally parseable AND has at least one function.
    let valid = errors.is_empty();

    WasmValidationResult {
        valid,
        errors,
        warnings,
        function_count,
        table_count,
        data_section_size,
        memory_pages,
        export_functions,
        import_functions,
        custom_sections,
        has_contract_spec,
        has_env_meta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal *valid* contract WASM: header + type + function + export + code
    /// sections, exporting a single no-op function `f`. `validate_wasm` treats a
    /// module with zero functions as invalid, so the fixture needs a real one.
    const MINIMAL_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header (\0asm v1)
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type section: one () -> ()
        0x03, 0x02, 0x01, 0x00, // function section: one func of type 0
        0x07, 0x05, 0x01, 0x01, b'f', 0x00, 0x00, // export "f" as func 0
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // code section: one empty body
    ];

    /// Append a custom section with `name` and empty payload to `wasm`.
    fn with_custom_section(wasm: Vec<u8>, name: &str) -> Vec<u8> {
        with_custom_section_payload(wasm, name, &[])
    }

    /// Append a custom section with `name` and the given payload to `wasm`.
    fn with_custom_section_payload(mut wasm: Vec<u8>, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.push(name.len() as u8); // name length (LEB128, fits in one byte here)
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(payload);
        wasm.push(0x00); // custom section id
        wasm.push(body.len() as u8); // section size (fits in one byte for test fixtures)
        wasm.extend_from_slice(&body);
        wasm
    }

    #[test]
    fn parses_minimal_wasm_without_parse_errors() {
        let result = validate_wasm(MINIMAL_WASM);
        assert!(
            !result
                .errors
                .iter()
                .any(|e| e.contains("WASM parsing error")),
            "unexpected parse errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn rejects_non_wasm_bytes() {
        let result = validate_wasm(b"not a wasm file at all");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn rejects_empty_input() {
        let result = validate_wasm(&[]);
        assert!(!result.valid);
    }

    #[test]
    fn detects_contract_spec_section() {
        let wasm = with_custom_section(MINIMAL_WASM.to_vec(), CONTRACT_SPEC_SECTION);
        let result = validate_wasm(&wasm);
        assert!(
            result.has_contract_spec,
            "should detect {CONTRACT_SPEC_SECTION}: sections={:?}",
            result.custom_sections
        );
        assert!(result.is_soroban_contract());
    }

    #[test]
    fn plain_wasm_is_not_a_soroban_contract() {
        let result = validate_wasm(MINIMAL_WASM);
        assert!(!result.has_contract_spec);
        assert!(!result.is_soroban_contract());
    }

    #[test]
    fn detects_env_meta_section() {
        let wasm = with_custom_section(MINIMAL_WASM.to_vec(), CONTRACT_ENV_META_SECTION);
        let result = validate_wasm(&wasm);
        assert!(result.has_env_meta);
    }

    #[test]
    fn extracts_contract_spec_payload_bytes() {
        let payload = [1u8, 2, 3, 4];
        let wasm =
            with_custom_section_payload(MINIMAL_WASM.to_vec(), CONTRACT_SPEC_SECTION, &payload);
        let extracted = extract_contract_spec_bytes(&wasm).expect("section should be found");
        assert_eq!(extracted, payload);
    }

    #[test]
    fn extract_returns_none_when_section_absent() {
        assert!(extract_contract_spec_bytes(MINIMAL_WASM).is_none());
    }

    #[test]
    fn canonical_hash_ignores_only_allowlisted_toolchain_metadata() {
        let first_build =
            with_custom_section_payload(MINIMAL_WASM.to_vec(), "producers", b"rustc-build-one");
        let second_build =
            with_custom_section_payload(MINIMAL_WASM.to_vec(), "producers", b"rustc-build-two");

        assert_ne!(first_build, second_build);
        assert_eq!(
            canonical_wasm_hash_v1(&first_build).unwrap(),
            canonical_wasm_hash_v1(&second_build).unwrap()
        );
        assert_eq!(canonicalize_wasm_v1(&first_build).unwrap(), MINIMAL_WASM);

        let named_build = with_custom_section(MINIMAL_WASM.to_vec(), "name");
        assert_eq!(canonicalize_wasm_v1(&named_build).unwrap(), MINIMAL_WASM);
    }

    #[test]
    fn canonical_hash_preserves_soroban_and_unknown_custom_sections() {
        for section_name in [
            CONTRACT_SPEC_SECTION,
            CONTRACT_ENV_META_SECTION,
            "vendor-security-metadata",
        ] {
            let first =
                with_custom_section_payload(MINIMAL_WASM.to_vec(), section_name, b"payload-one");
            let second =
                with_custom_section_payload(MINIMAL_WASM.to_vec(), section_name, b"payload-two");

            assert_ne!(
                canonical_wasm_hash_v1(&first).unwrap(),
                canonical_wasm_hash_v1(&second).unwrap(),
                "{section_name} must remain inside the trust boundary"
            );
        }
    }

    #[test]
    fn canonical_hash_preserves_executable_code() {
        let mut changed_instruction = MINIMAL_WASM.to_vec();
        // Change the exported function body from `end` to `nop; end` while
        // keeping the module valid and fixing both section/body lengths.
        let code_section = changed_instruction
            .iter()
            .position(|byte| *byte == 0x0a)
            .expect("fixture has a code section");
        changed_instruction[code_section + 1] = 0x05;
        changed_instruction[code_section + 3] = 0x03;
        changed_instruction.insert(code_section + 5, 0x01);

        Validator::new()
            .validate_all(&changed_instruction)
            .expect("mutated fixture should remain valid");
        assert_ne!(
            canonical_wasm_hash_v1(MINIMAL_WASM).unwrap(),
            canonical_wasm_hash_v1(&changed_instruction).unwrap()
        );
    }

    #[test]
    fn canonicalization_fails_closed_for_malformed_wasm() {
        let err = canonicalize_wasm_v1(b"not wasm").unwrap_err();
        assert!(matches!(err, CanonicalWasmError::InvalidWasm(_)));
    }
}
