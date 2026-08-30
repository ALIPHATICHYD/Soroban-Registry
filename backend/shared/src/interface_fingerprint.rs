//! Deterministic interface fingerprints for Soroban contracts.
//!
//! Derives stable, versioned identifiers from a parsed `contractspecv0`
//! (see [`crate::contract_spec`]) by normalizing away non-semantic data
//! (doc comments, library names, and any incidental entry ordering) before
//! hashing. Two specs that differ only in those respects must produce
//! identical fingerprints; any change to a callable ABI, type, event, or
//! error surface must change the relevant fingerprint.
//!
//! Algorithm identifier: `soroban-interface-v1`. Bump the version string
//! (and add a new module, e.g. `v2`) rather than silently changing this
//! logic in place — old fingerprints must never be reinterpreted under a
//! new algorithm.

use crate::contract_spec::{
    ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0, ScSpecTypeDef,
    ScSpecUdtUnionCaseV0, SpecParseError,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

pub const ALGORITHM: &str = "soroban-interface-v1";

/// The kind of entry a fingerprint was derived from, used to group and sort
/// entries deterministically. Distinct kinds are also mixed into the entry
/// fingerprint so that a struct and a function with the same name never
/// collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Function,
    Type,
    Event,
    Error,
}

/// A single named, fingerprinted entry (function, type, event, or error).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryFingerprint {
    pub kind: EntryKind,
    pub name: String,
    /// sha256 of the canonical signature, hex-encoded.
    pub fingerprint: String,
    /// Canonicalized human-readable signature (also the hash preimage).
    pub signature: String,
}

/// The full interface fingerprint for a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceFingerprint {
    pub algorithm: String,
    pub interface_id: String,
    pub functions: Vec<EntryFingerprint>,
    pub types: Vec<EntryFingerprint>,
    pub events: Vec<EntryFingerprint>,
    pub errors: Vec<EntryFingerprint>,
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Canonical (semantic-only) rendering of a type reference. Never includes
/// doc strings; only names and structural shape.
fn canonical_type(t: &ScSpecTypeDef) -> String {
    t.to_string()
}

fn canonical_union_case(case: &ScSpecUdtUnionCaseV0) -> String {
    match case {
        ScSpecUdtUnionCaseV0::Void { name, .. } => format!("{name}()"),
        ScSpecUdtUnionCaseV0::Tuple { name, types, .. } => {
            let types: Vec<String> = types.iter().map(canonical_type).collect();
            format!("{name}({})", types.join(","))
        }
    }
}

/// Build the human-readable signature, and the structured (collision-safe)
/// hash input, for a single spec entry.
///
/// The hash input is a `serde_json::Value` rather than a hand-joined
/// string: joining fields with plain delimiters (e.g. `,`/`:`) is not safe
/// because a name or UDT type reference could itself contain those
/// characters, letting two structurally different entries serialize to the
/// same joined string. JSON's own escaping and structural nesting rules out
/// that collision regardless of what the individual strings contain.
fn fingerprint_entry(entry: &ScSpecEntry) -> (EntryKind, String, String, serde_json::Value) {
    match entry {
        ScSpecEntry::FunctionV0(f) => {
            let inputs: Vec<String> = f
                .inputs
                .iter()
                .map(|i| format!("{}:{}", i.name, canonical_type(&i.type_def)))
                .collect();
            let outputs: Vec<String> = f.outputs.iter().map(canonical_type).collect();
            let sig = format!(
                "fn {}({}) -> ({})",
                f.name,
                inputs.join(","),
                outputs.join(",")
            );
            let hash_input = json!({
                "fn": f.name,
                "in": f.inputs.iter().map(|i| json!([i.name, canonical_type(&i.type_def)])).collect::<Vec<_>>(),
                "out": outputs,
            });
            (EntryKind::Function, f.name.clone(), sig, hash_input)
        }
        ScSpecEntry::UdtStructV0(s) => {
            let fields: Vec<String> = s
                .fields
                .iter()
                .map(|field| format!("{}:{}", field.name, canonical_type(&field.type_def)))
                .collect();
            let sig = format!("struct {} {{{}}}", s.name, fields.join(","));
            let hash_input = json!({
                "struct": s.name,
                "fields": s.fields.iter().map(|field| json!([field.name, canonical_type(&field.type_def)])).collect::<Vec<_>>(),
            });
            (EntryKind::Type, s.name.clone(), sig, hash_input)
        }
        ScSpecEntry::UdtUnionV0(u) => {
            let cases: Vec<String> = u.cases.iter().map(canonical_union_case).collect();
            let sig = format!("union {} {{{}}}", u.name, cases.join(","));
            // Union case order is part of the wire ABI (it is the
            // discriminant), so it is preserved rather than sorted.
            let hash_input = json!({
                "union": u.name,
                "cases": u.cases.iter().map(|case| match case {
                    ScSpecUdtUnionCaseV0::Void { name, .. } => json!(["void", name, []]),
                    ScSpecUdtUnionCaseV0::Tuple { name, types, .. } => {
                        let types: Vec<String> = types.iter().map(canonical_type).collect();
                        json!(["tuple", name, types])
                    }
                }).collect::<Vec<_>>(),
            });
            (EntryKind::Type, u.name.clone(), sig, hash_input)
        }
        ScSpecEntry::UdtEnumV0(e) => {
            let mut cases: Vec<(u32, &str)> =
                e.cases.iter().map(|c| (c.value, c.name.as_str())).collect();
            cases.sort_by_key(|(v, _)| *v);
            let sig_cases: Vec<String> = cases
                .iter()
                .map(|(v, name)| format!("{name}={v}"))
                .collect();
            let sig = format!("enum {} {{{}}}", e.name, sig_cases.join(","));
            let hash_input = json!({
                "enum": e.name,
                "cases": cases.iter().map(|(v, name)| json!([name, v])).collect::<Vec<_>>(),
            });
            (EntryKind::Type, e.name.clone(), sig, hash_input)
        }
        ScSpecEntry::UdtErrorEnumV0(e) => {
            let mut cases: Vec<(u32, &str)> =
                e.cases.iter().map(|c| (c.value, c.name.as_str())).collect();
            cases.sort_by_key(|(v, _)| *v);
            let sig_cases: Vec<String> = cases
                .iter()
                .map(|(v, name)| format!("{name}={v}"))
                .collect();
            let sig = format!("error {} {{{}}}", e.name, sig_cases.join(","));
            let hash_input = json!({
                "error": e.name,
                "cases": cases.iter().map(|(v, name)| json!([name, v])).collect::<Vec<_>>(),
            });
            (EntryKind::Error, e.name.clone(), sig, hash_input)
        }
        ScSpecEntry::EventV0(ev) => {
            let params: Vec<String> = ev
                .params
                .iter()
                .map(|p| {
                    let loc = match p.location {
                        ScSpecEventParamLocationV0::Data => "data",
                        ScSpecEventParamLocationV0::TopicList => "topic",
                    };
                    format!("{}:{}@{}", p.name, canonical_type(&p.type_def), loc)
                })
                .collect();
            let format_tag = match ev.data_format {
                ScSpecEventDataFormat::SingleValue => "single",
                ScSpecEventDataFormat::Vec => "vec",
                ScSpecEventDataFormat::Map => "map",
            };
            let topics = ev.prefix_topics.join(",");
            let sig = format!(
                "event {} topics=[{}] params=({}) format={}",
                ev.name,
                topics,
                params.join(","),
                format_tag
            );
            let hash_input = json!({
                "event": ev.name,
                "topics": ev.prefix_topics,
                "params": ev.params.iter().map(|p| {
                    let loc = match p.location {
                        ScSpecEventParamLocationV0::Data => "data",
                        ScSpecEventParamLocationV0::TopicList => "topic",
                    };
                    json!([p.name, canonical_type(&p.type_def), loc])
                }).collect::<Vec<_>>(),
                "format": format_tag,
            });
            (EntryKind::Event, ev.name.clone(), sig, hash_input)
        }
    }
}

/// Derive the full [`InterfaceFingerprint`] from a parsed contract spec.
///
/// Entries are grouped by kind and sorted by name so that the original
/// physical ordering of entries inside the WASM section never affects the
/// result. Union case order is preserved within a union's own signature
/// because it is part of that union's wire ABI.
pub fn fingerprint_spec(entries: &[ScSpecEntry]) -> InterfaceFingerprint {
    let mut functions = Vec::new();
    let mut types = Vec::new();
    let mut events = Vec::new();
    let mut errors = Vec::new();

    for entry in entries {
        let (kind, name, signature, hash_input) = fingerprint_entry(entry);
        let hash_input_str =
            serde_json::to_string(&hash_input).expect("json::Value serialization cannot fail");
        let fingerprint = sha256_hex(&format!("{ALGORITHM}:{kind:?}:{hash_input_str}"));
        let item = EntryFingerprint {
            kind,
            name,
            fingerprint,
            signature,
        };
        match kind {
            EntryKind::Function => functions.push(item),
            EntryKind::Type => types.push(item),
            EntryKind::Event => events.push(item),
            EntryKind::Error => errors.push(item),
        }
    }

    for group in [&mut functions, &mut types, &mut events, &mut errors] {
        group.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let mut canonical_all: Vec<&EntryFingerprint> = functions
        .iter()
        .chain(types.iter())
        .chain(events.iter())
        .chain(errors.iter())
        .collect();
    canonical_all.sort_by(|a, b| (a.kind, &a.name).cmp(&(b.kind, &b.name)));

    let joined = canonical_all
        .iter()
        .map(|e| e.fingerprint.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let interface_id = sha256_hex(&format!("{ALGORITHM}:contract:{joined}"));

    InterfaceFingerprint {
        algorithm: ALGORITHM.to_string(),
        interface_id,
        functions,
        types,
        events,
        errors,
    }
}

/// Why a WASM artifact yielded no interface fingerprint.
///
/// None of these are errors in the "the caller did something wrong" sense: a
/// contract may legitimately ship without a spec section. Callers persisting an
/// interface id should record "unknown", not reject the artifact.
#[derive(Debug, Clone, PartialEq)]
pub enum FingerprintWasmError {
    /// The module embeds no [`crate::wasm::CONTRACT_SPEC_SECTION`] custom section.
    NoSpecSection,
    /// The section is present but is not a well-formed spec.
    MalformedSpec(SpecParseError),
    /// The section parsed but declares nothing. Fingerprinting an empty spec
    /// would give every such contract the same id and make "the interfaces
    /// match" a meaningless statement, so it is reported instead.
    EmptySpec,
}

impl std::fmt::Display for FingerprintWasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSpecSection => write!(
                f,
                "no {} section: this module embeds no Soroban contract spec",
                crate::wasm::CONTRACT_SPEC_SECTION
            ),
            Self::MalformedSpec(err) => write!(
                f,
                "malformed {} section: {err}",
                crate::wasm::CONTRACT_SPEC_SECTION
            ),
            Self::EmptySpec => write!(
                f,
                "empty {} section: no interface to identify",
                crate::wasm::CONTRACT_SPEC_SECTION
            ),
        }
    }
}

impl std::error::Error for FingerprintWasmError {}

/// Derive an [`InterfaceFingerprint`] straight from WASM bytes.
///
/// Composes [`crate::wasm::extract_contract_spec_bytes`],
/// [`crate::contract_spec::parse_contract_spec`], and [`fingerprint_spec`].
/// Every step is pure and offline — **no WASM is executed**; the module is only
/// walked for custom sections, and the section is read by a dependency-free XDR
/// reader. That matters because artifacts are untrusted input.
pub fn fingerprint_wasm(wasm_bytes: &[u8]) -> Result<InterfaceFingerprint, FingerprintWasmError> {
    let spec_bytes = crate::wasm::extract_contract_spec_bytes(wasm_bytes)
        .ok_or(FingerprintWasmError::NoSpecSection)?;

    let entries = crate::contract_spec::parse_contract_spec(&spec_bytes)
        .map_err(FingerprintWasmError::MalformedSpec)?;

    if entries.is_empty() {
        return Err(FingerprintWasmError::EmptySpec);
    }

    Ok(fingerprint_spec(&entries))
}

#[cfg(test)]
mod wasm_tests {
    use super::*;

    /// Minimal valid WASM module header, no custom sections.
    const EMPTY_MODULE: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    #[test]
    fn module_without_spec_section_has_no_fingerprint() {
        assert!(matches!(
            fingerprint_wasm(EMPTY_MODULE),
            Err(FingerprintWasmError::NoSpecSection)
        ));
    }

    #[test]
    fn garbage_bytes_never_panic() {
        for bytes in [b"".as_slice(), b"not wasm".as_slice(), &[0xff; 64]] {
            assert!(fingerprint_wasm(bytes).is_err());
        }
    }

    #[test]
    fn spec_section_is_fingerprinted_with_the_published_algorithm() {
        let wasm = wasm_with_spec_section(&function_entry_xdr("transfer"));
        let fp = fingerprint_wasm(&wasm).expect("spec section should fingerprint");
        assert_eq!(fp.algorithm, ALGORITHM);
        assert_eq!(fp.interface_id.len(), 64, "expected hex sha256");
        assert_eq!(fp.functions.len(), 1);
        assert_eq!(fp.functions[0].name, "transfer");
    }

    #[test]
    fn different_interfaces_get_different_ids() {
        let a = fingerprint_wasm(&wasm_with_spec_section(&function_entry_xdr("transfer"))).unwrap();
        let b = fingerprint_wasm(&wasm_with_spec_section(&function_entry_xdr("burn"))).unwrap();
        assert_ne!(a.interface_id, b.interface_id);
    }

    #[test]
    fn identical_interfaces_get_identical_ids() {
        let a = fingerprint_wasm(&wasm_with_spec_section(&function_entry_xdr("transfer"))).unwrap();
        let b = fingerprint_wasm(&wasm_with_spec_section(&function_entry_xdr("transfer"))).unwrap();
        assert_eq!(a.interface_id, b.interface_id);
    }

    #[test]
    fn toolchain_metadata_drift_does_not_change_interface_id() {
        let base = wasm_with_spec_section(&function_entry_xdr("transfer"));
        let first = append_custom_section(base.clone(), "producers", b"build-one");
        let second = append_custom_section(base, "producers", b"build-two");

        assert_eq!(
            fingerprint_wasm(&first).unwrap().interface_id,
            fingerprint_wasm(&second).unwrap().interface_id
        );
    }

    #[test]
    fn empty_spec_section_is_not_an_interface() {
        let wasm = wasm_with_spec_section(&[]);
        assert!(matches!(
            fingerprint_wasm(&wasm),
            Err(FingerprintWasmError::EmptySpec)
        ));
    }

    #[test]
    fn malformed_spec_section_is_reported_not_panicked() {
        // A valid FunctionV0 discriminant followed by a truncated body.
        let wasm = wasm_with_spec_section(&[0, 0, 0, 0]);
        assert!(matches!(
            fingerprint_wasm(&wasm),
            Err(FingerprintWasmError::MalformedSpec(_))
        ));
    }

    // ── Fixtures ────────────────────────────────────────────────────────────

    /// XDR for `ScSpecEntry::FunctionV0` with no doc, no inputs, no outputs.
    /// Hand-encoded so these tests stay free of `stellar-xdr`, which this crate
    /// deliberately does not depend on.
    fn function_entry_xdr(name: &str) -> Vec<u8> {
        let mut out = 0u32.to_be_bytes().to_vec(); // discriminant: FunctionV0
        out.extend_from_slice(&xdr_string("")); // doc
        out.extend_from_slice(&xdr_string(name)); // name
        out.extend_from_slice(&0u32.to_be_bytes()); // inputs: empty vec
        out.extend_from_slice(&0u32.to_be_bytes()); // outputs: empty vec
        out
    }

    /// XDR variable-length string: 4-byte big-endian length, bytes, zero pad to 4.
    fn xdr_string(value: &str) -> Vec<u8> {
        let bytes = value.as_bytes();
        let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(bytes);
        out.resize(out.len() + (4 - bytes.len() % 4) % 4, 0);
        out
    }

    fn wasm_with_spec_section(payload: &[u8]) -> Vec<u8> {
        let name = crate::wasm::CONTRACT_SPEC_SECTION.as_bytes();
        let mut body = leb128(name.len() as u64);
        body.extend_from_slice(name);
        body.extend_from_slice(payload);

        let mut out = EMPTY_MODULE.to_vec();
        out.push(0); // custom section id
        out.extend_from_slice(&leb128(body.len() as u64));
        out.extend_from_slice(&body);
        out
    }

    fn append_custom_section(mut wasm: Vec<u8>, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut body = leb128(name.len() as u64);
        body.extend_from_slice(name.as_bytes());
        body.extend_from_slice(payload);
        wasm.push(0);
        wasm.extend_from_slice(&leb128(body.len() as u64));
        wasm.extend_from_slice(&body);
        wasm
    }

    fn leb128(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return out;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_spec::{ScSpecFunctionInputV0, ScSpecFunctionV0};

    fn func(name: &str, inputs: Vec<(&str, ScSpecTypeDef)>, outputs: Vec<ScSpecTypeDef>) -> ScSpecEntry {
        ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
            doc: "some doc".into(),
            name: name.into(),
            inputs: inputs
                .into_iter()
                .map(|(n, t)| ScSpecFunctionInputV0 {
                    doc: "input doc".into(),
                    name: n.into(),
                    type_def: t,
                })
                .collect(),
            outputs,
        })
    }

    #[test]
    fn identical_specs_produce_identical_fingerprints() {
        let a = vec![func("transfer", vec![("to", ScSpecTypeDef::Address)], vec![ScSpecTypeDef::Bool])];
        let b = vec![func("transfer", vec![("to", ScSpecTypeDef::Address)], vec![ScSpecTypeDef::Bool])];
        assert_eq!(fingerprint_spec(&a).interface_id, fingerprint_spec(&b).interface_id);
    }

    #[test]
    fn doc_changes_do_not_affect_fingerprint() {
        let mut a = func("transfer", vec![], vec![]);
        let mut b = a.clone();
        if let ScSpecEntry::FunctionV0(f) = &mut a {
            f.doc = "Docs A".into();
        }
        if let ScSpecEntry::FunctionV0(f) = &mut b {
            f.doc = "Totally different docs".into();
        }
        assert_eq!(
            fingerprint_spec(&[a]).interface_id,
            fingerprint_spec(&[b]).interface_id
        );
    }

    #[test]
    fn entry_order_does_not_affect_interface_id() {
        let f1 = func("a", vec![], vec![]);
        let f2 = func("b", vec![], vec![]);
        let id1 = fingerprint_spec(&[f1.clone(), f2.clone()]).interface_id;
        let id2 = fingerprint_spec(&[f2, f1]).interface_id;
        assert_eq!(id1, id2);
    }

    #[test]
    fn removing_a_function_changes_the_interface_id() {
        let a = vec![func("a", vec![], vec![]), func("b", vec![], vec![])];
        let b = vec![func("a", vec![], vec![])];
        assert_ne!(fingerprint_spec(&a).interface_id, fingerprint_spec(&b).interface_id);
    }

    #[test]
    fn changing_a_parameter_type_changes_the_function_fingerprint() {
        let a = func("f", vec![("x", ScSpecTypeDef::U32)], vec![]);
        let b = func("f", vec![("x", ScSpecTypeDef::U64)], vec![]);
        let fa = fingerprint_spec(&[a]);
        let fb = fingerprint_spec(&[b]);
        assert_ne!(fa.functions[0].fingerprint, fb.functions[0].fingerprint);
        assert_ne!(fa.interface_id, fb.interface_id);
    }

    #[test]
    fn reordering_parameters_changes_the_function_fingerprint() {
        let a = func(
            "f",
            vec![("x", ScSpecTypeDef::U32), ("y", ScSpecTypeDef::U32)],
            vec![],
        );
        let b = func(
            "f",
            vec![("y", ScSpecTypeDef::U32), ("x", ScSpecTypeDef::U32)],
            vec![],
        );
        assert_ne!(
            fingerprint_spec(&[a]).functions[0].fingerprint,
            fingerprint_spec(&[b]).functions[0].fingerprint
        );
    }

    #[test]
    fn delimiter_confusable_functions_do_not_collide() {
        // Two inputs "a:u32" + "b:u64" naively joined with ',' produce the
        // same string as one input named "a" whose UDT type name is
        // "u32,b:u64" naively joined with ':'. The structured JSON hash
        // input must keep these distinguishable.
        let two_inputs = func(
            "f",
            vec![("a", ScSpecTypeDef::U32), ("b", ScSpecTypeDef::U64)],
            vec![],
        );
        let one_input = func(
            "f",
            vec![("a", ScSpecTypeDef::Udt("u32,b:u64".to_string()))],
            vec![],
        );
        assert_ne!(
            fingerprint_spec(&[two_inputs]).functions[0].fingerprint,
            fingerprint_spec(&[one_input]).functions[0].fingerprint
        );
    }

    #[test]
    fn empty_spec_has_a_stable_interface_id() {
        let fp = fingerprint_spec(&[]);
        assert_eq!(fp.algorithm, ALGORITHM);
        assert!(fp.functions.is_empty());
        // Even with no entries, the id is deterministic for the empty set.
        assert_eq!(fp.interface_id, fingerprint_spec(&[]).interface_id);
    }
}
