use serde::{Deserialize, Serialize};
use wasmparser::Parser;

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
}

pub fn validate_wasm(wasm_bytes: &[u8]) -> WasmValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut function_count = 0u32;
    let mut table_count = 0u32;
    let mut data_section_size = 0u32;
    let mut memory_pages = 0u64;
    let mut export_functions = Vec::new();
    let mut import_functions = Vec::new();

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

    let valid = errors.is_empty();

    if function_count == 0 {
        errors.push("No functions found in WASM binary".to_string());
    }

    if export_functions.is_empty() {
        warnings.push("No exported functions found".to_string());
    }

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_wasm_empty_type_section() {
        // WASM magic number: \0asm (0x00 0x61 0x73 0x6d)
        // WASM version: 1 (0x01 0x00 0x00 0x00)
        // Section 1 (Type Section): ID = 1
        // Size of section: 1 (0x01)
        // Content: 0 types (0x00)
        let wasm_bytes = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // Header
            0x01, // Section ID (Type)
            0x01, // Section Size (1 byte)
            0x00, // Number of types (0)
        ];

        let result = validate_wasm(&wasm_bytes);

        // It shouldn't have WASM parsing errors
        let has_parse_error = result
            .errors
            .iter()
            .any(|e| e.contains("WASM parsing error"));
        assert!(
            !has_parse_error,
            "Should parse successfully, but got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_wasm_zero_length_type_section() {
        // WASM magic number: \0asm (0x00 0x61 0x73 0x6d)
        // WASM version: 1 (0x01 0x00 0x00 0x00)
        // Section 1 (Type Section): ID = 1
        // Size of section: 0 (0x00)
        let wasm_bytes = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // Header
            0x01, // Section ID (Type)
            0x00, // Section Size (0 bytes)
        ];

        let result = validate_wasm(&wasm_bytes);

        // This is invalid WASM because the type section needs a count, so we expect a parsing error
        let has_parse_error = result
            .errors
            .iter()
            .any(|e| e.contains("WASM parsing error"));
        assert!(
            has_parse_error,
            "Should fail with a parsing error, but got: {:?}",
            result.errors
        );
    }
}
