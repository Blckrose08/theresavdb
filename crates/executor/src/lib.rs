use std::collections::HashMap;
use std::sync::RwLock;
use theresav_catalog::{Catalog, ColumnCatalog};
use theresav_error::StorageError;
use theresav_index::BTreeIndex;
use theresav_optimizer::PhysicalPlan;
use theresav_parser::ast::OrderDirection;
use theresav_types::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub rows_affected: u64,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: 0,
        }
    }

    pub fn rows(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        let count = rows.len() as u64;
        Self {
            columns,
            rows,
            rows_affected: count,
        }
    }

    pub fn affected(count: u64) -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            rows_affected: count,
        }
    }
}

pub struct ExecutionContext<'a> {
    pub catalog: &'a Catalog,
    pub tables: &'a RwLock<HashMap<String, Vec<Vec<Value>>>>,
    pub indexes: &'a RwLock<HashMap<String, BTreeIndex>>,
}

pub struct Executor;

impl Executor {
    pub fn execute(
        plan: PhysicalPlan,
        ctx: &ExecutionContext,
    ) -> Result<QueryResult, StorageError> {
        match plan {
            PhysicalPlan::CreateTable {
                name,
                columns,
                if_not_exists,
            } => {
                if ctx.catalog.get_table(&name).is_ok() {
                    if if_not_exists {
                        return Ok(QueryResult::affected(0));
                    }
                    return Err(StorageError::CorruptedFile(format!(
                        "Table '{}' already exists",
                        name
                    )));
                }

                let col_catalogs: Vec<ColumnCatalog> = columns
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| {
                        ColumnCatalog::new(&c.name, c.data_type, c.nullable, c.is_primary_key, idx)
                    })
                    .collect();

                ctx.catalog
                    .create_table(&name, col_catalogs)
                    .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;

                let mut tables = ctx.tables.write().unwrap();
                tables.insert(name.to_ascii_lowercase(), Vec::new());

                Ok(QueryResult::affected(0))
            }
            PhysicalPlan::DropTable { name, if_exists } => {
                if ctx.catalog.get_table(&name).is_err() {
                    if if_exists {
                        return Ok(QueryResult::affected(0));
                    }
                    return Err(StorageError::CorruptedFile(format!(
                        "Table '{}' does not exist",
                        name
                    )));
                }

                ctx.catalog
                    .drop_table(&name)
                    .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;

                let mut tables = ctx.tables.write().unwrap();
                tables.remove(&name.to_ascii_lowercase());

                Ok(QueryResult::affected(0))
            }
            PhysicalPlan::Insert {
                table_catalog,
                column_indices,
                values,
            } => {
                let mut tables = ctx.tables.write().unwrap();
                let rows = tables
                    .get_mut(&table_catalog.name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        StorageError::CorruptedFile(format!(
                            "Table '{}' storage not found",
                            table_catalog.name
                        ))
                    })?;

                let mut inserted_count = 0;
                let empty_dummy = Vec::new();

                for val_row in values {
                    let mut new_row = vec![Value::Null; table_catalog.columns.len()];
                    for (i, expr) in val_row.into_iter().enumerate() {
                        let target_col_idx = column_indices[i];
                        let val = expr
                            .evaluate(&empty_dummy)
                            .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
                        new_row[target_col_idx] = val;
                    }
                    rows.push(new_row);
                    inserted_count += 1;
                }

                Ok(QueryResult::affected(inserted_count))
            }
            PhysicalPlan::SeqScan { table_catalog } => {
                let tables = ctx.tables.read().unwrap();
                let stored_rows = tables
                    .get(&table_catalog.name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_default();
                let cols = table_catalog
                    .columns
                    .iter()
                    .map(|c| c.name.clone())
                    .collect();
                Ok(QueryResult::rows(cols, stored_rows))
            }
            PhysicalPlan::IndexScan {
                table_catalog,
                search_key,
                ..
            } => {
                let tables = ctx.tables.read().unwrap();
                let stored_rows = tables
                    .get(&table_catalog.name.to_ascii_lowercase())
                    .cloned()
                    .unwrap_or_default();
                let target = search_key
                    .evaluate(&[])
                    .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;

                let filtered: Vec<Vec<Value>> = stored_rows
                    .into_iter()
                    .filter(|row| row.iter().any(|v| v == &target))
                    .collect();

                let cols = table_catalog
                    .columns
                    .iter()
                    .map(|c| c.name.clone())
                    .collect();
                Ok(QueryResult::rows(cols, filtered))
            }
            PhysicalPlan::Filter { predicate, input } => {
                let sub_res = Self::execute(*input, ctx)?;
                let mut filtered_rows = Vec::new();

                for row in sub_res.rows {
                    let res = predicate
                        .evaluate(&row)
                        .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
                    if let Value::Boolean(true) = res {
                        filtered_rows.push(row);
                    }
                }

                Ok(QueryResult::rows(sub_res.columns, filtered_rows))
            }
            PhysicalPlan::Projection { exprs, input } => {
                let sub_res = Self::execute(*input, ctx)?;
                let mut proj_rows = Vec::new();

                for row in sub_res.rows {
                    let mut new_row = Vec::with_capacity(exprs.len());
                    for (_, expr) in &exprs {
                        let val = expr
                            .evaluate(&row)
                            .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
                        new_row.push(val);
                    }
                    proj_rows.push(new_row);
                }

                let col_names: Vec<String> = exprs.into_iter().map(|(name, _)| name).collect();
                Ok(QueryResult::rows(col_names, proj_rows))
            }
            PhysicalPlan::Sort {
                col_idx,
                direction,
                input,
            } => {
                let mut sub_res = Self::execute(*input, ctx)?;
                sub_res.rows.sort_by(|a, b| {
                    let ord = a.get(col_idx).cmp(&b.get(col_idx));
                    if direction == OrderDirection::Desc {
                        ord.reverse()
                    } else {
                        ord
                    }
                });
                Ok(sub_res)
            }
            PhysicalPlan::Limit {
                limit,
                offset,
                input,
            } => {
                let sub_res = Self::execute(*input, ctx)?;
                let off = offset.unwrap_or(0);
                let rows: Vec<Vec<Value>> = match limit {
                    Some(lim) => sub_res.rows.into_iter().skip(off).take(lim).collect(),
                    None => sub_res.rows.into_iter().skip(off).collect(),
                };
                Ok(QueryResult::rows(sub_res.columns, rows))
            }
            PhysicalPlan::Update {
                table_catalog,
                assignments,
                where_clause,
            } => {
                let mut tables = ctx.tables.write().unwrap();
                let stored_rows = tables
                    .get_mut(&table_catalog.name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        StorageError::CorruptedFile(format!(
                            "Table '{}' not found",
                            table_catalog.name
                        ))
                    })?;

                let mut updated_count = 0;
                for row in stored_rows.iter_mut() {
                    let should_update = match &where_clause {
                        Some(f) => matches!(f.evaluate(row), Ok(Value::Boolean(true))),
                        None => true,
                    };

                    if should_update {
                        let snapshot = row.clone();
                        for (col_idx, expr) in &assignments {
                            let val = expr
                                .evaluate(&snapshot)
                                .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
                            row[*col_idx] = val;
                        }
                        updated_count += 1;
                    }
                }

                Ok(QueryResult::affected(updated_count))
            }
            PhysicalPlan::Delete {
                table_catalog,
                where_clause,
            } => {
                let mut tables = ctx.tables.write().unwrap();
                let stored_rows = tables
                    .get_mut(&table_catalog.name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        StorageError::CorruptedFile(format!(
                            "Table '{}' not found",
                            table_catalog.name
                        ))
                    })?;

                let original_count = stored_rows.len();
                stored_rows.retain(|row| {
                    if let Some(f) = &where_clause {
                        !matches!(f.evaluate(row), Ok(Value::Boolean(true)))
                    } else {
                        false
                    }
                });

                let deleted_count = (original_count - stored_rows.len()) as u64;
                Ok(QueryResult::affected(deleted_count))
            }
            PhysicalPlan::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            } => {
                ctx.catalog
                    .create_index(&name, &table_name, columns, is_unique)
                    .map_err(|e| StorageError::CorruptedFile(e.to_string()))?;
                Ok(QueryResult::affected(0))
            }
            PhysicalPlan::DropIndex { .. } => Ok(QueryResult::affected(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use theresav_binder::Binder;
    use theresav_optimizer::Optimizer;
    use theresav_parser::Parser;
    use theresav_planner::Planner;

    #[test]
    fn test_sql_query_pipeline() {
        let catalog = Catalog::new();
        let tables = RwLock::new(HashMap::new());
        let indexes = RwLock::new(HashMap::new());
        let ctx = ExecutionContext {
            catalog: &catalog,
            tables: &tables,
            indexes: &indexes,
        };

        let sql_create = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, active BOOLEAN);";
        let stmt = Parser::parse_sql(sql_create).unwrap();
        let bound = Binder::new(&catalog).bind_statement(stmt).unwrap();
        let logical = Planner::create_logical_plan(bound);
        let physical = Optimizer::optimize(logical, &catalog);
        let res = Executor::execute(physical, &ctx).unwrap();
        assert_eq!(res.rows_affected, 0);

        let sql_insert =
            "INSERT INTO users (id, name, active) VALUES (1, 'Alice', true), (2, 'Bob', false);";
        let stmt = Parser::parse_sql(sql_insert).unwrap();
        let bound = Binder::new(&catalog).bind_statement(stmt).unwrap();
        let logical = Planner::create_logical_plan(bound);
        let physical = Optimizer::optimize(logical, &catalog);
        let res = Executor::execute(physical, &ctx).unwrap();
        assert_eq!(res.rows_affected, 2);

        let sql_select = "SELECT id, name FROM users WHERE id = 1;";
        let stmt = Parser::parse_sql(sql_select).unwrap();
        let bound = Binder::new(&catalog).bind_statement(stmt).unwrap();
        let logical = Planner::create_logical_plan(bound);
        let physical = Optimizer::optimize(logical, &catalog);
        let res = Executor::execute(physical, &ctx).unwrap();
        assert_eq!(res.rows.len(), 1);
        assert_eq!(res.rows[0][0], Value::BigInt(1));
        assert_eq!(res.rows[0][1], Value::Text("Alice".into()));
    }
}
