//! Semantic ABI compatibility engine for Soroban contract specs (#1143).
//!
//! Structurally compares two normalized `contractspecv0` specs (see
//! [`crate::contract_spec`]) and classifies the differences, rather than
//! diffing raw JSON/WASM bytes. Depends on the entry model produced by the
//! same parser used for [`crate::interface_fingerprint`] (#1139), so a
//! function rename, a reordered parameter, or a changed struct field is
//! detected the same way in both engines.
//!
//! The single hard invariant here (called out explicitly in the tracking
//! issue): an incomplete source spec must never produce a `compatible` or
//! `breaking` verdict by accident. Any comparison built from a missing or
//! malformed side short-circuits to [`CompatibilityLevel::Unknown`].

use crate::contract_spec::{ScSpecEntry, ScSpecTypeDef, ScSpecUdtUnionCaseV0};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ALGORITHM: &str = "soroban-compatibility-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityLevel {
    Compatible,
    PotentiallyBreaking,
    Breaking,
    Unknown,
}

impl CompatibilityLevel {
    /// Combine two levels, keeping the more severe one. `Unknown` is most
    /// severe: once any part of the comparison is unknown, the overall
    /// result must not quietly claim `compatible`.
    fn severity_rank(self) -> u8 {
        match self {
            CompatibilityLevel::Compatible => 0,
            CompatibilityLevel::PotentiallyBreaking => 1,
            CompatibilityLevel::Breaking => 2,
            CompatibilityLevel::Unknown => 3,
        }
    }

    fn max(self, other: Self) -> Self {
        if other.severity_rank() > self.severity_rank() {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeCategory {
    Function,
    Type,
    Event,
    Error,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub category: ChangeCategory,
    pub level: CompatibilityLevel,
    /// Name of the affected function/type/event/error, or a fixed label
    /// (e.g. "network") for context-level changes.
    pub subject: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub algorithm: String,
    pub overall: CompatibilityLevel,
    pub changes: Vec<Change>,
}

impl CompatibilityReport {
    fn unknown(reason: &str) -> Self {
        Self {
            algorithm: ALGORITHM.to_string(),
            overall: CompatibilityLevel::Unknown,
            changes: vec![Change {
                category: ChangeCategory::Function,
                level: CompatibilityLevel::Unknown,
                subject: "spec".to_string(),
                description: reason.to_string(),
            }],
        }
    }

    pub fn has_at_least(&self, level: CompatibilityLevel) -> bool {
        self.changes
            .iter()
            .any(|c| c.level.severity_rank() >= level.severity_rank())
            || self.overall.severity_rank() >= level.severity_rank()
    }
}

/// One side of a compatibility comparison. `Missing`/`Malformed` both force
/// the overall result to [`CompatibilityLevel::Unknown`] — the engine never
/// guesses at a verdict from an incomplete spec.
pub enum SpecSource {
    /// Successfully parsed entries from a `contractspecv0` section.
    Entries(Vec<ScSpecEntry>),
    /// No contract spec section was present at all.
    Missing,
    /// A contract spec section was present but could not be parsed.
    Malformed(String),
}

/// Optional network identity context (Stellar network passphrase) for each
/// side, checked before the spec-level comparison runs.
#[derive(Debug, Clone, Default)]
pub struct NetworkContext {
    pub passphrase: Option<String>,
}

fn type_str(t: &ScSpecTypeDef) -> String {
    t.to_string()
}

fn union_case_key(case: &ScSpecUdtUnionCaseV0) -> &str {
    match case {
        ScSpecUdtUnionCaseV0::Void { name, .. } => name,
        ScSpecUdtUnionCaseV0::Tuple { name, .. } => name,
    }
}

fn union_case_types(case: &ScSpecUdtUnionCaseV0) -> Vec<ScSpecTypeDef> {
    match case {
        ScSpecUdtUnionCaseV0::Void { .. } => Vec::new(),
        ScSpecUdtUnionCaseV0::Tuple { types, .. } => types.clone(),
    }
}

/// Compare two parsed specs (already known to be present and well-formed)
/// and return the list of detected changes. This never returns `Unknown`
/// entries — callers are responsible for the missing/malformed short
/// circuit in [`compare`].
fn diff_entries(from: &[ScSpecEntry], to: &[ScSpecEntry]) -> Vec<Change> {
    let mut changes = Vec::new();

    diff_functions(from, to, &mut changes);
    diff_structs(from, to, &mut changes);
    diff_unions(from, to, &mut changes);
    diff_enums(from, to, &mut changes);
    diff_error_enums(from, to, &mut changes);
    diff_events(from, to, &mut changes);

    changes
}

fn index_by_name<'a, T>(
    entries: &'a [ScSpecEntry],
    select: impl Fn(&'a ScSpecEntry) -> Option<&'a T>,
    name_of: impl Fn(&'a T) -> &'a str,
) -> BTreeMap<&'a str, &'a T> {
    entries
        .iter()
        .filter_map(|e| select(e))
        .map(|item| (name_of(item), item))
        .collect()
}

fn diff_functions(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::FunctionV0(f) => Some(f),
            _ => None,
        },
        |f| f.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::FunctionV0(f) => Some(f),
            _ => None,
        },
        |f| f.name.as_str(),
    );

    for (name, f) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Function,
                level: CompatibilityLevel::Breaking,
                subject: name.to_string(),
                description: format!("Function `{name}` was removed"),
            }),
            Some(g) => {
                if f.inputs.len() != g.inputs.len() {
                    changes.push(Change {
                        category: ChangeCategory::Function,
                        level: CompatibilityLevel::Breaking,
                        subject: name.to_string(),
                        description: format!(
                            "Function `{name}` parameter count changed ({} -> {})",
                            f.inputs.len(),
                            g.inputs.len()
                        ),
                    });
                } else {
                    for (i, (fi, gi)) in f.inputs.iter().zip(g.inputs.iter()).enumerate() {
                        if fi.type_def != gi.type_def {
                            changes.push(Change {
                                category: ChangeCategory::Function,
                                level: CompatibilityLevel::Breaking,
                                subject: name.to_string(),
                                description: format!(
                                    "Function `{name}` parameter {i} type changed ({} -> {})",
                                    type_str(&fi.type_def),
                                    type_str(&gi.type_def)
                                ),
                            });
                        } else if fi.name != gi.name {
                            changes.push(Change {
                                category: ChangeCategory::Function,
                                level: CompatibilityLevel::PotentiallyBreaking,
                                subject: name.to_string(),
                                description: format!(
                                    "Function `{name}` parameter {i} renamed (`{}` -> `{}`)",
                                    fi.name, gi.name
                                ),
                            });
                        }
                    }
                }

                if f.outputs != g.outputs {
                    changes.push(Change {
                        category: ChangeCategory::Function,
                        level: CompatibilityLevel::Breaking,
                        subject: name.to_string(),
                        description: format!(
                            "Function `{name}` return type changed ({} -> {})",
                            f.outputs
                                .iter()
                                .map(type_str)
                                .collect::<Vec<_>>()
                                .join(","),
                            g.outputs
                                .iter()
                                .map(type_str)
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                    });
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Function,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Function `{name}` was added"),
            });
        }
    }
}

fn diff_structs(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::UdtStructV0(s) => Some(s),
            _ => None,
        },
        |s| s.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::UdtStructV0(s) => Some(s),
            _ => None,
        },
        |s| s.name.as_str(),
    );

    for (name, s) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Breaking,
                subject: name.to_string(),
                description: format!("Struct `{name}` was removed"),
            }),
            Some(t) => {
                let fa: BTreeMap<&str, &ScSpecTypeDef> = s
                    .fields
                    .iter()
                    .map(|f| (f.name.as_str(), &f.type_def))
                    .collect();
                let fb: BTreeMap<&str, &ScSpecTypeDef> = t
                    .fields
                    .iter()
                    .map(|f| (f.name.as_str(), &f.type_def))
                    .collect();

                for (fname, ftype) in &fa {
                    match fb.get(fname) {
                        None => changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::Breaking,
                            subject: name.to_string(),
                            description: format!("Struct `{name}` field `{fname}` was removed"),
                        }),
                        Some(other_type) => {
                            if ftype != other_type {
                                changes.push(Change {
                                    category: ChangeCategory::Type,
                                    level: CompatibilityLevel::Breaking,
                                    subject: name.to_string(),
                                    description: format!(
                                        "Struct `{name}` field `{fname}` type changed ({} -> {})",
                                        type_str(ftype),
                                        type_str(other_type)
                                    ),
                                });
                            }
                        }
                    }
                }

                for (fname, ftype) in &fb {
                    if !fa.contains_key(fname) {
                        let level = match ftype {
                            ScSpecTypeDef::Option(_) => CompatibilityLevel::PotentiallyBreaking,
                            _ => CompatibilityLevel::Breaking,
                        };
                        changes.push(Change {
                            category: ChangeCategory::Type,
                            level,
                            subject: name.to_string(),
                            description: format!(
                                "Struct `{name}` field `{fname}` was added ({})",
                                type_str(ftype)
                            ),
                        });
                    }
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Struct `{name}` was added"),
            });
        }
    }
}

fn diff_unions(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::UdtUnionV0(u) => Some(u),
            _ => None,
        },
        |u| u.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::UdtUnionV0(u) => Some(u),
            _ => None,
        },
        |u| u.name.as_str(),
    );

    for (name, u) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Breaking,
                subject: name.to_string(),
                description: format!("Union `{name}` was removed"),
            }),
            Some(v) => {
                let ca: BTreeMap<&str, Vec<ScSpecTypeDef>> = u
                    .cases
                    .iter()
                    .map(|c| (union_case_key(c), union_case_types(c)))
                    .collect();
                let cb: BTreeMap<&str, Vec<ScSpecTypeDef>> = v
                    .cases
                    .iter()
                    .map(|c| (union_case_key(c), union_case_types(c)))
                    .collect();

                for (case_name, types) in &ca {
                    match cb.get(case_name) {
                        None => changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::Breaking,
                            subject: name.to_string(),
                            description: format!(
                                "Union `{name}` case `{case_name}` was removed"
                            ),
                        }),
                        Some(other_types) => {
                            if types != other_types {
                                changes.push(Change {
                                    category: ChangeCategory::Type,
                                    level: CompatibilityLevel::Breaking,
                                    subject: name.to_string(),
                                    description: format!(
                                        "Union `{name}` case `{case_name}` payload changed"
                                    ),
                                });
                            }
                        }
                    }
                }

                for case_name in cb.keys() {
                    if !ca.contains_key(case_name) {
                        changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::PotentiallyBreaking,
                            subject: name.to_string(),
                            description: format!(
                                "Union `{name}` case `{case_name}` was added \
                                 (breaks exhaustive matches on consumers)"
                            ),
                        });
                    }
                }

                // Discriminant order is part of the wire ABI.
                let order_a: Vec<&str> = u.cases.iter().map(union_case_key).collect();
                let order_b: Vec<&str> = v.cases.iter().map(union_case_key).collect();
                let common_a: Vec<&&str> =
                    order_a.iter().filter(|c| cb.contains_key(**c)).collect();
                let common_b: Vec<&&str> =
                    order_b.iter().filter(|c| ca.contains_key(**c)).collect();
                if common_a != common_b {
                    changes.push(Change {
                        category: ChangeCategory::Type,
                        level: CompatibilityLevel::Breaking,
                        subject: name.to_string(),
                        description: format!(
                            "Union `{name}` case ordering (discriminant values) changed"
                        ),
                    });
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Union `{name}` was added"),
            });
        }
    }
}

fn diff_enums(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::UdtEnumV0(en) => Some(en),
            _ => None,
        },
        |en| en.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::UdtEnumV0(en) => Some(en),
            _ => None,
        },
        |en| en.name.as_str(),
    );

    for (name, en) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Breaking,
                subject: name.to_string(),
                description: format!("Enum `{name}` was removed"),
            }),
            Some(other) => {
                let ca: BTreeMap<&str, u32> =
                    en.cases.iter().map(|c| (c.name.as_str(), c.value)).collect();
                let cb: BTreeMap<&str, u32> = other
                    .cases
                    .iter()
                    .map(|c| (c.name.as_str(), c.value))
                    .collect();

                for (case_name, value) in &ca {
                    match cb.get(case_name) {
                        None => changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::Breaking,
                            subject: name.to_string(),
                            description: format!("Enum `{name}` case `{case_name}` was removed"),
                        }),
                        Some(other_value) if other_value != value => changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::Breaking,
                            subject: name.to_string(),
                            description: format!(
                                "Enum `{name}` case `{case_name}` value changed ({value} -> {other_value})"
                            ),
                        }),
                        _ => {}
                    }
                }

                for case_name in cb.keys() {
                    if !ca.contains_key(case_name) {
                        changes.push(Change {
                            category: ChangeCategory::Type,
                            level: CompatibilityLevel::PotentiallyBreaking,
                            subject: name.to_string(),
                            description: format!("Enum `{name}` case `{case_name}` was added"),
                        });
                    }
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Type,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Enum `{name}` was added"),
            });
        }
    }
}

fn diff_error_enums(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::UdtErrorEnumV0(en) => Some(en),
            _ => None,
        },
        |en| en.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::UdtErrorEnumV0(en) => Some(en),
            _ => None,
        },
        |en| en.name.as_str(),
    );

    for (name, en) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Error,
                level: CompatibilityLevel::PotentiallyBreaking,
                subject: name.to_string(),
                description: format!("Error enum `{name}` was removed"),
            }),
            Some(other) => {
                let ca: BTreeMap<&str, u32> =
                    en.cases.iter().map(|c| (c.name.as_str(), c.value)).collect();
                let cb: BTreeMap<&str, u32> = other
                    .cases
                    .iter()
                    .map(|c| (c.name.as_str(), c.value))
                    .collect();

                for (case_name, value) in &ca {
                    match cb.get(case_name) {
                        None => changes.push(Change {
                            category: ChangeCategory::Error,
                            level: CompatibilityLevel::PotentiallyBreaking,
                            subject: name.to_string(),
                            description: format!(
                                "Error `{name}::{case_name}` was removed or renamed"
                            ),
                        }),
                        Some(other_value) if other_value != value => changes.push(Change {
                            category: ChangeCategory::Error,
                            level: CompatibilityLevel::PotentiallyBreaking,
                            subject: name.to_string(),
                            description: format!(
                                "Error `{name}::{case_name}` code changed ({value} -> {other_value})"
                            ),
                        }),
                        _ => {}
                    }
                }

                for case_name in cb.keys() {
                    if !ca.contains_key(case_name) {
                        changes.push(Change {
                            category: ChangeCategory::Error,
                            level: CompatibilityLevel::Compatible,
                            subject: name.to_string(),
                            description: format!("Error `{name}::{case_name}` was added"),
                        });
                    }
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Error,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Error enum `{name}` was added"),
            });
        }
    }
}

fn diff_events(from: &[ScSpecEntry], to: &[ScSpecEntry], changes: &mut Vec<Change>) {
    let a = index_by_name(
        from,
        |e| match e {
            ScSpecEntry::EventV0(ev) => Some(ev),
            _ => None,
        },
        |ev| ev.name.as_str(),
    );
    let b = index_by_name(
        to,
        |e| match e {
            ScSpecEntry::EventV0(ev) => Some(ev),
            _ => None,
        },
        |ev| ev.name.as_str(),
    );

    for (name, ev) in &a {
        match b.get(name) {
            None => changes.push(Change {
                category: ChangeCategory::Event,
                level: CompatibilityLevel::PotentiallyBreaking,
                subject: name.to_string(),
                description: format!("Event `{name}` was removed (breaking for indexers)"),
            }),
            Some(other) => {
                if ev.data_format != other.data_format || ev.prefix_topics != other.prefix_topics
                {
                    changes.push(Change {
                        category: ChangeCategory::Event,
                        level: CompatibilityLevel::Breaking,
                        subject: name.to_string(),
                        description: format!(
                            "Event `{name}` topic layout or data format changed"
                        ),
                    });
                } else {
                    let pa: Vec<(&str, String)> = ev
                        .params
                        .iter()
                        .map(|p| (p.name.as_str(), type_str(&p.type_def)))
                        .collect();
                    let pb: Vec<(&str, String)> = other
                        .params
                        .iter()
                        .map(|p| (p.name.as_str(), type_str(&p.type_def)))
                        .collect();
                    if pa != pb {
                        changes.push(Change {
                            category: ChangeCategory::Event,
                            level: CompatibilityLevel::Breaking,
                            subject: name.to_string(),
                            description: format!("Event `{name}` payload shape changed"),
                        });
                    }
                }
            }
        }
    }

    for name in b.keys() {
        if !a.contains_key(name) {
            changes.push(Change {
                category: ChangeCategory::Event,
                level: CompatibilityLevel::Compatible,
                subject: name.to_string(),
                description: format!("Event `{name}` was added"),
            });
        }
    }
}

/// Compare two contract specs plus optional network identity context.
///
/// Returns [`CompatibilityLevel::Unknown`] as soon as either side is
/// missing or malformed, per the acceptance criterion that an incomplete
/// source spec must never silently produce a compatible/breaking verdict.
pub fn compare(
    from: &SpecSource,
    to: &SpecSource,
    from_network: &NetworkContext,
    to_network: &NetworkContext,
) -> CompatibilityReport {
    let (from_entries, to_entries) = match (from, to) {
        (SpecSource::Entries(a), SpecSource::Entries(b)) => (a, b),
        (SpecSource::Missing, _) | (_, SpecSource::Missing) => {
            return CompatibilityReport::unknown(
                "One or both contract specs are missing (no contractspecv0 section); \
                 compatibility cannot be determined.",
            );
        }
        (SpecSource::Malformed(reason), _) | (_, SpecSource::Malformed(reason)) => {
            return CompatibilityReport::unknown(&format!(
                "One or both contract specs are malformed and could not be parsed: {reason}"
            ));
        }
    };

    let mut changes = Vec::new();

    if let (Some(a), Some(b)) = (&from_network.passphrase, &to_network.passphrase) {
        if a != b {
            changes.push(Change {
                category: ChangeCategory::Network,
                level: CompatibilityLevel::Breaking,
                subject: "network".to_string(),
                description: format!(
                    "Network passphrase differs ({a} vs {b}); these are not the same \
                     deployment context and must not be compared as compatible."
                ),
            });
        }
    }

    changes.extend(diff_entries(from_entries, to_entries));

    let overall = changes
        .iter()
        .fold(CompatibilityLevel::Compatible, |acc, c| acc.max(c.level));

    CompatibilityReport {
        algorithm: ALGORITHM.to_string(),
        overall,
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_spec::{ScSpecFunctionInputV0, ScSpecFunctionV0};

    fn func(name: &str, inputs: Vec<(&str, ScSpecTypeDef)>, outputs: Vec<ScSpecTypeDef>) -> ScSpecEntry {
        ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
            doc: "".into(),
            name: name.into(),
            inputs: inputs
                .into_iter()
                .map(|(n, t)| ScSpecFunctionInputV0 {
                    doc: "".into(),
                    name: n.into(),
                    type_def: t,
                })
                .collect(),
            outputs,
        })
    }

    fn net() -> NetworkContext {
        NetworkContext::default()
    }

    #[test]
    fn identical_specs_are_compatible() {
        let a = vec![func("f", vec![], vec![])];
        let b = vec![func("f", vec![], vec![])];
        let report = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(b),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Compatible);
        assert!(report.changes.is_empty());
    }

    #[test]
    fn adding_a_function_is_compatible() {
        let a = vec![func("f", vec![], vec![])];
        let b = vec![func("f", vec![], vec![]), func("g", vec![], vec![])];
        let report = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(b),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Compatible);
    }

    #[test]
    fn removing_a_function_is_breaking() {
        let a = vec![func("f", vec![], vec![]), func("g", vec![], vec![])];
        let b = vec![func("f", vec![], vec![])];
        let report = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(b),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Breaking);
    }

    #[test]
    fn reordering_parameters_is_breaking() {
        let a = vec![func(
            "f",
            vec![("x", ScSpecTypeDef::U32), ("y", ScSpecTypeDef::U64)],
            vec![],
        )];
        let b = vec![func(
            "f",
            vec![("y", ScSpecTypeDef::U64), ("x", ScSpecTypeDef::U32)],
            vec![],
        )];
        let report = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(b),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Breaking);
    }

    #[test]
    fn changing_a_parameter_type_is_breaking() {
        let a = vec![func("f", vec![("x", ScSpecTypeDef::U32)], vec![])];
        let b = vec![func("f", vec![("x", ScSpecTypeDef::U64)], vec![])];
        let report = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(b),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Breaking);
    }

    #[test]
    fn missing_source_spec_is_unknown_not_breaking() {
        let report = compare(
            &SpecSource::Missing,
            &SpecSource::Entries(vec![func("f", vec![], vec![])]),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Unknown);
    }

    #[test]
    fn malformed_spec_is_unknown() {
        let report = compare(
            &SpecSource::Malformed("truncated".into()),
            &SpecSource::Entries(vec![]),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::Unknown);
    }

    #[test]
    fn different_network_passphrase_is_breaking() {
        let a = vec![func("f", vec![], vec![])];
        let b = vec![func("f", vec![], vec![])];
        let from_net = NetworkContext {
            passphrase: Some("Public Global Stellar Network ; September 2015".into()),
        };
        let to_net = NetworkContext {
            passphrase: Some("Test SDF Network ; September 2015".into()),
        };
        let report = compare(&SpecSource::Entries(a), &SpecSource::Entries(b), &from_net, &to_net);
        assert_eq!(report.overall, CompatibilityLevel::Breaking);
        assert!(report
            .changes
            .iter()
            .any(|c| c.category == ChangeCategory::Network));
    }

    #[test]
    fn identical_network_passphrase_does_not_flag() {
        let from_net = NetworkContext {
            passphrase: Some("Test SDF Network ; September 2015".into()),
        };
        let to_net = from_net.clone();
        let report = compare(
            &SpecSource::Entries(vec![]),
            &SpecSource::Entries(vec![]),
            &from_net,
            &to_net,
        );
        assert_eq!(report.overall, CompatibilityLevel::Compatible);
    }

    #[test]
    fn adding_required_struct_field_is_breaking_but_optional_is_potentially_breaking() {
        use crate::contract_spec::{ScSpecUdtStructFieldV0, ScSpecUdtStructV0};

        let make = |fields: Vec<ScSpecUdtStructFieldV0>| {
            ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
                doc: "".into(),
                lib: "".into(),
                name: "Position".into(),
                fields,
            })
        };

        let a = vec![make(vec![])];
        let required = vec![make(vec![ScSpecUdtStructFieldV0 {
            doc: "".into(),
            name: "amount".into(),
            type_def: ScSpecTypeDef::I64,
        }])];
        let optional = vec![make(vec![ScSpecUdtStructFieldV0 {
            doc: "".into(),
            name: "amount".into(),
            type_def: ScSpecTypeDef::Option(Box::new(ScSpecTypeDef::I64)),
        }])];

        let report_required = compare(
            &SpecSource::Entries(a.clone()),
            &SpecSource::Entries(required),
            &net(),
            &net(),
        );
        assert_eq!(report_required.overall, CompatibilityLevel::Breaking);

        let report_optional = compare(
            &SpecSource::Entries(a),
            &SpecSource::Entries(optional),
            &net(),
            &net(),
        );
        assert_eq!(
            report_optional.overall,
            CompatibilityLevel::PotentiallyBreaking
        );
    }

    #[test]
    fn removing_an_event_is_potentially_breaking() {
        use crate::contract_spec::{ScSpecEventDataFormat, ScSpecEventV0};

        let ev = ScSpecEntry::EventV0(ScSpecEventV0 {
            doc: "".into(),
            lib: "".into(),
            name: "transfer".into(),
            prefix_topics: vec![],
            params: vec![],
            data_format: ScSpecEventDataFormat::SingleValue,
        });
        let report = compare(
            &SpecSource::Entries(vec![ev]),
            &SpecSource::Entries(vec![]),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::PotentiallyBreaking);
    }

    #[test]
    fn changing_an_error_code_is_potentially_breaking() {
        use crate::contract_spec::{ScSpecUdtErrorEnumCaseV0, ScSpecUdtErrorEnumV0};

        let make = |value: u32| {
            ScSpecEntry::UdtErrorEnumV0(ScSpecUdtErrorEnumV0 {
                doc: "".into(),
                lib: "".into(),
                name: "Error".into(),
                cases: vec![ScSpecUdtErrorEnumCaseV0 {
                    doc: "".into(),
                    name: "NotFound".into(),
                    value,
                }],
            })
        };

        let report = compare(
            &SpecSource::Entries(vec![make(1)]),
            &SpecSource::Entries(vec![make(2)]),
            &net(),
            &net(),
        );
        assert_eq!(report.overall, CompatibilityLevel::PotentiallyBreaking);
    }
}
