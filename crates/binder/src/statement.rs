//! Bound SQL statements.

use crate::expr::BoundExpr;
use theresav_catalog::TableCatalog;
use theresav_parser::ast::{ColumnDef, OrderDirection};

#[derive(Debug, Clone, PartialEq)]
pub enum BoundStatement {
    CreateTable {
        name: String,
        columns: Vec<ColumnDef>,
        if_not_exists: bool,
    },
    DropTable {
        name: String,
        if_exists: bool,
    },
    CreateIndex {
        name: String,
        table_name: String,
        columns: Vec<String>,
        is_unique: bool,
    },
    DropIndex {
        name: String,
    },
    Insert {
        table_catalog: TableCatalog,
        column_indices: Vec<usize>,
        values: Vec<Vec<BoundExpr>>,
    },
    Select {
        table_catalog: TableCatalog,
        projection: Vec<(String, BoundExpr)>,
        where_clause: Option<BoundExpr>,
        order_by: Option<(usize, OrderDirection)>,
        limit: Option<usize>,
        offset: Option<usize>,
    },
    Update {
        table_catalog: TableCatalog,
        assignments: Vec<(usize, BoundExpr)>,
        where_clause: Option<BoundExpr>,
    },
    Delete {
        table_catalog: TableCatalog,
        where_clause: Option<BoundExpr>,
    },
    Begin,
    Commit,
    Rollback,
}
