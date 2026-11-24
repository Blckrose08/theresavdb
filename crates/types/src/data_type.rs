//! Data types supported by TheresavDB.

use serde::{Deserialize, Serialize};
use std::fmt;
use theresav_error::TypeError;

/// Logical data type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DataType {
    Boolean = 1,
    SmallInt = 2,
    Integer = 3,
    BigInt = 4,
    Real = 5,
    Double = 6,
    Decimal = 7,
    Text = 8,
    Varchar = 9,
    Bytea = 10,
    Uuid = 11,
    Timestamp = 12,
    Date = 13,
    Json = 14,
    Jsonb = 15,
}

impl DataType {
    pub fn from_u8(val: u8) -> Result<Self, TypeError> {
        match val {
            1 => Ok(Self::Boolean),
            2 => Ok(Self::SmallInt),
            3 => Ok(Self::Integer),
            4 => Ok(Self::BigInt),
            5 => Ok(Self::Real),
            6 => Ok(Self::Double),
            7 => Ok(Self::Decimal),
            8 => Ok(Self::Text),
            9 => Ok(Self::Varchar),
            10 => Ok(Self::Bytea),
            11 => Ok(Self::Uuid),
            12 => Ok(Self::Timestamp),
            13 => Ok(Self::Date),
            14 => Ok(Self::Json),
            15 => Ok(Self::Jsonb),
            _ => Err(TypeError::InvalidEncoding(format!(
                "Unknown DataType byte: {}",
                val
            ))),
        }
    }

    #[inline]
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Whether this type is fixed-length in its binary serialization.
    pub const fn fixed_size(&self) -> Option<usize> {
        match self {
            Self::Boolean => Some(1),
            Self::SmallInt => Some(2),
            Self::Integer => Some(4),
            Self::BigInt => Some(8),
            Self::Real => Some(4),
            Self::Double => Some(8),
            Self::Decimal => Some(16), // Fixed 16-byte layout (128-bit exact decimal)
            Self::Uuid => Some(16),
            Self::Timestamp => Some(8), // i64 microseconds since epoch
            Self::Date => Some(4),      // i32 days since epoch
            Self::Text | Self::Varchar | Self::Bytea | Self::Json | Self::Jsonb => None,
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boolean => write!(f, "BOOLEAN"),
            Self::SmallInt => write!(f, "SMALLINT"),
            Self::Integer => write!(f, "INTEGER"),
            Self::BigInt => write!(f, "BIGINT"),
            Self::Real => write!(f, "REAL"),
            Self::Double => write!(f, "DOUBLE PRECISION"),
            Self::Decimal => write!(f, "DECIMAL"),
            Self::Text => write!(f, "TEXT"),
            Self::Varchar => write!(f, "VARCHAR"),
            Self::Bytea => write!(f, "BYTEA"),
            Self::Uuid => write!(f, "UUID"),
            Self::Timestamp => write!(f, "TIMESTAMP"),
            Self::Date => write!(f, "DATE"),
            Self::Json => write!(f, "JSON"),
            Self::Jsonb => write!(f, "JSONB"),
        }
    }
}
