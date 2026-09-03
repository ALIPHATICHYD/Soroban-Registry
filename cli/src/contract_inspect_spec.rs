//! contract_inspect_spec.rs — Offline contract-spec inspection for local WASM artifacts (#1142)

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;
use stellar_xdr::curr::{Limits, ReadXdr, ScSpecEntry, ScSpecTypeDef};

/// Stable JSON output format for automation and CI consumers.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InspectSpecOutput {
    /// Validation status: "valid" or "invalid"
    pub status: String,
    /// SHA-256 checksum of the WASM file
    pub wasm_sha256: String,
    /// Information about the `contractspecv0` custom section
    pub spec_section: SpecSectionOutput,
    /// Contract interface fingerprint information
    pub interface: InterfaceOutput,
    /// Aggregate counts of contract specification entities
    pub counts: SpecCountsOutput,
    /// Diagnostic items (warnings, errors, missing section, malformed XDR)
    pub diagnostics: Vec<DiagnosticOutput>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecSectionOutput {
    pub name: String,
    pub present: bool,
    pub bytes: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceOutput {
    pub algorithm: String,
    pub interface_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SpecCountsOutput {
    pub functions: usize,
    pub types: usize,
    pub events: usize,
    pub errors: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct DiagnosticOutput {
    pub severity: String, // "error" | "warning" | "info"
    pub message: String,
}

/// Parsed function summary for diagnostic inspection.
#[derive(Debug, Clone)]
struct FunctionSummary {
    name: String,
    signature: String,
    referenced_udts: Vec<String>,
}

/// Parsed UDT summary for diagnostic inspection.
#[derive(Debug, Clone)]
struct UdtSummary {
    name: String,
    kind: &'static str,
    referenced_udts: Vec<String>,
}

/// Main entry point for `soroban-registry contract inspect-spec <WASM_FILE> [--json]`
pub async fn run_inspect_spec(wasm_file: &str, json_output: bool) -> Result<()> {
    let path = Path::new(wasm_file);
    if !path.exists() {
        if json_output {
            let err_json = serde_json::json!({
                "status": "invalid",
                "wasm_sha256": "",
                "spec_section": {
                    "name": "contractspecv0",
                    "present": false,
                    "bytes": 0
                },
                "interface": {
                    "algorithm": "soroban-interface-v1",
                    "interface_id": null
                },
                "counts": {
                    "functions": 0,
                    "types": 0,
                    "events": 0,
                    "errors": 0
                },
                "diagnostics": [{
                    "severity": "error",
                    "message": format!("WASM file not found: {}", wasm_file)
                }]
            });
            println!("{err_json}");
            std::process::exit(1);
        }
        anyhow::bail!("WASM file not found: {}", wasm_file);
    }

    let wasm_bytes = std::fs::read(path)
        .with_context(|| format!("failed to read WASM file {}", wasm_file))?;

    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_sha256 = hex::encode(hasher.finalize());

    let wasm_val = shared::wasm::validate_wasm(&wasm_bytes);
    let spec_bytes = shared::wasm::extract_custom_section(&wasm_bytes, "contractspecv0");

    let mut diagnostics = Vec::new();

    for err in &wasm_val.errors {
        diagnostics.push(DiagnosticOutput {
            severity: "error".to_string(),
            message: format!("WASM structural error: {}", err),
        });
    }
    for warn in &wasm_val.warnings {
        diagnostics.push(DiagnosticOutput {
            severity: "warning".to_string(),
            message: format!("WASM structural warning: {}", warn),
        });
    }

    let mut counts = SpecCountsOutput::default();
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    let mut parsed_entries = Vec::new();
    let mut has_malformed_xdr = false;

    let (present, bytes_len) = match spec_bytes {
        Some(ref bytes) => {
            let mut cursor = Cursor::new(bytes.as_slice());
            let total_len = bytes.len() as u64;

            let mut limited = stellar_xdr::curr::Limited::new(&mut cursor, Limits::none());

            while limited.inner.position() < total_len {
                let start_pos = limited.inner.position();
                match ScSpecEntry::read_xdr(&mut limited) {
                    Ok(entry) => {
                        parsed_entries.push(entry);
                    }
                    Err(e) => {
                        has_malformed_xdr = true;
                        diagnostics.push(DiagnosticOutput {
                            severity: "error".to_string(),
                            message: format!(
                                "Malformed XDR entry in contractspecv0 section at offset {}: {}",
                                start_pos, e
                            ),
                        });
                        break;
                    }
                }
            }

            (true, bytes.len())
        }
        None => {
            diagnostics.push(DiagnosticOutput {
                severity: "error".to_string(),
                message: "Missing 'contractspecv0' custom section in WASM binary".to_string(),
            });
            (false, 0)
        }
    };

    // Analyze parsed XDR spec entries
    let mut defined_udt_names = HashSet::new();
    let mut function_names = HashSet::new();
    let mut event_names = HashSet::new();

    for entry in &parsed_entries {
        #[allow(unreachable_patterns)] // future-proof against new stellar-xdr variants
        match entry {
            ScSpecEntry::FunctionV0(f) => {
                counts.functions += 1;
                let func_name = f.name.to_utf8_string_lossy().to_string();
                if !function_names.insert(func_name.clone()) {
                    diagnostics.push(DiagnosticOutput {
                        severity: "error".to_string(),
                        message: format!("Duplicate function name defined in contract spec: '{}'", func_name),
                    });
                }

                let mut input_parts = Vec::new();
                let mut referenced = Vec::new();

                for inp in f.inputs.iter() {
                    let p_name = inp.name.to_utf8_string_lossy().to_string();
                    let (p_type, p_udts) = format_type_def(&inp.type_);
                    input_parts.push(format!("{}: {}", p_name, p_type));
                    referenced.extend(p_udts);
                }

                let mut out_parts = Vec::new();
                for out in f.outputs.iter() {
                    let (o_type, o_udts) = format_type_def(out);
                    out_parts.push(o_type);
                    referenced.extend(o_udts);
                }

                let return_sig = if out_parts.is_empty() {
                    "()".to_string()
                } else if out_parts.len() == 1 {
                    out_parts[0].clone()
                } else {
                    format!("({})", out_parts.join(", "))
                };

                let sig = format!("fn {}({}) -> {}", func_name, input_parts.join(", "), return_sig);
                functions.push(FunctionSummary {
                    name: func_name,
                    signature: sig,
                    referenced_udts: referenced,
                });
            }
            ScSpecEntry::UdtStructV0(s) => {
                counts.types += 1;
                let struct_name = s.name.to_utf8_string_lossy().to_string();
                if !defined_udt_names.insert(struct_name.clone()) {
                    diagnostics.push(DiagnosticOutput {
                        severity: "error".to_string(),
                        message: format!("Duplicate type name defined in contract spec: '{}'", struct_name),
                    });
                }

                let mut referenced = Vec::new();
                for f in s.fields.iter() {
                    let (_, f_udts) = format_type_def(&f.type_);
                    referenced.extend(f_udts);
                }

                types.push(UdtSummary {
                    name: struct_name,
                    kind: "struct",
                    referenced_udts: referenced,
                });
            }
            ScSpecEntry::UdtUnionV0(u) => {
                counts.types += 1;
                let union_name = u.name.to_utf8_string_lossy().to_string();
                if !defined_udt_names.insert(union_name.clone()) {
                    diagnostics.push(DiagnosticOutput {
                        severity: "error".to_string(),
                        message: format!("Duplicate type name defined in contract spec: '{}'", union_name),
                    });
                }

                let mut referenced = Vec::new();
                for c in u.cases.iter() {
                    if let stellar_xdr::curr::ScSpecUdtUnionCaseV0::TupleV0(t) = c {
                        for type_def in t.type_.iter() {
                            let (_, t_udts) = format_type_def(type_def);
                            referenced.extend(t_udts);
                        }
                    }
                }

                types.push(UdtSummary {
                    name: union_name,
                    kind: "union",
                    referenced_udts: referenced,
                });
            }
            ScSpecEntry::UdtEnumV0(e) => {
                counts.types += 1;
                let enum_name = e.name.to_utf8_string_lossy().to_string();
                if !defined_udt_names.insert(enum_name.clone()) {
                    diagnostics.push(DiagnosticOutput {
                        severity: "error".to_string(),
                        message: format!("Duplicate type name defined in contract spec: '{}'", enum_name),
                    });
                }

                types.push(UdtSummary {
                    name: enum_name,
                    kind: "enum",
                    referenced_udts: Vec::new(),
                });
            }
            ScSpecEntry::UdtErrorEnumV0(err_enum) => {
                counts.errors += err_enum.cases.len();
                let err_name = err_enum.name.to_utf8_string_lossy().to_string();
                defined_udt_names.insert(err_name.clone());

                for c in err_enum.cases.iter() {
                    errors.push(format!("{}::{} = {}", err_name, c.name.to_utf8_string_lossy(), c.value));
                }
            }
            ScSpecEntry::EventV0(ev) => {
                counts.events += 1;
                let ev_name = ev.name.to_utf8_string_lossy().to_string();
                if !event_names.insert(ev_name.clone()) {
                    diagnostics.push(DiagnosticOutput {
                        severity: "warning".to_string(),
                        message: format!("Duplicate event name in contract spec: '{}'", ev_name),
                    });
                }
                events.push(ev_name);
            }
            // Catch-all for future or unknown ScSpecEntry variants.
            // This ensures forward-compatibility: if the stellar-xdr crate
            // adds new variants, they are reported as unsupported rather than
            // causing a compile-time or silent-data-loss issue.
            _ => {
                diagnostics.push(DiagnosticOutput {
                    severity: "warning".to_string(),
                    message: "Unsupported or unknown contract-spec entry version detected".to_string(),
                });
            }
        }
    }

    // Check unresolved type references
    for func in &functions {
        for udt in &func.referenced_udts {
            if !defined_udt_names.contains(udt) {
                diagnostics.push(DiagnosticOutput {
                    severity: "error".to_string(),
                    message: format!(
                        "Unresolved type reference '{}' in function '{}'",
                        udt, func.name
                    ),
                });
            }
        }
    }

    for udt_summary in &types {
        for ref_udt in &udt_summary.referenced_udts {
            if !defined_udt_names.contains(ref_udt) {
                diagnostics.push(DiagnosticOutput {
                    severity: "error".to_string(),
                    message: format!(
                        "Unresolved type reference '{}' in UDT '{}'",
                        ref_udt, udt_summary.name
                    ),
                });
            }
        }
    }

    // Compute interface fingerprint ID if spec is present and parseable
    let has_errors = diagnostics.iter().any(|d| d.severity == "error");

    let interface_id = if present && !has_malformed_xdr && !has_errors {
        Some(compute_interface_fingerprint(&functions, &types, &events, &errors))
    } else {
        None
    };

    let status = if has_errors { "invalid" } else { "valid" };

    let output = InspectSpecOutput {
        status: status.to_string(),
        wasm_sha256: wasm_sha256.clone(),
        spec_section: SpecSectionOutput {
            name: "contractspecv0".to_string(),
            present,
            bytes: bytes_len,
        },
        interface: InterfaceOutput {
            algorithm: "soroban-interface-v1".to_string(),
            interface_id,
        },
        counts,
        diagnostics: diagnostics.clone(),
    };

    if json_output {
        let json_str = serde_json::to_string_pretty(&output)?;
        println!("{json_str}");
    } else {
        render_human_readable(&wasm_file, &output, &functions, &types, &events, &errors);
    }

    if status == "invalid" {
        std::process::exit(1);
    }

    Ok(())
}

/// Compute deterministic SHA-256 interface fingerprint (soroban-interface-v1).
fn compute_interface_fingerprint(
    functions: &[FunctionSummary],
    types: &[UdtSummary],
    events: &[String],
    errors: &[String],
) -> String {
    let mut lines = Vec::new();

    let mut func_sigs: Vec<String> = functions.iter().map(|f| f.signature.clone()).collect();
    func_sigs.sort();
    lines.extend(func_sigs);

    let mut type_sigs: Vec<String> = types.iter().map(|t| format!("{}:{}", t.kind, t.name)).collect();
    type_sigs.sort();
    lines.extend(type_sigs);

    let mut event_sigs = events.to_vec();
    event_sigs.sort();
    lines.extend(event_sigs);

    let mut err_sigs = errors.to_vec();
    err_sigs.sort();
    lines.extend(err_sigs);

    let canonical = lines.join("\n");
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Format ScSpecTypeDef into human-readable type string and list of custom UDT references.
fn format_type_def(def: &ScSpecTypeDef) -> (String, Vec<String>) {
    let mut udts = Vec::new();
    let type_str = match def {
        ScSpecTypeDef::Val => "val".to_string(),
        ScSpecTypeDef::Bool => "bool".to_string(),
        ScSpecTypeDef::Void => "()".to_string(),
        ScSpecTypeDef::Error => "error".to_string(),
        ScSpecTypeDef::U32 => "u32".to_string(),
        ScSpecTypeDef::I32 => "i32".to_string(),
        ScSpecTypeDef::U64 => "u64".to_string(),
        ScSpecTypeDef::I64 => "i64".to_string(),
        ScSpecTypeDef::Timepoint => "timepoint".to_string(),
        ScSpecTypeDef::Duration => "duration".to_string(),
        ScSpecTypeDef::U128 => "u128".to_string(),
        ScSpecTypeDef::I128 => "i128".to_string(),
        ScSpecTypeDef::U256 => "u256".to_string(),
        ScSpecTypeDef::I256 => "i256".to_string(),
        ScSpecTypeDef::Bytes => "bytes".to_string(),
        ScSpecTypeDef::String => "string".to_string(),
        ScSpecTypeDef::Symbol => "symbol".to_string(),
        ScSpecTypeDef::Address => "address".to_string(),
        ScSpecTypeDef::MuxedAddress => "muxed_address".to_string(),
        ScSpecTypeDef::Option(opt) => {
            let (inner_type, inner_udts) = format_type_def(&opt.value_type);
            udts.extend(inner_udts);
            format!("Option<{}>", inner_type)
        }
        ScSpecTypeDef::Vec(vec) => {
            let (inner_type, inner_udts) = format_type_def(&vec.element_type);
            udts.extend(inner_udts);
            format!("Vec<{}>", inner_type)
        }
        ScSpecTypeDef::Map(map) => {
            let (k_type, k_udts) = format_type_def(&map.key_type);
            let (v_type, v_udts) = format_type_def(&map.value_type);
            udts.extend(k_udts);
            udts.extend(v_udts);
            format!("Map<{}, {}>", k_type, v_type)
        }
        ScSpecTypeDef::BytesN(b) => format!("BytesN<{}>", b.n),
        ScSpecTypeDef::Udt(udt) => {
            let name = udt.name.to_utf8_string_lossy().to_string();
            udts.push(name.clone());
            name
        }
        ScSpecTypeDef::Tuple(t) => {
            let mut parts = Vec::new();
            for type_def in t.value_types.iter() {
                let (elem_type, elem_udts) = format_type_def(type_def);
                parts.push(elem_type);
                udts.extend(elem_udts);
            }
            format!("({})", parts.join(", "))
        }
        ScSpecTypeDef::Result(res) => {
            let (ok_type, ok_udts) = format_type_def(&res.ok_type);
            let (err_type, err_udts) = format_type_def(&res.error_type);
            udts.extend(ok_udts);
            udts.extend(err_udts);
            format!("Result<{}, {}>", ok_type, err_type)
        }
    };

    (type_str, udts)
}

/// Render colored human-readable text output for terminal users.
fn render_human_readable(
    wasm_file: &str,
    output: &InspectSpecOutput,
    functions: &[FunctionSummary],
    types: &[UdtSummary],
    events: &[String],
    errors: &[String],
) {
    println!("\n{}", "=== Soroban Contract Specification Inspection ===".bold().cyan());
    println!("File:         {}", wasm_file.bold());
    println!("SHA-256:      {}", output.wasm_sha256);
    println!(
        "Status:       {}",
        if output.status == "valid" {
            "VALID".green().bold()
        } else {
            "INVALID".red().bold()
        }
    );
    println!(
        "Spec Section: {} ({} bytes)",
        if output.spec_section.present {
            "contractspecv0 (present)".green()
        } else {
            "contractspecv0 (missing)".red()
        },
        output.spec_section.bytes
    );

    if let Some(ref fp) = output.interface.interface_id {
        println!("Interface ID: {} ({})", fp.bright_blue(), output.interface.algorithm);
    }

    println!("\n{}", "Summary Counts:".bold());
    println!("  Functions: {}", output.counts.functions);
    println!("  Types:     {}", output.counts.types);
    println!("  Events:    {}", output.counts.events);
    println!("  Errors:    {}", output.counts.errors);

    if !functions.is_empty() {
        println!("\n{}", "Functions:".bold().yellow());
        for f in functions {
            println!("  - {}", f.signature);
        }
    }

    if !types.is_empty() {
        println!("\n{}", "User-Defined Types:".bold().yellow());
        for t in types {
            println!("  - {} ({})", t.name, t.kind);
        }
    }

    if !events.is_empty() {
        println!("\n{}", "Events:".bold().yellow());
        for e in events {
            println!("  - {}", e);
        }
    }

    if !errors.is_empty() {
        println!("\n{}", "Errors:".bold().yellow());
        for err in errors {
            println!("  - {}", err);
        }
    }

    if !output.diagnostics.is_empty() {
        println!("\n{}", "Diagnostics:".bold());
        for d in &output.diagnostics {
            if d.severity == "error" {
                println!("  {} {}", "[ERROR]".red().bold(), d.message);
            } else if d.severity == "warning" {
                println!("  {} {}", "[WARN]".yellow().bold(), d.message);
            } else {
                println!("  {} {}", "[INFO]".blue().bold(), d.message);
            }
        }
    }

    println!();
}
