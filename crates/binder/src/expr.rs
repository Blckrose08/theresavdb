//! Bound expressions with resolved column indices and data types.

use theresav_error::TypeError;
use theresav_parser::ast::{BinaryOp, UnaryOp};
use theresav_types::{DataType, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum BoundExpr {
    Literal(Value),
    ColumnRef {
        col_name: String,
        col_idx: usize,
        data_type: DataType,
    },
    Binary {
        left: Box<BoundExpr>,
        op: BinaryOp,
        right: Box<BoundExpr>,
        return_type: DataType,
    },
    Unary {
        op: UnaryOp,
        expr: Box<BoundExpr>,
        return_type: DataType,
    },
}

impl BoundExpr {
    pub fn return_type(&self) -> DataType {
        match self {
            Self::Literal(v) => v.data_type().unwrap_or(DataType::Text),
            Self::ColumnRef { data_type, .. } => *data_type,
            Self::Binary { return_type, .. } => *return_type,
            Self::Unary { return_type, .. } => *return_type,
        }
    }

    pub fn evaluate(&self, row: &[Value]) -> Result<Value, TypeError> {
        match self {
            Self::Literal(v) => Ok(v.clone()),
            Self::ColumnRef { col_idx, .. } => {
                if *col_idx < row.len() {
                    Ok(row[*col_idx].clone())
                } else {
                    Ok(Value::Null)
                }
            }
            Self::Unary { op, expr, .. } => {
                let v = expr.evaluate(row)?;
                match op {
                    UnaryOp::Not => match v {
                        Value::Boolean(b) => Ok(Value::Boolean(!b)),
                        Value::Null => Ok(Value::Null),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "BOOLEAN",
                            found: "non-boolean",
                        }),
                    },
                    UnaryOp::Negate => match v {
                        Value::SmallInt(n) => Ok(Value::SmallInt(-n)),
                        Value::Integer(n) => Ok(Value::Integer(-n)),
                        Value::BigInt(n) => Ok(Value::BigInt(-n)),
                        Value::Real(n) => Ok(Value::Real(-n)),
                        Value::Double(n) => Ok(Value::Double(-n)),
                        Value::Decimal(d) => Ok(Value::Decimal(-d)),
                        Value::Null => Ok(Value::Null),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "numeric",
                            found: "non-numeric",
                        }),
                    },
                }
            }
            Self::Binary {
                left, op, right, ..
            } => {
                let l = left.evaluate(row)?;
                let r = right.evaluate(row)?;
                if l.is_null() || r.is_null() {
                    return Ok(Value::Null);
                }
                match op {
                    BinaryOp::Eq => Ok(Value::Boolean(l == r)),
                    BinaryOp::NotEq => Ok(Value::Boolean(l != r)),
                    BinaryOp::Lt => Ok(Value::Boolean(l < r)),
                    BinaryOp::Lte => Ok(Value::Boolean(l <= r)),
                    BinaryOp::Gt => Ok(Value::Boolean(l > r)),
                    BinaryOp::Gte => Ok(Value::Boolean(l >= r)),
                    BinaryOp::And => match (l, r) {
                        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
                        _ => Ok(Value::Null),
                    },
                    BinaryOp::Or => match (l, r) {
                        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
                        _ => Ok(Value::Null),
                    },
                    BinaryOp::Plus => match (l, r) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a + b)),
                        _ => Ok(Value::Null),
                    },
                    BinaryOp::Minus => match (l, r) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a - b)),
                        _ => Ok(Value::Null),
                    },
                    BinaryOp::Multiply => match (l, r) {
                        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a * b)),
                        _ => Ok(Value::Null),
                    },
                    BinaryOp::Divide => match (l, r) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if b != 0 {
                                Ok(Value::Integer(a / b))
                            } else {
                                Err(TypeError::DivisionByZero)
                            }
                        }
                        (Value::BigInt(a), Value::BigInt(b)) => {
                            if b != 0 {
                                Ok(Value::BigInt(a / b))
                            } else {
                                Err(TypeError::DivisionByZero)
                            }
                        }
                        _ => Ok(Value::Null),
                    },
                }
            }
        }
    }
}
