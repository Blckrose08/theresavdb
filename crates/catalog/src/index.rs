use serde::{Deserialize, Serialize};
use theresav_common::Oid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexCatalog {
    pub index_oid: Oid,
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

impl IndexCatalog {
    pub fn new(
        index_oid: Oid,
        index_name: impl Into<String>,
        table_name: impl Into<String>,
        columns: Vec<String>,
        is_unique: bool,
    ) -> Self {
        Self {
            index_oid,
            index_name: index_name.into(),
            table_name: table_name.into(),
            columns,
            is_unique,
        }
    }
}
