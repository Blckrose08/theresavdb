//! Abstract Syntax Tree (AST) definitions for TheresavSQL.

use theresav_types::{DataType, Value};

/// Binary Operators in SQL expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Plus,
    Minus,
    Multiply,
    Divide,
    And,
    Or,
}

/// Unary Operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
}

/// SQL Expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    Column(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Wildcard,
}

/// Column definition in `CREATE TABLE`.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub is_primary_key: bool,
    pub nullable: bool,
}

/// Order by direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// SQL Statements supported by TheresavSQL.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    CreateTable {
        name: String,
        if_not_exists: bool,
        columns: Vec<ColumnDef>,
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
        table_name: String,
        columns: Option<Vec<String>>,
        values: Vec<Vec<Expr>>,
    },
    Select {
        distinct: bool,
        projection: Vec<Expr>,
        table_name: String,
        where_clause: Option<Expr>,
        order_by: Option<(String, OrderDirection)>,
        limit: Option<usize>,
        offset: Option<usize>,
    },
    Update {
        table_name: String,
        assignments: Vec<(String, Expr)>,
        where_clause: Option<Expr>,
    },
    Delete {
        table_name: String,
        where_clause: Option<Expr>,
    },
    Begin,
    Commit,
    Rollback,
}
