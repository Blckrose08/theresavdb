//! Index keys supporting single and composite column values with strict ordering.

use std::cmp::Ordering;
use theresav_error::TypeError;
use theresav_types::{DataType, Value};

/// Composite or single-column index key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexKey(pub Vec<Value>);

impl IndexKey {
    pub fn single(val: Value) -> Self {
        Self(vec![val])
    }

    pub fn composite(vals: Vec<Value>) -> Self {
        Self(vals)
    }

    /// Serialize composite key into binary bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.0.len() as u8);
        for val in &self.0 {
            if let Some(dt) = val.data_type() {
                out.push(dt.to_u8());
                val.serialize(&mut out);
            } else {
                out.push(0); // Null marker
            }
        }
        out
    }

    /// Deserialize composite key from binary bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<(Self, usize), TypeError> {
        if bytes.is_empty() {
            return Err(TypeError::InvalidEncoding(
                "Empty buffer for IndexKey".into(),
            ));
        }
        let count = bytes[0] as usize;
        let mut offset = 1;
        let mut values = Vec::with_capacity(count);

        for _ in 0..count {
            if offset >= bytes.len() {
                return Err(TypeError::InvalidEncoding(
                    "Truncated IndexKey bytes".into(),
                ));
            }
            let type_byte = bytes[offset];
            offset += 1;
            if type_byte == 0 {
                values.push(Value::Null);
            } else {
                let dt = DataType::from_u8(type_byte)?;
                let (val, read) = Value::deserialize(dt, &bytes[offset..])?;
                offset += read;
                values.push(val);
            }
        }

        Ok((Self(values), offset))
    }
}

impl PartialOrd for IndexKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexKey {
    fn cmp(&self, other: &Self) -> Ordering {
        let min_len = self.0.len().min(other.0.len());
        for i in 0..min_len {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                non_eq => return non_eq,
            }
        }
        self.0.len().cmp(&other.0.len())
    }
}
