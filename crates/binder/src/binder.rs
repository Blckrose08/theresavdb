use crate::expr::BoundExpr;
use crate::statement::BoundStatement;
use theresav_catalog::{Catalog, TableCatalog};
use theresav_error::CatalogError;
use theresav_parser::ast::{BinaryOp, Expr, Statement};
use theresav_types::DataType;

pub struct Binder<'a> {
    catalog: &'a Catalog,
}

impl<'a> Binder<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self { catalog }
    }

    #[inline]
    pub fn bind(&self, stmt: Statement) -> Result<BoundStatement, CatalogError> {
        self.bind_statement(stmt)
    }

    pub fn bind_statement(&self, stmt: Statement) -> Result<BoundStatement, CatalogError> {
        match stmt {
            Statement::CreateTable {
                name,
                if_not_exists,
                columns,
            } => Ok(BoundStatement::CreateTable {
                name,
                if_not_exists,
                columns,
            }),
            Statement::DropTable { name, if_exists } => {
                Ok(BoundStatement::DropTable { name, if_exists })
            }
            Statement::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            } => Ok(BoundStatement::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            }),
            Statement::DropIndex { name } => Ok(BoundStatement::DropIndex { name }),
            Statement::Insert {
                table_name,
                columns,
                values,
            } => {
                let table_catalog = self.catalog.get_table(&table_name)?;
                let col_indices = match columns {
                    Some(names) => {
                        let mut idxs = Vec::new();
                        for name in names {
                            let idx = table_catalog.column_index(&name).ok_or_else(|| {
                                CatalogError::ColumnNotFound(format!(
                                    "Column '{}' not in table '{}'",
                                    name, table_name
                                ))
                            })?;
                            idxs.push(idx);
                        }
                        idxs
                    }
                    None => (0..table_catalog.columns.len()).collect(),
                };

                let mut bound_values = Vec::new();
                for row in values {
                    let mut bound_row = Vec::new();
                    for expr in row {
                        bound_row.push(self.bind_expr(&expr, &table_catalog)?);
                    }
                    bound_values.push(bound_row);
                }

                Ok(BoundStatement::Insert {
                    table_catalog,
                    column_indices: col_indices,
                    values: bound_values,
                })
            }
            Statement::Select {
                projection,
                table_name,
                where_clause,
                order_by,
                limit,
                offset,
                ..
            } => {
                let table_catalog = self.catalog.get_table(&table_name)?;

                let mut bound_proj = Vec::new();
                for expr in projection {
                    if expr == Expr::Wildcard {
                        for (idx, col) in table_catalog.columns.iter().enumerate() {
                            bound_proj.push((
                                col.name.clone(),
                                BoundExpr::ColumnRef {
                                    col_name: col.name.clone(),
                                    col_idx: idx,
                                    data_type: col.data_type,
                                },
                            ));
                        }
                    } else {
                        let bound = self.bind_expr(&expr, &table_catalog)?;
                        let col_name = match &bound {
                            BoundExpr::ColumnRef { col_name, .. } => col_name.clone(),
                            _ => format!("col_{}", bound_proj.len()),
                        };
                        bound_proj.push((col_name, bound));
                    }
                }

                let bound_where = where_clause
                    .map(|w| self.bind_expr(&w, &table_catalog))
                    .transpose()?;

                let bound_order = match order_by {
                    Some((col, dir)) => {
                        let idx = table_catalog.column_index(&col).ok_or_else(|| {
                            CatalogError::ColumnNotFound(format!(
                                "Column '{}' in ORDER BY not found",
                                col
                            ))
                        })?;
                        Some((idx, dir))
                    }
                    None => None,
                };

                Ok(BoundStatement::Select {
                    table_catalog,
                    projection: bound_proj,
                    where_clause: bound_where,
                    order_by: bound_order,
                    limit,
                    offset,
                })
            }
            Statement::Update {
                table_name,
                assignments,
                where_clause,
            } => {
                let table_catalog = self.catalog.get_table(&table_name)?;
                let mut bound_assigns = Vec::new();
                for (col, expr) in assignments {
                    let idx = table_catalog.column_index(&col).ok_or_else(|| {
                        CatalogError::ColumnNotFound(format!("Column '{}' not found", col))
                    })?;
                    let bound = self.bind_expr(&expr, &table_catalog)?;
                    bound_assigns.push((idx, bound));
                }

                let bound_where = where_clause
                    .map(|w| self.bind_expr(&w, &table_catalog))
                    .transpose()?;

                Ok(BoundStatement::Update {
                    table_catalog,
                    assignments: bound_assigns,
                    where_clause: bound_where,
                })
            }
            Statement::Delete {
                table_name,
                where_clause,
            } => {
                let table_catalog = self.catalog.get_table(&table_name)?;
                let bound_where = where_clause
                    .map(|w| self.bind_expr(&w, &table_catalog))
                    .transpose()?;

                Ok(BoundStatement::Delete {
                    table_catalog,
                    where_clause: bound_where,
                })
            }
            Statement::Begin => Ok(BoundStatement::Begin),
            Statement::Commit => Ok(BoundStatement::Commit),
            Statement::Rollback => Ok(BoundStatement::Rollback),
        }
    }

    fn bind_expr(&self, expr: &Expr, table: &TableCatalog) -> Result<BoundExpr, CatalogError> {
        match expr {
            Expr::Literal(val) => Ok(BoundExpr::Literal(val.clone())),
            Expr::Column(name) => {
                let idx = table.column_index(name).ok_or_else(|| {
                    CatalogError::ColumnNotFound(format!(
                        "Column '{}' does not exist in table '{}'",
                        name, table.name
                    ))
                })?;
                let dt = table.columns[idx].data_type;
                Ok(BoundExpr::ColumnRef {
                    col_name: name.clone(),
                    col_idx: idx,
                    data_type: dt,
                })
            }
            Expr::Binary { left, op, right } => {
                let bound_left = self.bind_expr(left, table)?;
                let bound_right = self.bind_expr(right, table)?;
                let return_type = match op {
                    BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::Lte
                    | BinaryOp::Gt
                    | BinaryOp::Gte
                    | BinaryOp::And
                    | BinaryOp::Or => DataType::Boolean,
                    _ => bound_left.return_type(),
                };
                Ok(BoundExpr::Binary {
                    left: Box::new(bound_left),
                    op: *op,
                    right: Box::new(bound_right),
                    return_type,
                })
            }
            Expr::Unary { op, expr } => {
                let bound_inner = self.bind_expr(expr, table)?;
                Ok(BoundExpr::Unary {
                    op: *op,
                    return_type: bound_inner.return_type(),
                    expr: Box::new(bound_inner),
                })
            }
            Expr::Wildcard => Err(CatalogError::ColumnNotFound(
                "Invalid wildcard position".into(),
            )),
        }
    }
}
