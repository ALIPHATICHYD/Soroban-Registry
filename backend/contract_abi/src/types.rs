//! Type definitions for Soroban contract ABI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soroban native types supported in contracts
#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SorobanType {
    Bool,
    I32,
    I64,
    I128,
    I256,
    U32,
    U64,
    U128,
    U256,
    Symbol,
    String,
    Bytes,
    BytesN {
        n: u32,
    },
    Address,
    Void,
    Timepoint,
    Duration,
    Option {
        value_type: Box<SorobanType>,
    },
    Result {
        ok_type: Box<SorobanType>,
        err_type: Box<SorobanType>,
    },
    Vec {
        element_type: Box<SorobanType>,
    },
    Map {
        key_type: Box<SorobanType>,
        value_type: Box<SorobanType>,
    },
    Tuple {
        elements: Vec<SorobanType>,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    Custom {
        name: String,
    },
}

impl SorobanType {
    pub fn from_type_string(type_str: &str) -> Self {
        let trimmed = type_str.trim();
        match trimmed.to_lowercase().as_str() {
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
            _ => {
                if let Some(inner) = Self::extract_generic(trimmed, "Option") {
                    return SorobanType::Option {
                        value_type: Box::new(Self::from_type_string(&inner)),
                    };
                }
                if let Some(inner) = Self::extract_generic(trimmed, "Vec") {
                    return SorobanType::Vec {
                        element_type: Box::new(Self::from_type_string(&inner)),
                    };
                }
                if let Some(n) = Self::extract_bytes_n(trimmed) {
                    return SorobanType::BytesN { n };
                }
                SorobanType::Custom {
                    name: trimmed.to_string(),
                }
            }
        }
    }

    fn extract_generic(type_str: &str, wrapper: &str) -> Option<String> {
        let prefix = format!("{}<", wrapper);
        if type_str.starts_with(&prefix) && type_str.ends_with('>') {
            Some(type_str[prefix.len()..type_str.len() - 1].to_string())
        } else {
            None
        }
    }

    fn extract_bytes_n(type_str: &str) -> Option<u32> {
        if type_str.starts_with("BytesN<") && type_str.ends_with('>') {
            type_str[7..type_str.len() - 1].parse().ok()
        } else {
            None
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            SorobanType::Bool => "bool".to_string(),
            SorobanType::I32 => "i32".to_string(),
            SorobanType::I64 => "i64".to_string(),
            SorobanType::I128 => "i128".to_string(),
            SorobanType::I256 => "i256".to_string(),
            SorobanType::U32 => "u32".to_string(),
            SorobanType::U64 => "u64".to_string(),
            SorobanType::U128 => "u128".to_string(),
            SorobanType::U256 => "u256".to_string(),
            SorobanType::Symbol => "Symbol".to_string(),
            SorobanType::String => "String".to_string(),
            SorobanType::Bytes => "Bytes".to_string(),
            SorobanType::BytesN { n } => format!("BytesN<{}>", n),
            SorobanType::Address => "Address".to_string(),
            SorobanType::Void => "void".to_string(),
            SorobanType::Timepoint => "Timepoint".to_string(),
            SorobanType::Duration => "Duration".to_string(),
            SorobanType::Option { value_type } => format!("Option<{}>", value_type.display_name()),
            SorobanType::Result { ok_type, err_type } => {
                format!(
                    "Result<{}, {}>",
                    ok_type.display_name(),
                    err_type.display_name()
                )
            }
            SorobanType::Vec { element_type } => format!("Vec<{}>", element_type.display_name()),
            SorobanType::Map {
                key_type,
                value_type,
            } => {
                format!(
                    "Map<{}, {}>",
                    key_type.display_name(),
                    value_type.display_name()
                )
            }
            SorobanType::Tuple { elements } => {
                let inner: Vec<String> = elements.iter().map(|e| e.display_name()).collect();
                format!("({})", inner.join(", "))
            }
            SorobanType::Struct { name, .. } => name.clone(),
            SorobanType::Enum { name, .. } => name.clone(),
            SorobanType::Custom { name } => name.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: SorobanType,
    pub doc: Option<String>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<u32>,
    pub fields: Option<Vec<StructField>>,
    pub doc: Option<String>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FunctionVisibility {
    #[default]
    Public,
    Internal,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: SorobanType,
    pub doc: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ContractFunction {
    pub name: String,
    pub visibility: FunctionVisibility,
    pub params: Vec<FunctionParam>,
    pub return_type: SorobanType,
    pub doc: Option<String>,
    pub is_mutable: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ContractABI {
    pub name: String,
    pub version: Option<String>,
    pub functions: Vec<ContractFunction>,
    pub types: HashMap<String, SorobanType>,
    pub events: Vec<ContractEvent>,
    pub errors: Vec<ContractError>,
}

impl ContractABI {
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            functions: Vec::new(),
            types: HashMap::new(),
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<&ContractFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn public_functions(&self) -> impl Iterator<Item = &ContractFunction> {
        self.functions
            .iter()
            .filter(|f| f.visibility == FunctionVisibility::Public)
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|f| f.name == name)
    }
}

#[derive(Serialize, Deserialize)]
pub struct ContractEvent {
    pub name: String,
    pub topics: Vec<FunctionParam>,
    pub data: Vec<FunctionParam>,
    pub doc: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ContractError {
    pub name: String,
    pub code: u32,
    pub doc: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soroban_type_serialization_round_trips() {
        let ty = SorobanType::Map {
            key_type: Box::new(SorobanType::Symbol),
            value_type: Box::new(SorobanType::Option {
                value_type: Box::new(SorobanType::BytesN { n: 32 }),
            }),
        };

        let json = serde_json::to_string(&ty).expect("SorobanType should serialize");
        let decoded: SorobanType =
            serde_json::from_str(&json).expect("SorobanType should deserialize");

        assert!(matches!(
            decoded,
            SorobanType::Map {
                key_type,
                value_type
            } if *key_type == SorobanType::Symbol
                && matches!(*value_type, SorobanType::Option { .. })
        ));
    }

    #[test]
    fn contract_abi_serialization_round_trips() {
        let mut abi = ContractABI::new("Token".to_string());
        abi.version = Some("1.0.0".to_string());
        abi.types.insert(
            "Balance".to_string(),
            SorobanType::Struct {
                name: "Balance".to_string(),
                fields: vec![StructField {
                    name: "amount".to_string(),
                    field_type: SorobanType::I128,
                    doc: Some("Current balance".to_string()),
                }],
            },
        );
        abi.functions.push(ContractFunction {
            name: "balance".to_string(),
            visibility: FunctionVisibility::Public,
            params: vec![FunctionParam {
                name: "id".to_string(),
                param_type: SorobanType::Address,
                doc: None,
            }],
            return_type: SorobanType::I128,
            doc: Some("Read balance".to_string()),
            is_mutable: false,
        });
        abi.events.push(ContractEvent {
            name: "Transfer".to_string(),
            topics: vec![FunctionParam {
                name: "from".to_string(),
                param_type: SorobanType::Address,
                doc: None,
            }],
            data: vec![FunctionParam {
                name: "amount".to_string(),
                param_type: SorobanType::I128,
                doc: None,
            }],
            doc: None,
        });
        abi.errors.push(ContractError {
            name: "InsufficientBalance".to_string(),
            code: 1,
            doc: None,
        });

        let json = serde_json::to_string(&abi).expect("ContractABI should serialize");
        let decoded: ContractABI =
            serde_json::from_str(&json).expect("ContractABI should deserialize");

        assert_eq!(decoded.name, "Token");
        assert_eq!(decoded.version.as_deref(), Some("1.0.0"));
        assert!(decoded.has_function("balance"));
        assert_eq!(decoded.functions[0].return_type.display_name(), "i128");
        assert_eq!(
            decoded.events[0].topics[0].param_type.display_name(),
            "Address"
        );
        assert_eq!(decoded.errors[0].code, 1);
    }
}
