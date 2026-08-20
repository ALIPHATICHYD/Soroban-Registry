//! Decoder for the Soroban `contractspecv0` custom WASM section.
//!
//! The section is a concatenation of XDR-encoded `SCSpecEntry` values (no
//! outer length prefix — entries are read back to back until the byte slice
//! is exhausted). This module implements a minimal, dependency-free XDR
//! reader for exactly the subset of the Soroban contract-spec XDR schema
//! needed to enumerate functions, user-defined types, events and errors:
//! this crate intentionally avoids pulling in `stellar-xdr` so that
//! `backend/shared` stays usable from every workspace member without adding
//! a new dependency.
//!
//! Reference: `Stellar-contract-spec.x` (`SCSpecEntry`, `SCSpecTypeDef`).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A single decoded entry from a `contractspecv0` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScSpecEntry {
    FunctionV0(ScSpecFunctionV0),
    UdtStructV0(ScSpecUdtStructV0),
    UdtUnionV0(ScSpecUdtUnionV0),
    UdtEnumV0(ScSpecUdtEnumV0),
    UdtErrorEnumV0(ScSpecUdtErrorEnumV0),
    EventV0(ScSpecEventV0),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecFunctionV0 {
    pub doc: String,
    pub name: String,
    pub inputs: Vec<ScSpecFunctionInputV0>,
    pub outputs: Vec<ScSpecTypeDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecFunctionInputV0 {
    pub doc: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_def: ScSpecTypeDef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtStructFieldV0 {
    pub doc: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_def: ScSpecTypeDef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtStructV0 {
    pub doc: String,
    pub lib: String,
    pub name: String,
    pub fields: Vec<ScSpecUdtStructFieldV0>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScSpecUdtUnionCaseV0 {
    Void { doc: String, name: String },
    Tuple {
        doc: String,
        name: String,
        types: Vec<ScSpecTypeDef>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtUnionV0 {
    pub doc: String,
    pub lib: String,
    pub name: String,
    pub cases: Vec<ScSpecUdtUnionCaseV0>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtEnumCaseV0 {
    pub doc: String,
    pub name: String,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtEnumV0 {
    pub doc: String,
    pub lib: String,
    pub name: String,
    pub cases: Vec<ScSpecUdtEnumCaseV0>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtErrorEnumCaseV0 {
    pub doc: String,
    pub name: String,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecUdtErrorEnumV0 {
    pub doc: String,
    pub lib: String,
    pub name: String,
    pub cases: Vec<ScSpecUdtErrorEnumCaseV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScSpecEventParamLocationV0 {
    Data,
    TopicList,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecEventParamV0 {
    pub doc: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_def: ScSpecTypeDef,
    pub location: ScSpecEventParamLocationV0,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScSpecEventDataFormat {
    SingleValue,
    Vec,
    Map,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScSpecEventV0 {
    pub doc: String,
    pub lib: String,
    pub name: String,
    pub prefix_topics: Vec<String>,
    pub params: Vec<ScSpecEventParamV0>,
    pub data_format: ScSpecEventDataFormat,
}

/// A structural Soroban contract-spec type reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScSpecTypeDef {
    Val,
    Bool,
    Void,
    Error,
    U32,
    I32,
    U64,
    I64,
    Timepoint,
    Duration,
    U128,
    I128,
    U256,
    I256,
    Bytes,
    String,
    Symbol,
    Address,
    MuxedAddress,
    Option(Box<ScSpecTypeDef>),
    Result {
        ok: Box<ScSpecTypeDef>,
        error: Box<ScSpecTypeDef>,
    },
    Vec(Box<ScSpecTypeDef>),
    Map {
        key: Box<ScSpecTypeDef>,
        value: Box<ScSpecTypeDef>,
    },
    Tuple(Vec<ScSpecTypeDef>),
    BytesN(u32),
    Udt(String),
}

impl fmt::Display for ScSpecTypeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScSpecTypeDef::Val => write!(f, "Val"),
            ScSpecTypeDef::Bool => write!(f, "bool"),
            ScSpecTypeDef::Void => write!(f, "void"),
            ScSpecTypeDef::Error => write!(f, "error"),
            ScSpecTypeDef::U32 => write!(f, "u32"),
            ScSpecTypeDef::I32 => write!(f, "i32"),
            ScSpecTypeDef::U64 => write!(f, "u64"),
            ScSpecTypeDef::I64 => write!(f, "i64"),
            ScSpecTypeDef::Timepoint => write!(f, "timepoint"),
            ScSpecTypeDef::Duration => write!(f, "duration"),
            ScSpecTypeDef::U128 => write!(f, "u128"),
            ScSpecTypeDef::I128 => write!(f, "i128"),
            ScSpecTypeDef::U256 => write!(f, "u256"),
            ScSpecTypeDef::I256 => write!(f, "i256"),
            ScSpecTypeDef::Bytes => write!(f, "bytes"),
            ScSpecTypeDef::String => write!(f, "string"),
            ScSpecTypeDef::Symbol => write!(f, "symbol"),
            ScSpecTypeDef::Address => write!(f, "address"),
            ScSpecTypeDef::MuxedAddress => write!(f, "muxed_address"),
            ScSpecTypeDef::Option(inner) => write!(f, "option<{}>", inner),
            ScSpecTypeDef::Result { ok, error } => write!(f, "result<{}, {}>", ok, error),
            ScSpecTypeDef::Vec(inner) => write!(f, "vec<{}>", inner),
            ScSpecTypeDef::Map { key, value } => write!(f, "map<{}, {}>", key, value),
            ScSpecTypeDef::Tuple(items) => {
                write!(f, "tuple<")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ">")
            }
            ScSpecTypeDef::BytesN(n) => write!(f, "bytes{}", n),
            ScSpecTypeDef::Udt(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpecParseError {
    UnexpectedEof { at: usize },
    InvalidDiscriminant { at: usize, value: i32 },
    InvalidUtf8 { at: usize },
    TrailingBytes { at: usize },
    TypeNestingTooDeep { at: usize },
}

impl fmt::Display for SpecParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecParseError::UnexpectedEof { at } => {
                write!(f, "unexpected end of contractspecv0 section at byte {at}")
            }
            SpecParseError::InvalidDiscriminant { at, value } => write!(
                f,
                "unrecognized XDR discriminant {value} at byte {at}"
            ),
            SpecParseError::InvalidUtf8 { at } => {
                write!(f, "invalid UTF-8 string at byte {at}")
            }
            SpecParseError::TrailingBytes { at } => {
                write!(f, "trailing unparsed bytes starting at {at}")
            }
            SpecParseError::TypeNestingTooDeep { at } => write!(
                f,
                "type nesting exceeds maximum depth of {MAX_TYPE_DEPTH} at byte {at}"
            ),
        }
    }
}

impl std::error::Error for SpecParseError {}

/// Upper bound on `ScSpecTypeDef` nesting (`Option<Option<...>>`, etc.).
/// Real Soroban contract specs nest a handful of levels deep at most; this
/// exists purely to bound recursion against a crafted/corrupted section, not
/// to constrain legitimate specs.
const MAX_TYPE_DEPTH: usize = 32;

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    type_depth: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            type_depth: 0,
        }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn read_u32(&mut self) -> Result<u32, SpecParseError> {
        if self.remaining() < 4 {
            return Err(SpecParseError::UnexpectedEof { at: self.pos });
        }
        let v = u32::from_be_bytes(self.bytes[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    fn read_i32(&mut self) -> Result<i32, SpecParseError> {
        Ok(self.read_u32()? as i32)
    }

    /// XDR variable-length opaque/string: u32 length, bytes, zero-padded to a
    /// 4-byte boundary.
    fn read_var_bytes(&mut self) -> Result<&'a [u8], SpecParseError> {
        let len = self.read_u32()? as usize;
        if self.remaining() < len {
            return Err(SpecParseError::UnexpectedEof { at: self.pos });
        }
        let start = self.pos;
        let data = &self.bytes[start..start + len];
        self.pos += len;
        let padding = (4 - (len % 4)) % 4;
        if self.remaining() < padding {
            return Err(SpecParseError::UnexpectedEof { at: self.pos });
        }
        self.pos += padding;
        Ok(data)
    }

    fn read_string(&mut self) -> Result<String, SpecParseError> {
        let at = self.pos;
        let bytes = self.read_var_bytes()?;
        String::from_utf8(bytes.to_vec()).map_err(|_| SpecParseError::InvalidUtf8 { at })
    }

    fn read_vec<T>(
        &mut self,
        mut read_item: impl FnMut(&mut Self) -> Result<T, SpecParseError>,
    ) -> Result<Vec<T>, SpecParseError> {
        let count = self.read_u32()? as usize;
        let mut items = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            items.push(read_item(self)?);
        }
        Ok(items)
    }

    fn read_type_def(&mut self) -> Result<ScSpecTypeDef, SpecParseError> {
        let at = self.pos;
        if self.type_depth >= MAX_TYPE_DEPTH {
            return Err(SpecParseError::TypeNestingTooDeep { at });
        }
        self.type_depth += 1;
        let result = self.read_type_def_inner(at);
        self.type_depth -= 1;
        result
    }

    fn read_type_def_inner(&mut self, at: usize) -> Result<ScSpecTypeDef, SpecParseError> {
        let disc = self.read_i32()?;
        let ty = match disc {
            0 => ScSpecTypeDef::Val,
            1 => ScSpecTypeDef::Bool,
            2 => ScSpecTypeDef::Void,
            3 => ScSpecTypeDef::Error,
            4 => ScSpecTypeDef::U32,
            5 => ScSpecTypeDef::I32,
            6 => ScSpecTypeDef::U64,
            7 => ScSpecTypeDef::I64,
            8 => ScSpecTypeDef::Timepoint,
            9 => ScSpecTypeDef::Duration,
            10 => ScSpecTypeDef::U128,
            11 => ScSpecTypeDef::I128,
            12 => ScSpecTypeDef::U256,
            13 => ScSpecTypeDef::I256,
            14 => ScSpecTypeDef::Bytes,
            16 => ScSpecTypeDef::String,
            17 => ScSpecTypeDef::Symbol,
            19 => ScSpecTypeDef::Address,
            20 => ScSpecTypeDef::MuxedAddress,
            1000 => ScSpecTypeDef::Option(Box::new(self.read_type_def()?)),
            1001 => {
                let ok = Box::new(self.read_type_def()?);
                let error = Box::new(self.read_type_def()?);
                ScSpecTypeDef::Result { ok, error }
            }
            1002 => ScSpecTypeDef::Vec(Box::new(self.read_type_def()?)),
            1004 => {
                let key = Box::new(self.read_type_def()?);
                let value = Box::new(self.read_type_def()?);
                ScSpecTypeDef::Map { key, value }
            }
            1005 => {
                let items = self.read_vec(|c| c.read_type_def())?;
                ScSpecTypeDef::Tuple(items)
            }
            1006 => ScSpecTypeDef::BytesN(self.read_u32()?),
            2000 => ScSpecTypeDef::Udt(self.read_string()?),
            other => {
                return Err(SpecParseError::InvalidDiscriminant { at, value: other });
            }
        };
        Ok(ty)
    }

    fn read_function_input(&mut self) -> Result<ScSpecFunctionInputV0, SpecParseError> {
        let doc = self.read_string()?;
        let name = self.read_string()?;
        let type_def = self.read_type_def()?;
        Ok(ScSpecFunctionInputV0 {
            doc,
            name,
            type_def,
        })
    }

    fn read_function(&mut self) -> Result<ScSpecFunctionV0, SpecParseError> {
        let doc = self.read_string()?;
        let name = self.read_string()?;
        let inputs = self.read_vec(|c| c.read_function_input())?;
        let outputs = self.read_vec(|c| c.read_type_def())?;
        Ok(ScSpecFunctionV0 {
            doc,
            name,
            inputs,
            outputs,
        })
    }

    fn read_struct(&mut self) -> Result<ScSpecUdtStructV0, SpecParseError> {
        let doc = self.read_string()?;
        let lib = self.read_string()?;
        let name = self.read_string()?;
        let fields = self.read_vec(|c| {
            let doc = c.read_string()?;
            let name = c.read_string()?;
            let type_def = c.read_type_def()?;
            Ok(ScSpecUdtStructFieldV0 {
                doc,
                name,
                type_def,
            })
        })?;
        Ok(ScSpecUdtStructV0 {
            doc,
            lib,
            name,
            fields,
        })
    }

    fn read_union(&mut self) -> Result<ScSpecUdtUnionV0, SpecParseError> {
        let doc = self.read_string()?;
        let lib = self.read_string()?;
        let name = self.read_string()?;
        let cases = self.read_vec(|c| {
            let at = c.pos;
            let kind = c.read_i32()?;
            match kind {
                0 => {
                    let doc = c.read_string()?;
                    let name = c.read_string()?;
                    Ok(ScSpecUdtUnionCaseV0::Void { doc, name })
                }
                1 => {
                    let doc = c.read_string()?;
                    let name = c.read_string()?;
                    let types = c.read_vec(|c2| c2.read_type_def())?;
                    Ok(ScSpecUdtUnionCaseV0::Tuple { doc, name, types })
                }
                other => Err(SpecParseError::InvalidDiscriminant { at, value: other }),
            }
        })?;
        Ok(ScSpecUdtUnionV0 {
            doc,
            lib,
            name,
            cases,
        })
    }

    fn read_enum(&mut self) -> Result<ScSpecUdtEnumV0, SpecParseError> {
        let doc = self.read_string()?;
        let lib = self.read_string()?;
        let name = self.read_string()?;
        let cases = self.read_vec(|c| {
            let doc = c.read_string()?;
            let name = c.read_string()?;
            let value = c.read_u32()?;
            Ok(ScSpecUdtEnumCaseV0 { doc, name, value })
        })?;
        Ok(ScSpecUdtEnumV0 {
            doc,
            lib,
            name,
            cases,
        })
    }

    fn read_error_enum(&mut self) -> Result<ScSpecUdtErrorEnumV0, SpecParseError> {
        let doc = self.read_string()?;
        let lib = self.read_string()?;
        let name = self.read_string()?;
        let cases = self.read_vec(|c| {
            let doc = c.read_string()?;
            let name = c.read_string()?;
            let value = c.read_u32()?;
            Ok(ScSpecUdtErrorEnumCaseV0 { doc, name, value })
        })?;
        Ok(ScSpecUdtErrorEnumV0 {
            doc,
            lib,
            name,
            cases,
        })
    }

    fn read_event(&mut self) -> Result<ScSpecEventV0, SpecParseError> {
        let doc = self.read_string()?;
        let lib = self.read_string()?;
        let name = self.read_string()?;
        let prefix_topics = self.read_vec(|c| c.read_string())?;
        let params = self.read_vec(|c| {
            let doc = c.read_string()?;
            let name = c.read_string()?;
            let type_def = c.read_type_def()?;
            let at = c.pos;
            let location = match c.read_i32()? {
                0 => ScSpecEventParamLocationV0::Data,
                1 => ScSpecEventParamLocationV0::TopicList,
                other => return Err(SpecParseError::InvalidDiscriminant { at, value: other }),
            };
            Ok(ScSpecEventParamV0 {
                doc,
                name,
                type_def,
                location,
            })
        })?;
        let at = self.pos;
        let data_format = match self.read_i32()? {
            0 => ScSpecEventDataFormat::SingleValue,
            1 => ScSpecEventDataFormat::Vec,
            2 => ScSpecEventDataFormat::Map,
            other => return Err(SpecParseError::InvalidDiscriminant { at, value: other }),
        };
        Ok(ScSpecEventV0 {
            doc,
            lib,
            name,
            prefix_topics,
            params,
            data_format,
        })
    }

    fn read_entry(&mut self) -> Result<ScSpecEntry, SpecParseError> {
        let at = self.pos;
        let kind = self.read_i32()?;
        match kind {
            0 => Ok(ScSpecEntry::FunctionV0(self.read_function()?)),
            1 => Ok(ScSpecEntry::UdtStructV0(self.read_struct()?)),
            2 => Ok(ScSpecEntry::UdtUnionV0(self.read_union()?)),
            3 => Ok(ScSpecEntry::UdtEnumV0(self.read_enum()?)),
            4 => Ok(ScSpecEntry::UdtErrorEnumV0(self.read_error_enum()?)),
            5 => Ok(ScSpecEntry::EventV0(self.read_event()?)),
            other => Err(SpecParseError::InvalidDiscriminant { at, value: other }),
        }
    }
}

/// Parse the raw bytes of a `contractspecv0` custom section into its
/// sequence of entries. Returns a typed [`SpecParseError`] rather than
/// panicking on malformed input.
pub fn parse_contract_spec(bytes: &[u8]) -> Result<Vec<ScSpecEntry>, SpecParseError> {
    let mut cursor = Cursor::new(bytes);
    let mut entries = Vec::new();
    while cursor.remaining() > 0 {
        entries.push(cursor.read_entry()?);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Encoder {
        buf: Vec<u8>,
    }

    impl Encoder {
        fn new() -> Self {
            Self { buf: Vec::new() }
        }

        fn u32(&mut self, v: u32) -> &mut Self {
            self.buf.extend_from_slice(&v.to_be_bytes());
            self
        }

        fn i32(&mut self, v: i32) -> &mut Self {
            self.u32(v as u32)
        }

        fn string(&mut self, s: &str) -> &mut Self {
            self.u32(s.len() as u32);
            self.buf.extend_from_slice(s.as_bytes());
            let padding = (4 - (s.len() % 4)) % 4;
            self.buf.extend(std::iter::repeat(0u8).take(padding));
            self
        }

        fn type_scalar(&mut self, disc: i32) -> &mut Self {
            self.i32(disc)
        }

        fn finish(self) -> Vec<u8> {
            self.buf
        }
    }

    #[test]
    fn decodes_function_with_scalar_types() {
        let mut e = Encoder::new();
        e.i32(0); // SC_SPEC_ENTRY_FUNCTION_V0
        e.string(""); // doc
        e.string("transfer"); // name
        e.u32(2); // inputs count
        e.string("");
        e.string("to");
        e.type_scalar(19); // Address
        e.string("");
        e.string("amount");
        e.type_scalar(6); // U64
        e.u32(1); // outputs count
        e.type_scalar(1); // Bool
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            ScSpecEntry::FunctionV0(f) => {
                assert_eq!(f.name, "transfer");
                assert_eq!(f.inputs.len(), 2);
                assert_eq!(f.inputs[0].name, "to");
                assert_eq!(f.inputs[0].type_def, ScSpecTypeDef::Address);
                assert_eq!(f.inputs[1].type_def, ScSpecTypeDef::U64);
                assert_eq!(f.outputs, vec![ScSpecTypeDef::Bool]);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn decodes_nested_option_vec_type() {
        let mut e = Encoder::new();
        e.i32(0);
        e.string("");
        e.string("f");
        e.u32(1);
        e.string("");
        e.string("x");
        e.i32(1000); // Option
        e.i32(1002); // Vec
        e.type_scalar(4); // U32
        e.u32(0); // outputs count
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        match &entries[0] {
            ScSpecEntry::FunctionV0(f) => {
                assert_eq!(
                    f.inputs[0].type_def,
                    ScSpecTypeDef::Option(Box::new(ScSpecTypeDef::Vec(Box::new(
                        ScSpecTypeDef::U32
                    ))))
                );
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn decodes_struct_with_fields() {
        let mut e = Encoder::new();
        e.i32(1); // SC_SPEC_ENTRY_UDT_STRUCT_V0
        e.string("");
        e.string(""); // lib
        e.string("Position");
        e.u32(1); // fields count
        e.string("");
        e.string("amount");
        e.type_scalar(7); // I64
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        match &entries[0] {
            ScSpecEntry::UdtStructV0(s) => {
                assert_eq!(s.name, "Position");
                assert_eq!(s.fields.len(), 1);
                assert_eq!(s.fields[0].name, "amount");
                assert_eq!(s.fields[0].type_def, ScSpecTypeDef::I64);
            }
            other => panic!("expected struct, got {other:?}"),
        }
    }

    #[test]
    fn decodes_union_void_and_tuple_cases() {
        let mut e = Encoder::new();
        e.i32(2); // SC_SPEC_ENTRY_UDT_UNION_V0
        e.string("");
        e.string("");
        e.string("Status");
        e.u32(2); // cases
        e.i32(0); // void case
        e.string("");
        e.string("Idle");
        e.i32(1); // tuple case
        e.string("");
        e.string("Running");
        e.u32(1);
        e.type_scalar(4); // U32
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        match &entries[0] {
            ScSpecEntry::UdtUnionV0(u) => {
                assert_eq!(u.cases.len(), 2);
                assert_eq!(
                    u.cases[0],
                    ScSpecUdtUnionCaseV0::Void {
                        doc: "".into(),
                        name: "Idle".into()
                    }
                );
                assert_eq!(
                    u.cases[1],
                    ScSpecUdtUnionCaseV0::Tuple {
                        doc: "".into(),
                        name: "Running".into(),
                        types: vec![ScSpecTypeDef::U32]
                    }
                );
            }
            other => panic!("expected union, got {other:?}"),
        }
    }

    #[test]
    fn decodes_enum_and_error_enum() {
        let mut e = Encoder::new();
        e.i32(3); // SC_SPEC_ENTRY_UDT_ENUM_V0
        e.string("");
        e.string("");
        e.string("Kind");
        e.u32(1);
        e.string("");
        e.string("A");
        e.u32(0);

        e.i32(4); // SC_SPEC_ENTRY_UDT_ERROR_ENUM_V0
        e.string("");
        e.string("");
        e.string("Error");
        e.u32(1);
        e.string("");
        e.string("NotFound");
        e.u32(1);
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            ScSpecEntry::UdtEnumV0(en) => assert_eq!(en.cases[0].value, 0),
            other => panic!("expected enum, got {other:?}"),
        }
        match &entries[1] {
            ScSpecEntry::UdtErrorEnumV0(en) => {
                assert_eq!(en.cases[0].name, "NotFound");
                assert_eq!(en.cases[0].value, 1);
            }
            other => panic!("expected error enum, got {other:?}"),
        }
    }

    #[test]
    fn decodes_event() {
        let mut e = Encoder::new();
        e.i32(5); // SC_SPEC_ENTRY_EVENT_V0
        e.string("");
        e.string("");
        e.string("transfer");
        e.u32(1); // prefix_topics
        e.string("transfer");
        e.u32(1); // params
        e.string("");
        e.string("amount");
        e.type_scalar(6); // U64
        e.i32(0); // Data location
        e.i32(0); // SingleValue
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        match &entries[0] {
            ScSpecEntry::EventV0(ev) => {
                assert_eq!(ev.name, "transfer");
                assert_eq!(ev.prefix_topics, vec!["transfer".to_string()]);
                assert_eq!(ev.params[0].location, ScSpecEventParamLocationV0::Data);
                assert_eq!(ev.data_format, ScSpecEventDataFormat::SingleValue);
            }
            other => panic!("expected event, got {other:?}"),
        }
    }

    #[test]
    fn multiple_entries_back_to_back() {
        let mut e = Encoder::new();
        e.i32(0);
        e.string("");
        e.string("f1");
        e.u32(0);
        e.u32(0);
        e.i32(0);
        e.string("");
        e.string("f2");
        e.u32(0);
        e.u32(0);
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn rejects_truncated_input() {
        let bytes = vec![0u8, 0, 0]; // 3 bytes, not enough for a u32 discriminant
        let err = parse_contract_spec(&bytes).unwrap_err();
        assert!(matches!(err, SpecParseError::UnexpectedEof { .. }));
    }

    #[test]
    fn rejects_unknown_entry_kind() {
        let mut e = Encoder::new();
        e.i32(99);
        let bytes = e.finish();
        let err = parse_contract_spec(&bytes).unwrap_err();
        assert!(matches!(err, SpecParseError::InvalidDiscriminant { .. }));
    }

    #[test]
    fn empty_section_yields_no_entries() {
        let entries = parse_contract_spec(&[]).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn deeply_nested_option_type_is_rejected_not_stack_overflowed() {
        let mut e = Encoder::new();
        e.i32(0); // SC_SPEC_ENTRY_FUNCTION_V0
        e.string("");
        e.string("f");
        e.u32(1); // inputs count
        e.string("");
        e.string("x");
        // Nest far past MAX_TYPE_DEPTH consecutive Option wrappers with no
        // terminal scalar; this would recurse unboundedly without the guard.
        for _ in 0..(MAX_TYPE_DEPTH + 100) {
            e.i32(1000); // Option
        }
        let bytes = e.finish();

        let err = parse_contract_spec(&bytes).unwrap_err();
        assert!(matches!(err, SpecParseError::TypeNestingTooDeep { .. }));
    }

    #[test]
    fn moderately_nested_types_still_parse_successfully() {
        let mut e = Encoder::new();
        e.i32(0);
        e.string("");
        e.string("f");
        e.u32(1);
        e.string("");
        e.string("x");
        for _ in 0..5 {
            e.i32(1000); // Option
        }
        e.type_scalar(4); // terminal U32
        e.u32(0);
        let bytes = e.finish();

        let entries = parse_contract_spec(&bytes).unwrap();
        assert_eq!(entries.len(), 1);
    }
}
