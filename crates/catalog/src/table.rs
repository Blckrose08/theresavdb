use crate::column::ColumnCatalog;
use serde::{Deserialize, Serialize};
use theresav_common::Oid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCatalog {
    pub oid: Oid,
    pub name: String,
    pub columns: Vec<ColumnCatalog>,
    pub primary_key: Option<String>,
}

impl TableCatalog {
    pub fn new(oid: Oid, name: impl Into<String>, columns: Vec<ColumnCatalog>) -> Self {
        let name = name.into();
        let primary_key = columns
            .iter()
            .find(|c| c.is_primary_key)
            .map(|c| c.name.clone());

        Self {
            oid,
            name,
            columns,
            primary_key,
        }
    }

    pub fn find_column(&self, col_name: &str) -> Option<&ColumnCatalog> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(col_name))
    }

    pub fn column_index(&self, col_name: &str) -> Option<usize> {
        self.columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(col_name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexCatalog {
    pub oid: Oid,
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

impl IndexCatalog {
    pub fn new(
        oid: Oid,
        name: impl Into<String>,
        table_name: impl Into<String>,
        columns: Vec<String>,
        is_unique: bool,
    ) -> Self {
        Self {
            oid,
            name: name.into(),
            table_name: table_name.into(),
            columns,
            is_unique,
        }
    }
}
