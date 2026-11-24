//! Bound expressions with resolved catalog types and evaluation routines.

use std::cmp::Ordering;
use theresav_error::TypeError;
use theresav_parser::ast::{BinaryOp, UnaryOp};
use theresav_types::{DataType, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum BoundExpr {
    Literal(Value),
    ColumnRef {
        col_idx: usize,
        col_name: String,
        data_type: DataType,
    },
    Binary {
        left: Box<BoundExpr>,
        op: BinaryOp,
        right: Box<BoundExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<BoundExpr>,
    },
}

impl BoundExpr {
    pub fn evaluate(&self, row: &[Value]) -> Result<Value, TypeError> {
        match self {
            Self::Literal(val) => Ok(val.clone()),
            Self::ColumnRef { col_idx, .. } => {
                if *col_idx < row.len() {
                    Ok(row[*col_idx].clone())
                } else {
                    Err(TypeError::InvalidEncoding(format!(
                        "Column index {} out of range (row len: {})",
                        col_idx,
                        row.len()
                    )))
                }
            }
            Self::Unary { op, expr } => {
                let v = expr.evaluate(row)?;
                match op {
                    UnaryOp::Not => match v {
                        Value::Boolean(b) => Ok(Value::Boolean(!b)),
                        Value::Null => Ok(Value::Null),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "BOOLEAN",
                            found: "Non-boolean in NOT",
                        }),
                    },
                    UnaryOp::Negate => match v {
                        Value::SmallInt(n) => Ok(Value::SmallInt(-n)),
                        Value::Integer(n) => Ok(Value::Integer(-n)),
                        Value::BigInt(n) => Ok(Value::BigInt(-n)),
                        Value::Real(n) => Ok(Value::Real(-n)),
                        Value::Double(n) => Ok(Value::Double(-n)),
                        Value::Decimal(n) => Ok(Value::Decimal(-n)),
                        Value::Null => Ok(Value::Null),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "NUMERIC",
                            found: "Non-numeric in unary negation",
                        }),
                    },
                }
            }
            Self::Binary { left, op, right } => {
                let lv = left.evaluate(row)?;
                let rv = right.evaluate(row)?;

                match op {
                    BinaryOp::Eq => Ok(Value::Boolean(lv == rv)),
                    BinaryOp::NotEq => Ok(Value::Boolean(lv != rv)),
                    BinaryOp::Lt => Ok(Value::Boolean(lv < rv)),
                    BinaryOp::Lte => Ok(Value::Boolean(lv <= rv)),
                    BinaryOp::Gt => Ok(Value::Boolean(lv > rv)),
                    BinaryOp::Gte => Ok(Value::Boolean(lv >= rv)),
                    BinaryOp::And => match (lv, rv) {
                        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "BOOLEAN",
                            found: "Non-boolean in AND",
                        }),
                    },
                    BinaryOp::Or => match (lv, rv) {
                        (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
                        _ => Err(TypeError::TypeMismatch {
                            expected: "BOOLEAN",
                            found: "Non-boolean in OR",
                        }),
                    },
                    BinaryOp::Plus => add_values(lv, rv),
                    BinaryOp::Minus => sub_values(lv, rv),
                    BinaryOp::Multiply => mul_values(lv, rv),
                    BinaryOp::Divide => div_values(lv, rv),
                }
            }
        }
    }
}

fn add_values(lv: Value, rv: Value) -> Result<Value, TypeError> {
    match (lv, rv) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.wrapping_add(b))),
        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a.wrapping_add(b))),
        (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
        (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Decimal(a + b)),
        _ => Err(TypeError::TypeMismatch {
            expected: "NUMERIC",
            found: "Incompatible types in ADD",
        }),
    }
}

fn sub_values(lv: Value, rv: Value) -> Result<Value, TypeError> {
    match (lv, rv) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.wrapping_sub(b))),
        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a.wrapping_sub(b))),
        (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
        (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Decimal(a - b)),
        _ => Err(TypeError::TypeMismatch {
            expected: "NUMERIC",
            found: "Incompatible types in SUB",
        }),
    }
}

fn mul_values(lv: Value, rv: Value) -> Result<Value, TypeError> {
    match (lv, rv) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.wrapping_mul(b))),
        (Value::BigInt(a), Value::BigInt(b)) => Ok(Value::BigInt(a.wrapping_mul(b))),
        (Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
        (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Decimal(a * b)),
        _ => Err(TypeError::TypeMismatch {
            expected: "NUMERIC",
            found: "Incompatible types in MUL",
        }),
    }
}

fn div_values(lv: Value, rv: Value) -> Result<Value, TypeError> {
    match (lv, rv) {
        (Value::Integer(a), Value::Integer(b)) => {
            if b == 0 {
                return Err(TypeError::DivisionByZero);
            }
            Ok(Value::Integer(a / b))
        }
        (Value::BigInt(a), Value::BigInt(b)) => {
            if b == 0 {
                return Err(TypeError::DivisionByZero);
            }
            Ok(Value::BigInt(a / b))
        }
        (Value::Double(a), Value::Double(b)) => {
            if b == 0.0 {
                return Err(TypeError::DivisionByZero);
            }
            Ok(Value::Double(a / b))
        }
        (Value::Decimal(a), Value::Decimal(b)) => {
            if b == rust_decimal::Decimal::ZERO {
                return Err(TypeError::DivisionByZero);
            }
            Ok(Value::Decimal(a / b))
        }
        _ => Err(TypeError::TypeMismatch {
            expected: "NUMERIC",
            found: "Incompatible types in DIV",
        }),
    }
}
