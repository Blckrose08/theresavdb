use theresav_binder::BoundExpr;
use theresav_catalog::TableCatalog;
use theresav_parser::ast::{ColumnDef, OrderDirection};

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    Scan {
        table_catalog: TableCatalog,
    },
    Filter {
        predicate: BoundExpr,
        input: Box<LogicalPlan>,
    },
    Projection {
        exprs: Vec<(String, BoundExpr)>,
        input: Box<LogicalPlan>,
    },
    Sort {
        col_idx: usize,
        direction: OrderDirection,
        input: Box<LogicalPlan>,
    },
    Limit {
        limit: Option<usize>,
        offset: Option<usize>,
        input: Box<LogicalPlan>,
    },
    Insert {
        table_catalog: TableCatalog,
        column_indices: Vec<usize>,
        values: Vec<Vec<BoundExpr>>,
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
}
