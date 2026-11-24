use byteorder::{ByteOrder, LittleEndian};
use theresav_common::TxId;
use theresav_error::PageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tuple {
    pub xmin: TxId,
    pub xmax: TxId,
    pub flags: u16,
    pub schema_version: u32,
    pub null_bitmap: Vec<u8>,
    pub payload: Vec<u8>,
}

pub const TUPLE_HEADER_MIN_SIZE: usize = 8 + 8 + 2 + 4 + 2;

impl Tuple {
    pub fn new(
        xmin: TxId,
        xmax: TxId,
        flags: u16,
        schema_version: u32,
        null_bitmap: Vec<u8>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            xmin,
            xmax,
            flags,
            schema_version,
            null_bitmap,
            payload,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let total_size = TUPLE_HEADER_MIN_SIZE + self.null_bitmap.len() + self.payload.len();
        let mut out = Vec::with_capacity(total_size);

        out.extend_from_slice(&self.xmin.0.to_le_bytes());
        out.extend_from_slice(&self.xmax.0.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.schema_version.to_le_bytes());
        out.extend_from_slice(&(self.null_bitmap.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.null_bitmap);
        out.extend_from_slice(&self.payload);

        out
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, PageError> {
        if bytes.len() < TUPLE_HEADER_MIN_SIZE {
            return Err(PageError::TupleCorrupted(format!(
                "Tuple buffer too small: {} bytes (minimum required {})",
                bytes.len(),
                TUPLE_HEADER_MIN_SIZE
            )));
        }

        let xmin = TxId::new(LittleEndian::read_u64(&bytes[0..8]));
        let xmax = TxId::new(LittleEndian::read_u64(&bytes[8..16]));
        let flags = LittleEndian::read_u16(&bytes[16..18]);
        let schema_version = LittleEndian::read_u32(&bytes[18..22]);
        let null_bitmap_len = LittleEndian::read_u16(&bytes[22..24]) as usize;

        let bitmap_end = TUPLE_HEADER_MIN_SIZE + null_bitmap_len;
        if bytes.len() < bitmap_end {
            return Err(PageError::TupleCorrupted(
                "Truncated null bitmap in tuple".into(),
            ));
        }

        let null_bitmap = bytes[TUPLE_HEADER_MIN_SIZE..bitmap_end].to_vec();
        let payload = bytes[bitmap_end..].to_vec();

        Ok(Self {
            xmin,
            xmax,
            flags,
            schema_version,
            null_bitmap,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuple_serialization_roundtrip() {
        let tuple = Tuple::new(
            TxId::new(100),
            TxId::new(105),
            0x0001,
            2,
            vec![0b00000101, 0b00000000],
            b"Column data encoded here".to_vec(),
        );

        let bytes = tuple.serialize();
        let decoded = Tuple::deserialize(&bytes).expect("Tuple deserialization must succeed");

        assert_eq!(tuple, decoded);
    }

    #[test]
    fn test_corrupted_tuple_detection() {
        let too_short = vec![0u8; 10];
        assert!(matches!(
            Tuple::deserialize(&too_short),
            Err(PageError::TupleCorrupted(..))
        ));
    }
}
