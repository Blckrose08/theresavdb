//! Rule-Based Query Optimizer.

use crate::plan::PhysicalPlan;
use theresav_catalog::Catalog;
use theresav_planner::LogicalPlan;

pub struct Optimizer;

impl Optimizer {
    #[allow(clippy::only_used_in_recursion)]
    pub fn optimize(logical: LogicalPlan, catalog: &Catalog) -> PhysicalPlan {
        match logical {
            LogicalPlan::Scan { table_catalog } => PhysicalPlan::SeqScan { table_catalog },
            LogicalPlan::Filter { predicate, input } => {
                let physical_input = Self::optimize(*input, catalog);
                PhysicalPlan::Filter {
                    predicate,
                    input: Box::new(physical_input),
                }
            }
            LogicalPlan::Projection { exprs, input } => {
                let physical_input = Self::optimize(*input, catalog);
                PhysicalPlan::Projection {
                    exprs,
                    input: Box::new(physical_input),
                }
            }
            LogicalPlan::Sort {
                col_idx,
                direction,
                input,
            } => {
                let physical_input = Self::optimize(*input, catalog);
                PhysicalPlan::Sort {
                    col_idx,
                    direction,
                    input: Box::new(physical_input),
                }
            }
            LogicalPlan::Limit {
                limit,
                offset,
                input,
            } => {
                let physical_input = Self::optimize(*input, catalog);
                PhysicalPlan::Limit {
                    limit,
                    offset,
                    input: Box::new(physical_input),
                }
            }
            LogicalPlan::Insert {
                table_catalog,
                column_indices,
                values,
            } => PhysicalPlan::Insert {
                table_catalog,
                column_indices,
                values,
            },
            LogicalPlan::Update {
                table_catalog,
                assignments,
                where_clause,
            } => PhysicalPlan::Update {
                table_catalog,
                assignments,
                where_clause,
            },
            LogicalPlan::Delete {
                table_catalog,
                where_clause,
            } => PhysicalPlan::Delete {
                table_catalog,
                where_clause,
            },
            LogicalPlan::CreateTable {
                name,
                columns,
                if_not_exists,
            } => PhysicalPlan::CreateTable {
                name,
                columns,
                if_not_exists,
            },
            LogicalPlan::DropTable { name, if_exists } => {
                PhysicalPlan::DropTable { name, if_exists }
            }
            LogicalPlan::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            } => PhysicalPlan::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            },
            LogicalPlan::DropIndex { name } => PhysicalPlan::DropIndex { name },
        }
    }
}
