//! Binary JSON (JSONB) implementation for TheresavDB.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use theresav_error::TypeError;

/// JSONB values decomposed into a validated internal representation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JsonbValue {
    Null,
    Bool(bool),
    Number(rust_decimal::Decimal),
    String(String),
    Array(Vec<JsonbValue>),
    Object(BTreeMap<String, JsonbValue>),
}

const JSONB_TAG_NULL: u8 = 0;
const JSONB_TAG_FALSE: u8 = 1;
const JSONB_TAG_TRUE: u8 = 2;
const JSONB_TAG_NUMBER: u8 = 3;
const JSONB_TAG_STRING: u8 = 4;
const JSONB_TAG_ARRAY: u8 = 5;
const JSONB_TAG_OBJECT: u8 = 6;

impl JsonbValue {
    /// Parse from valid JSON text.
    pub fn parse_str(json_str: &str) -> Result<Self, TypeError> {
        let serde_val: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| TypeError::InvalidJson(e.to_string()))?;
        Self::from_serde_value(serde_val)
    }

    pub fn from_serde_value(val: serde_json::Value) -> Result<Self, TypeError> {
        match val {
            serde_json::Value::Null => Ok(Self::Null),
            serde_json::Value::Bool(b) => Ok(Self::Bool(b)),
            serde_json::Value::Number(num) => {
                let dec = rust_decimal::Decimal::from_str_exact(&num.to_string())
                    .map_err(|e| TypeError::InvalidJson(format!("Invalid number: {}", e)))?;
                Ok(Self::Number(dec))
            }
            serde_json::Value::String(s) => Ok(Self::String(s)),
            serde_json::Value::Array(arr) => {
                let items: Result<Vec<_>, _> =
                    arr.into_iter().map(Self::from_serde_value).collect();
                Ok(Self::Array(items?))
            }
            serde_json::Value::Object(map) => {
                let mut btree = BTreeMap::new();
                for (k, v) in map {
                    btree.insert(k, Self::from_serde_value(v)?);
                }
                Ok(Self::Object(btree))
            }
        }
    }

    /// Explicit binary serialization of JSONB.
    pub fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::Null => out.push(JSONB_TAG_NULL),
            Self::Bool(false) => out.push(JSONB_TAG_FALSE),
            Self::Bool(true) => out.push(JSONB_TAG_TRUE),
            Self::Number(dec) => {
                out.push(JSONB_TAG_NUMBER);
                out.extend_from_slice(&dec.serialize());
            }
            Self::String(s) => {
                out.push(JSONB_TAG_STRING);
                let bytes = s.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Array(arr) => {
                out.push(JSONB_TAG_ARRAY);
                out.extend_from_slice(&(arr.len() as u32).to_le_bytes());
                for item in arr {
                    item.encode(out);
                }
            }
            Self::Object(map) => {
                out.push(JSONB_TAG_OBJECT);
                out.extend_from_slice(&(map.len() as u32).to_le_bytes());
                for (k, v) in map {
                    let k_bytes = k.as_bytes();
                    out.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(k_bytes);
                    v.encode(out);
                }
            }
        }
    }

    /// Binary deserialization of JSONB.
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), TypeError> {
        if bytes.is_empty() {
            return Err(TypeError::InvalidEncoding("Empty buffer for JSONB".into()));
        }
        let tag = bytes[0];
        let mut offset = 1;

        match tag {
            JSONB_TAG_NULL => Ok((Self::Null, 1)),
            JSONB_TAG_FALSE => Ok((Self::Bool(false), 1)),
            JSONB_TAG_TRUE => Ok((Self::Bool(true), 1)),
            JSONB_TAG_NUMBER => {
                if bytes.len() < offset + 16 {
                    return Err(TypeError::InvalidEncoding("Truncated JSONB number".into()));
                }
                let mut num_bytes = [0u8; 16];
                num_bytes.copy_from_slice(&bytes[offset..offset + 16]);
                let dec = rust_decimal::Decimal::deserialize(num_bytes);
                offset += 16;
                Ok((Self::Number(dec), offset))
            }
            JSONB_TAG_STRING => {
                if bytes.len() < offset + 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB string length".into(),
                    ));
                }
                let len = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as usize;
                offset += 4;
                if bytes.len() < offset + len {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB string payload".into(),
                    ));
                }
                let s = std::str::from_utf8(&bytes[offset..offset + len])
                    .map_err(|e| {
                        TypeError::InvalidEncoding(format!("Invalid UTF-8 in JSONB: {}", e))
                    })?
                    .to_string();
                offset += len;
                Ok((Self::String(s), offset))
            }
            JSONB_TAG_ARRAY => {
                if bytes.len() < offset + 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB array length".into(),
                    ));
                }
                let len = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as usize;
                offset += 4;
                let mut arr = Vec::with_capacity(len);
                for _ in 0..len {
                    let (item, read) = Self::decode(&bytes[offset..])?;
                    offset += read;
                    arr.push(item);
                }
                Ok((Self::Array(arr), offset))
            }
            JSONB_TAG_OBJECT => {
                if bytes.len() < offset + 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB object length".into(),
                    ));
                }
                let len = u32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]) as usize;
                offset += 4;
                let mut map = BTreeMap::new();
                for _ in 0..len {
                    if bytes.len() < offset + 4 {
                        return Err(TypeError::InvalidEncoding(
                            "Truncated JSONB key length".into(),
                        ));
                    }
                    let k_len = u32::from_le_bytes([
                        bytes[offset],
                        bytes[offset + 1],
                        bytes[offset + 2],
                        bytes[offset + 3],
                    ]) as usize;
                    offset += 4;
                    if bytes.len() < offset + k_len {
                        return Err(TypeError::InvalidEncoding(
                            "Truncated JSONB key string".into(),
                        ));
                    }
                    let key = std::str::from_utf8(&bytes[offset..offset + k_len])
                        .map_err(|e| {
                            TypeError::InvalidEncoding(format!("Invalid UTF-8 in key: {}", e))
                        })?
                        .to_string();
                    offset += k_len;
                    let (val, read) = Self::decode(&bytes[offset..])?;
                    offset += read;
                    map.insert(key, val);
                }
                Ok((Self::Object(map), offset))
            }
            _ => Err(TypeError::InvalidEncoding(format!(
                "Unknown JSONB tag: {}",
                tag
            ))),
        }
    }
}

impl fmt::Display for JsonbValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Number(n) => write!(f, "{}", n),
            Self::String(s) => write!(f, "\"{}\"", s),
            Self::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Self::Object(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}
