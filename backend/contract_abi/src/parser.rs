//! ABI Parser for Soroban contracts.

use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawContractSpec {
    #[serde(rename = "type")]
    pub spec_type: String,
    pub name: String,
    #[serde(default)]
    pub inputs: Vec<RawInputSpec>,
    #[serde(default)]
    pub outputs: Vec<RawOutputSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(default)]
    pub fields: Vec<RawFieldSpec>,
    #[serde(default)]
    pub cases: Vec<RawEnumCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInputSpec {
    pub name: String,
    pub value: RawTypeValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawOutputSpec {
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTypeValue {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<Box<RawTypeValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFieldSpec {
    pub name: String,
    pub value: RawTypeValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEnumCase {
    pub name: String,
    pub value: Option<u32>,
    #[serde(default)]
    pub fields: Vec<RawFieldSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    pub message: String,
    pub context: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            context: None,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ctx) = &self.context {
            write!(f, "{}: {}", ctx, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_contract_abi(
    specs: &[RawContractSpec],
    contract_name: &str,
) -> Result<ContractABI, ParseError> {
    let mut abi = ContractABI::new(contract_name.to_string());

    for spec in specs {
        match spec.spec_type.as_str() {
            "struct" => {
                let struct_type = parse_struct_type(spec)?;
                abi.types.insert(spec.name.clone(), struct_type);
            }
            "union" | "enum" => {
                let enum_type = parse_enum_type(spec)?;
                abi.types.insert(spec.name.clone(), enum_type);
            }
            "error_enum" => {
                let enum_type = parse_error_enum(spec)?;
                if let SorobanType::Enum { variants, .. } = &enum_type {
                    for variant in variants {
                        abi.errors.push(ContractError {
                            name: format!("{}::{}", spec.name, variant.name),
                            code: variant.value.unwrap_or(0),
                            doc: variant.doc.clone(),
                        });
                    }
                }
                abi.types.insert(spec.name.clone(), enum_type);
            }
            _ => {}
        }
    }

    for spec in specs {
        if spec.spec_type == "function" {
            let func = parse_function(spec, &abi.types)?;
            abi.functions.push(func);
        }
    }

    Ok(abi)
}

fn parse_struct_type(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    let fields: Vec<StructField> = spec
        .fields
        .iter()
        .map(|f| StructField {
            name: f.name.clone(),
            field_type: parse_type_value(&f.value),
            doc: f.doc.clone(),
        })
        .collect();
    Ok(SorobanType::Struct {
        name: spec.name.clone(),
        fields,
    })
}

fn parse_enum_type(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    let variants: Vec<EnumVariant> = spec
        .cases
        .iter()
        .map(|c| {
            let fields = if c.fields.is_empty() {
                None
            } else {
                Some(
                    c.fields
                        .iter()
                        .map(|f| StructField {
                            name: f.name.clone(),
                            field_type: parse_type_value(&f.value),
                            doc: f.doc.clone(),
                        })
                        .collect(),
                )
            };
            EnumVariant {
                name: c.name.clone(),
                value: c.value,
                fields,
                doc: c.doc.clone(),
            }
        })
        .collect();
    Ok(SorobanType::Enum {
        name: spec.name.clone(),
        variants,
    })
}

fn parse_error_enum(spec: &RawContractSpec) -> Result<SorobanType, ParseError> {
    parse_enum_type(spec)
}

fn parse_function(
    spec: &RawContractSpec,
    _types: &HashMap<String, SorobanType>,
) -> Result<ContractFunction, ParseError> {
    let params: Vec<FunctionParam> = spec
        .inputs
        .iter()
        .map(|input| FunctionParam {
            name: input.name.clone(),
            param_type: parse_type_value(&input.value),
            doc: input.doc.clone(),
        })
        .collect();

    let return_type = spec
        .outputs
        .first()
        .map(|o| SorobanType::from_type_string(&o.type_name))
        .unwrap_or(SorobanType::Void);

    let is_mutable = !spec.name.starts_with("get_")
        && !spec.name.starts_with("view_")
        && !spec.name.starts_with("query_")
        && !spec.name.starts_with("is_")
        && !spec.name.starts_with("has_");

    Ok(ContractFunction {
        name: spec.name.clone(),
        visibility: FunctionVisibility::Public,
        params,
        return_type,
        doc: spec.doc.clone(),
        is_mutable,
    })
}

fn parse_type_value(value: &RawTypeValue) -> SorobanType {
    match value.type_name.to_lowercase().as_str() {
        "bool" => SorobanType::Bool,
        "i32" => SorobanType::I32,
        "i64" => SorobanType::I64,
        "i128" => SorobanType::I128,
        "i256" => SorobanType::I256,
        "u32" => SorobanType::U32,
        "u64" => SorobanType::U64,
        "u128" => SorobanType::U128,
        "u256" => SorobanType::U256,
        "symbol" => SorobanType::Symbol,
        "string" => SorobanType::String,
        "bytes" => SorobanType::Bytes,
        "address" => SorobanType::Address,
        "void" | "()" => SorobanType::Void,
        "timepoint" => SorobanType::Timepoint,
        "duration" => SorobanType::Duration,
        "option" => {
            let inner = value
                .element
                .as_ref()
                .map(|e| parse_type_value(e))
                .unwrap_or(SorobanType::Void);
            SorobanType::Option {
                value_type: Box::new(inner),
            }
        }
        "vec" => {
            let elem = value
                .element
                .as_ref()
                .map(|e| parse_type_value(e))
                .unwrap_or(SorobanType::Void);
            SorobanType::Vec {
                element_type: Box::new(elem),
            }
        }
        "map" => {
            let key = value
                .key
                .as_ref()
                .map(|k| parse_type_value(k))
                .unwrap_or(SorobanType::Void);
            let val = value
                .val
                .as_ref()
                .map(|v| parse_type_value(v))
                .unwrap_or(SorobanType::Void);
            SorobanType::Map {
                key_type: Box::new(key),
                value_type: Box::new(val),
            }
        }
        "bytesn" => {
            let n = value.n.unwrap_or(32);
            SorobanType::BytesN { n }
        }
        _ => SorobanType::Custom {
            name: value.type_name.clone(),
        },
    }
}

pub fn parse_json_spec(json: &str, contract_name: &str) -> Result<ContractABI, ParseError> {
    let specs: Vec<RawContractSpec> = serde_json::from_str(json)
        .map_err(|e| ParseError::new(format!("Failed to parse JSON: {}", e)))?;
    parse_contract_abi(&specs, contract_name)
}
