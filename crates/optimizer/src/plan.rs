use theresav_binder::BoundExpr;
use theresav_catalog::TableCatalog;
use theresav_parser::ast::{ColumnDef, OrderDirection};

#[derive(Debug, Clone, PartialEq)]
pub enum PhysicalPlan {
    SeqScan {
        table_catalog: TableCatalog,
    },
    IndexScan {
        table_catalog: TableCatalog,
        index_name: String,
        search_key: BoundExpr,
    },
    Filter {
        predicate: BoundExpr,
        input: Box<PhysicalPlan>,
    },
    Projection {
        exprs: Vec<(String, BoundExpr)>,
        input: Box<PhysicalPlan>,
    },
    Sort {
        col_idx: usize,
        direction: OrderDirection,
        input: Box<PhysicalPlan>,
    },
    Limit {
        limit: Option<usize>,
        offset: Option<usize>,
        input: Box<PhysicalPlan>,
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
