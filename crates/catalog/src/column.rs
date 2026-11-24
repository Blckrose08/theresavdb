use serde::{Deserialize, Serialize};
use theresav_types::DataType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnCatalog {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub ordinal_position: usize,
}

impl ColumnCatalog {
    pub fn new(
        name: impl Into<String>,
        data_type: DataType,
        nullable: bool,
        is_primary_key: bool,
        ordinal_position: usize,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable,
            is_primary_key,
            ordinal_position,
        }
    }
}
