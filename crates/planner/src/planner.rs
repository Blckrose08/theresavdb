use crate::plan::LogicalPlan;
use theresav_binder::BoundStatement;

pub struct Planner;

impl Planner {
    pub fn create_logical_plan(stmt: BoundStatement) -> LogicalPlan {
        match stmt {
            BoundStatement::CreateTable {
                name,
                columns,
                if_not_exists,
            } => LogicalPlan::CreateTable {
                name,
                columns,
                if_not_exists,
            },
            BoundStatement::DropTable { name, if_exists } => {
                LogicalPlan::DropTable { name, if_exists }
            }
            BoundStatement::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            } => LogicalPlan::CreateIndex {
                name,
                table_name,
                columns,
                is_unique,
            },
            BoundStatement::DropIndex { name } => LogicalPlan::DropIndex { name },
            BoundStatement::Insert {
                table_catalog,
                column_indices,
                values,
            } => LogicalPlan::Insert {
                table_catalog,
                column_indices,
                values,
            },
            BoundStatement::Select {
                table_catalog,
                projection,
                where_clause,
                order_by,
                limit,
                offset,
            } => {
                let mut current_plan = LogicalPlan::Scan { table_catalog };

                if let Some(pred) = where_clause {
                    current_plan = LogicalPlan::Filter {
                        predicate: pred,
                        input: Box::new(current_plan),
                    };
                }

                current_plan = LogicalPlan::Projection {
                    exprs: projection,
                    input: Box::new(current_plan),
                };

                if let Some((col_idx, dir)) = order_by {
                    current_plan = LogicalPlan::Sort {
                        col_idx,
                        direction: dir,
                        input: Box::new(current_plan),
                    };
                }

                if limit.is_some() || offset.is_some() {
                    current_plan = LogicalPlan::Limit {
                        limit,
                        offset,
                        input: Box::new(current_plan),
                    };
                }

                current_plan
            }
            BoundStatement::Update {
                table_catalog,
                assignments,
                where_clause,
            } => LogicalPlan::Update {
                table_catalog,
                assignments,
                where_clause,
            },
            BoundStatement::Delete {
                table_catalog,
                where_clause,
            } => LogicalPlan::Delete {
                table_catalog,
                where_clause,
            },
            BoundStatement::Begin | BoundStatement::Commit | BoundStatement::Rollback => {
                panic!("Transaction commands do not produce a logical query plan")
            }
        }
    }
}
