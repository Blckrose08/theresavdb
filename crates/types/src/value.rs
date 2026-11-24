//! Value representation and binary serialization for all database types.

use crate::data_type::DataType;
use crate::jsonb::JsonbValue;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use theresav_error::TypeError;
use uuid::Uuid;

/// Typed runtime value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Boolean(bool),
    SmallInt(i16),
    Integer(i32),
    BigInt(i64),
    Real(f32),
    Double(f64),
    Decimal(Decimal),
    Text(String),
    Varchar(String),
    Bytea(Vec<u8>),
    Uuid(Uuid),
    Timestamp(NaiveDateTime),
    Date(NaiveDate),
    Json(String),
    Jsonb(JsonbValue),
}

impl Value {
    /// Return the corresponding DataType.
    pub fn data_type(&self) -> Option<DataType> {
        match self {
            Self::Null => None,
            Self::Boolean(_) => Some(DataType::Boolean),
            Self::SmallInt(_) => Some(DataType::SmallInt),
            Self::Integer(_) => Some(DataType::Integer),
            Self::BigInt(_) => Some(DataType::BigInt),
            Self::Real(_) => Some(DataType::Real),
            Self::Double(_) => Some(DataType::Double),
            Self::Decimal(_) => Some(DataType::Decimal),
            Self::Text(_) => Some(DataType::Text),
            Self::Varchar(_) => Some(DataType::Varchar),
            Self::Bytea(_) => Some(DataType::Bytea),
            Self::Uuid(_) => Some(DataType::Uuid),
            Self::Timestamp(_) => Some(DataType::Timestamp),
            Self::Date(_) => Some(DataType::Date),
            Self::Json(_) => Some(DataType::Json),
            Self::Jsonb(_) => Some(DataType::Jsonb),
        }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Explicit binary serialization of a Value (little-endian, bounds checked).
    pub fn serialize(&self, out: &mut Vec<u8>) {
        match self {
            Self::Null => {} // Null is tracked in the tuple null bitmap
            Self::Boolean(b) => out.push(if *b { 1 } else { 0 }),
            Self::SmallInt(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::Integer(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::BigInt(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::Real(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::Double(v) => out.extend_from_slice(&v.to_le_bytes()),
            Self::Decimal(d) => out.extend_from_slice(&d.serialize()),
            Self::Text(s) | Self::Varchar(s) => {
                let bytes = s.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Bytea(b) => {
                out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                out.extend_from_slice(b);
            }
            Self::Uuid(u) => out.extend_from_slice(u.as_bytes()),
            Self::Timestamp(dt) => {
                let micros = dt.and_utc().timestamp_micros();
                out.extend_from_slice(&micros.to_le_bytes());
            }
            Self::Date(d) => {
                let days = d.num_days_from_ce();
                out.extend_from_slice(&days.to_le_bytes());
            }
            Self::Json(s) => {
                let bytes = s.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Jsonb(jb) => {
                let mut jb_buf = Vec::new();
                jb.encode(&mut jb_buf);
                out.extend_from_slice(&(jb_buf.len() as u32).to_le_bytes());
                out.extend_from_slice(&jb_buf);
            }
        }
    }

    /// Explicit binary deserialization of a Value from disk bytes.
    pub fn deserialize(data_type: DataType, bytes: &[u8]) -> Result<(Self, usize), TypeError> {
        match data_type {
            DataType::Boolean => {
                if bytes.is_empty() {
                    return Err(TypeError::InvalidEncoding(
                        "Missing byte for boolean".into(),
                    ));
                }
                Ok((Self::Boolean(bytes[0] != 0), 1))
            }
            DataType::SmallInt => {
                if bytes.len() < 2 {
                    return Err(TypeError::InvalidEncoding("Truncated SmallInt".into()));
                }
                let val = i16::from_le_bytes([bytes[0], bytes[1]]);
                Ok((Self::SmallInt(val), 2))
            }
            DataType::Integer => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding("Truncated Integer".into()));
                }
                let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok((Self::Integer(val), 4))
            }
            DataType::BigInt => {
                if bytes.len() < 8 {
                    return Err(TypeError::InvalidEncoding("Truncated BigInt".into()));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[0..8]);
                let val = i64::from_le_bytes(buf);
                Ok((Self::BigInt(val), 8))
            }
            DataType::Real => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding("Truncated Real".into()));
                }
                let val = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                Ok((Self::Real(val), 4))
            }
            DataType::Double => {
                if bytes.len() < 8 {
                    return Err(TypeError::InvalidEncoding("Truncated Double".into()));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[0..8]);
                let val = f64::from_le_bytes(buf);
                Ok((Self::Double(val), 8))
            }
            DataType::Decimal => {
                if bytes.len() < 16 {
                    return Err(TypeError::InvalidEncoding("Truncated Decimal".into()));
                }
                let mut buf = [0u8; 16];
                buf.copy_from_slice(&bytes[0..16]);
                let dec = Decimal::deserialize(buf);
                Ok((Self::Decimal(dec), 16))
            }
            DataType::Text | DataType::Varchar => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated string length prefix".into(),
                    ));
                }
                let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                if bytes.len() < 4 + len {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated string payload".into(),
                    ));
                }
                let s = std::str::from_utf8(&bytes[4..4 + len])
                    .map_err(|e| TypeError::InvalidEncoding(format!("Invalid UTF-8: {}", e)))?
                    .to_string();
                let val = if data_type == DataType::Text {
                    Self::Text(s)
                } else {
                    Self::Varchar(s)
                };
                Ok((val, 4 + len))
            }
            DataType::Bytea => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated bytea length prefix".into(),
                    ));
                }
                let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                if bytes.len() < 4 + len {
                    return Err(TypeError::InvalidEncoding("Truncated bytea payload".into()));
                }
                let b = bytes[4..4 + len].to_vec();
                Ok((Self::Bytea(b), 4 + len))
            }
            DataType::Uuid => {
                if bytes.len() < 16 {
                    return Err(TypeError::InvalidEncoding("Truncated UUID".into()));
                }
                let u = Uuid::from_slice(&bytes[0..16])
                    .map_err(|e| TypeError::InvalidEncoding(format!("Invalid UUID: {}", e)))?;
                Ok((Self::Uuid(u), 16))
            }
            DataType::Timestamp => {
                if bytes.len() < 8 {
                    return Err(TypeError::InvalidEncoding("Truncated Timestamp".into()));
                }
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[0..8]);
                let micros = i64::from_le_bytes(buf);
                let dt = DateTime::from_timestamp_micros(micros)
                    .ok_or_else(|| TypeError::InvalidEncoding("Timestamp out of range".into()))?
                    .naive_utc();
                Ok((Self::Timestamp(dt), 8))
            }
            DataType::Date => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding("Truncated Date".into()));
                }
                let days = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let d = NaiveDate::from_num_days_from_ce_opt(days)
                    .ok_or_else(|| TypeError::InvalidEncoding("Date out of range".into()))?;
                Ok((Self::Date(d), 4))
            }
            DataType::Json => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSON length prefix".into(),
                    ));
                }
                let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                if bytes.len() < 4 + len {
                    return Err(TypeError::InvalidEncoding("Truncated JSON payload".into()));
                }
                let s = std::str::from_utf8(&bytes[4..4 + len])
                    .map_err(|e| {
                        TypeError::InvalidEncoding(format!("Invalid UTF-8 in JSON: {}", e))
                    })?
                    .to_string();
                serde_json::from_str::<serde_json::Value>(&s)
                    .map_err(|e| TypeError::InvalidJson(e.to_string()))?;
                Ok((Self::Json(s), 4 + len))
            }
            DataType::Jsonb => {
                if bytes.len() < 4 {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB container length".into(),
                    ));
                }
                let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
                if bytes.len() < 4 + len {
                    return Err(TypeError::InvalidEncoding(
                        "Truncated JSONB container payload".into(),
                    ));
                }
                let (jb, _) = JsonbValue::decode(&bytes[4..4 + len])?;
                Ok((Self::Jsonb(jb), 4 + len))
            }
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Null, Self::Null) => Ordering::Equal,
            (Self::Null, _) => Ordering::Less,
            (_, Self::Null) => Ordering::Greater,
            (Self::Boolean(a), Self::Boolean(b)) => a.cmp(b),
            (Self::SmallInt(a), Self::SmallInt(b)) => a.cmp(b),
            (Self::Integer(a), Self::Integer(b)) => a.cmp(b),
            (Self::BigInt(a), Self::BigInt(b)) => a.cmp(b),
            (Self::Real(a), Self::Real(b)) => a.total_cmp(b),
            (Self::Double(a), Self::Double(b)) => a.total_cmp(b),
            (Self::Decimal(a), Self::Decimal(b)) => a.cmp(b),
            (Self::Text(a), Self::Text(b)) | (Self::Varchar(a), Self::Varchar(b)) => a.cmp(b),
            (Self::Bytea(a), Self::Bytea(b)) => a.cmp(b),
            (Self::Uuid(a), Self::Uuid(b)) => a.cmp(b),
            (Self::Timestamp(a), Self::Timestamp(b)) => a.cmp(b),
            (Self::Date(a), Self::Date(b)) => a.cmp(b),
            (Self::Json(a), Self::Json(b)) => a.cmp(b),
            (Self::Jsonb(a), Self::Jsonb(b)) => a.cmp(b),
            _ => Ordering::Equal, // cross-type comparisons handled via coercion in planner
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "NULL"),
            Self::Boolean(b) => write!(f, "{}", b),
            Self::SmallInt(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::BigInt(v) => write!(f, "{}", v),
            Self::Real(v) => write!(f, "{}", v),
            Self::Double(v) => write!(f, "{}", v),
            Self::Decimal(v) => write!(f, "{}", v),
            Self::Text(v) | Self::Varchar(v) => write!(f, "{}", v),
            Self::Bytea(v) => write!(f, "\\x{}", hex_encode(v)),
            Self::Uuid(v) => write!(f, "{}", v),
            Self::Timestamp(v) => write!(f, "{}", v),
            Self::Date(v) => write!(f, "{}", v),
            Self::Json(v) => write!(f, "{}", v),
            Self::Jsonb(v) => write!(f, "{}", v),
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_types_serialization_roundtrip() {
        let values = vec![
            Value::Boolean(true),
            Value::Boolean(false),
            Value::SmallInt(-1234),
            Value::Integer(987654321),
            Value::BigInt(-987654321012345678),
            Value::Real(123.456),
            Value::Double(-789.101112),
            Value::Decimal(Decimal::from_str("123456.7890123").unwrap()),
            Value::Text("Hello, TheresavDB!".to_string()),
            Value::Varchar("Varchar test".to_string()),
            Value::Bytea(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            Value::Uuid(Uuid::new_v4()),
            Value::Date(NaiveDate::from_ymd_opt(2026, 11, 24).unwrap()),
            Value::Timestamp(
                NaiveDate::from_ymd_opt(2026, 11, 24)
                    .unwrap()
                    .and_hms_opt(12, 0, 0)
                    .unwrap(),
            ),
            Value::Json("{\"theresav\": true, \"v\": 1}".to_string()),
            Value::Jsonb(
                JsonbValue::parse_str("{\"users\": [\"alice\", \"bob\"], \"active\": true}")
                    .unwrap(),
            ),
        ];

        for val in values {
            let dt = val.data_type().unwrap();
            let mut buf = Vec::new();
            val.serialize(&mut buf);

            let (decoded, read) = Value::deserialize(dt, &buf).unwrap();
            assert_eq!(read, buf.len());
            assert_eq!(val, decoded);
        }
    }
}
