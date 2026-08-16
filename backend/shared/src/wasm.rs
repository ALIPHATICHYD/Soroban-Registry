//! Structural validation of contract WASM binaries.
//!
//! This is the single source of truth for "is this a valid (Soroban) contract
//! WASM?" — used by the backend simulation/verification path and by the CLI's
//! local `contract verify` command, so both apply identical checks.

use serde::{Deserialize, Serialize};
use wasmparser::Parser;

/// Soroban embeds its contract spec (the ABI) in a custom WASM section with
/// this name. Its presence is what distinguishes a deployable Soroban contract
/// from an arbitrary WASM module.
pub const CONTRACT_SPEC_SECTION: &str = "contractspecv0";
/// Custom section carrying the environment/SDK metadata (interface version).
pub const CONTRACT_ENV_META_SECTION: &str = "contractenvmetav0";

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
    let has_env_meta = custom_sections.iter().any(|s| s == CONTRACT_ENV_META_SECTION);

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

/// Extract payload bytes of a named custom section from a WASM binary.
pub fn extract_custom_section(wasm_bytes: &[u8], target_name: &str) -> Option<Vec<u8>> {
    let parser = Parser::new(0);
    for payload in parser.parse_all(wasm_bytes) {
        if let Ok(wasmparser::Payload::CustomSection(c)) = payload {
            if c.name() == target_name {
                return Some(c.data().to_vec());
            }
        }
    }
    None
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
    fn with_custom_section(mut wasm: Vec<u8>, name: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.push(name.len() as u8); // name length (LEB128, fits in one byte here)
        body.extend_from_slice(name.as_bytes());
        wasm.push(0x00); // custom section id
        wasm.push(body.len() as u8); // section size
        wasm.extend_from_slice(&body);
        wasm
    }

    #[test]
    fn parses_minimal_wasm_without_parse_errors() {
        let result = validate_wasm(MINIMAL_WASM);
        assert!(
            !result.errors.iter().any(|e| e.contains("WASM parsing error")),
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
}
