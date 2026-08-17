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
    ScSpecUdtUnionCaseV0,
};
use serde::{Deserialize, Serialize};
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

/// Build the canonical signature and derived fingerprint for a single spec
/// entry. Returns `None` for entry variants that carry no independent
/// identity (there are none today, but this keeps the mapping total and
/// future-proof).
fn fingerprint_entry(entry: &ScSpecEntry) -> (EntryKind, String, String) {
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
            (EntryKind::Function, f.name.clone(), sig)
        }
        ScSpecEntry::UdtStructV0(s) => {
            let fields: Vec<String> = s
                .fields
                .iter()
                .map(|field| format!("{}:{}", field.name, canonical_type(&field.type_def)))
                .collect();
            let sig = format!("struct {} {{{}}}", s.name, fields.join(","));
            (EntryKind::Type, s.name.clone(), sig)
        }
        ScSpecEntry::UdtUnionV0(u) => {
            let mut cases: Vec<String> = u.cases.iter().map(canonical_union_case).collect();
            // Union case order is part of the wire ABI (it is the
            // discriminant), so it is preserved rather than sorted.
            let _ = &mut cases;
            let sig = format!("union {} {{{}}}", u.name, cases.join(","));
            (EntryKind::Type, u.name.clone(), sig)
        }
        ScSpecEntry::UdtEnumV0(e) => {
            let mut cases: Vec<(u32, &str)> =
                e.cases.iter().map(|c| (c.value, c.name.as_str())).collect();
            cases.sort_by_key(|(v, _)| *v);
            let cases: Vec<String> = cases
                .into_iter()
                .map(|(v, name)| format!("{name}={v}"))
                .collect();
            let sig = format!("enum {} {{{}}}", e.name, cases.join(","));
            (EntryKind::Type, e.name.clone(), sig)
        }
        ScSpecEntry::UdtErrorEnumV0(e) => {
            let mut cases: Vec<(u32, &str)> =
                e.cases.iter().map(|c| (c.value, c.name.as_str())).collect();
            cases.sort_by_key(|(v, _)| *v);
            let cases: Vec<String> = cases
                .into_iter()
                .map(|(v, name)| format!("{name}={v}"))
                .collect();
            let sig = format!("error {} {{{}}}", e.name, cases.join(","));
            (EntryKind::Error, e.name.clone(), sig)
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
            (EntryKind::Event, ev.name.clone(), sig)
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
        let (kind, name, signature) = fingerprint_entry(entry);
        let fingerprint = sha256_hex(&format!("{ALGORITHM}:{kind:?}:{signature}"));
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
    fn empty_spec_has_a_stable_interface_id() {
        let fp = fingerprint_spec(&[]);
        assert_eq!(fp.algorithm, ALGORITHM);
        assert!(fp.functions.is_empty());
        // Even with no entries, the id is deterministic for the empty set.
        assert_eq!(fp.interface_id, fingerprint_spec(&[]).interface_id);
    }
}
