use crate::column::ColumnCatalog;
use crate::table::{IndexCatalog, TableCatalog};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use theresav_common::Oid;
use theresav_error::CatalogError;

pub struct Catalog {
    next_oid: AtomicU64,
    tables: RwLock<HashMap<String, TableCatalog>>,
    indexes: RwLock<HashMap<String, IndexCatalog>>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    pub fn new() -> Self {
        Self {
            next_oid: AtomicU64::new(1000),
            tables: RwLock::new(HashMap::new()),
            indexes: RwLock::new(HashMap::new()),
        }
    }

    pub fn next_oid(&self) -> Oid {
        Oid::new(self.next_oid.fetch_add(1, Ordering::SeqCst))
    }

    pub fn create_table(
        &self,
        name: &str,
        columns: Vec<ColumnCatalog>,
    ) -> Result<Oid, CatalogError> {
        let mut tables = self.tables.write().unwrap();
        let lower_name = name.to_ascii_lowercase();

        if tables.contains_key(&lower_name) {
            return Err(CatalogError::TableAlreadyExists(name.to_string()));
        }

        let oid = self.next_oid();
        let table = TableCatalog::new(oid, name, columns);
        tables.insert(lower_name, table);

        Ok(oid)
    }

    pub fn drop_table(&self, name: &str) -> Result<(), CatalogError> {
        let mut tables = self.tables.write().unwrap();
        let lower_name = name.to_ascii_lowercase();

        if tables.remove(&lower_name).is_some() {
            let mut indexes = self.indexes.write().unwrap();
            indexes.retain(|_, idx| !idx.table_name.eq_ignore_ascii_case(name));
            Ok(())
        } else {
            Err(CatalogError::TableNotFound(name.to_string()))
        }
    }

    pub fn table_exists(&self, name: &str) -> bool {
        let tables = self.tables.read().unwrap();
        tables.contains_key(&name.to_ascii_lowercase())
    }

    pub fn get_table(&self, name: &str) -> Result<TableCatalog, CatalogError> {
        let tables = self.tables.read().unwrap();
        tables
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| CatalogError::TableNotFound(name.to_string()))
    }

    pub fn create_index(
        &self,
        name: &str,
        table_name: &str,
        columns: Vec<String>,
        is_unique: bool,
    ) -> Result<Oid, CatalogError> {
        if !self.table_exists(table_name) {
            return Err(CatalogError::TableNotFound(table_name.to_string()));
        }

        let mut indexes = self.indexes.write().unwrap();
        let lower_name = name.to_ascii_lowercase();

        if indexes.contains_key(&lower_name) {
            return Err(CatalogError::IndexNotFound(format!(
                "Index '{}' already exists",
                name
            )));
        }

        let oid = self.next_oid();
        let index = IndexCatalog::new(oid, name, table_name, columns, is_unique);
        indexes.insert(lower_name, index);

        Ok(oid)
    }

    pub fn get_index(&self, name: &str) -> Result<IndexCatalog, CatalogError> {
        let indexes = self.indexes.read().unwrap();
        indexes
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| CatalogError::IndexNotFound(name.to_string()))
    }

    pub fn list_tables(&self) -> Vec<String> {
        let tables = self.tables.read().unwrap();
        tables.values().map(|t| t.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theresav_types::DataType;

    #[test]
    fn test_catalog_table_and_index_lifecycle() {
        let catalog = Catalog::new();

        let cols = vec![
            ColumnCatalog::new("id", DataType::BigInt, false, true, 0),
            ColumnCatalog::new("name", DataType::Text, false, false, 1),
            ColumnCatalog::new("email", DataType::Varchar, true, false, 2),
        ];

        let table_oid = catalog.create_table("users", cols).unwrap();
        assert!(catalog.table_exists("users"));
        assert!(catalog.table_exists("USERS"));

        let table = catalog.get_table("users").unwrap();
        assert_eq!(table.oid, table_oid);
        assert_eq!(table.primary_key, Some("id".to_string()));
        assert_eq!(table.column_index("email"), Some(2));

        let idx_oid = catalog
            .create_index("idx_users_email", "users", vec!["email".to_string()], true)
            .unwrap();
        assert!(idx_oid.is_valid());

        catalog.drop_table("users").unwrap();
        assert!(!catalog.table_exists("users"));
    }
}
