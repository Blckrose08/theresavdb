//! Recursive descent parser translating SQL token stream into TheresavSQL AST.

use crate::ast::*;
use crate::token::{Lexer, Token};
use rust_decimal::Decimal;
use std::str::FromStr;
use theresav_error::TypeError;
use theresav_types::{DataType, Value};

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn parse_sql(sql: &str) -> Result<Statement, TypeError> {
        let mut lexer = Lexer::new(sql);
        let tokens = lexer.tokenize()?;
        let mut parser = Self { tokens, cursor: 0 };
        parser.parse_statement()
    }

    fn peek(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else {
            &Token::Eof
        }
    }

    fn next(&mut self) -> Token {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } else {
            Token::Eof
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), TypeError> {
        let actual = self.next();
        if actual == expected {
            Ok(())
        } else {
            Err(TypeError::ParseError(format!(
                "Expected token {:?}, found {:?}",
                expected, actual
            )))
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, TypeError> {
        match self.peek() {
            Token::Create => self.parse_create(),
            Token::Drop => self.parse_drop(),
            Token::Insert => self.parse_insert(),
            Token::Select => self.parse_select(),
            Token::Update => self.parse_update(),
            Token::Delete => self.parse_delete(),
            Token::Begin => {
                self.next();
                Ok(Statement::Begin)
            }
            Token::Commit => {
                self.next();
                Ok(Statement::Commit)
            }
            Token::Rollback => {
                self.next();
                Ok(Statement::Rollback)
            }
            other => Err(TypeError::ParseError(format!(
                "Unexpected statement start: {:?}",
                other
            ))),
        }
    }

    fn parse_create(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Create)?;
        let is_unique = if self.peek() == &Token::Unique {
            self.next();
            true
        } else {
            false
        };

        if self.peek() == &Token::Table {
            self.next();
            let mut if_not_exists = false;
            if self.peek() == &Token::If {
                self.next();
                self.expect(Token::Not)?;
                self.expect(Token::Exists)?;
                if_not_exists = true;
            }

            let name = self.parse_ident()?;
            self.expect(Token::LParen)?;

            let mut columns = Vec::new();
            while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                let col_name = self.parse_ident()?;
                let data_type = self.parse_data_type()?;

                let mut is_primary_key = false;
                let mut nullable = true;

                while self.peek() == &Token::Primary
                    || self.peek() == &Token::Not
                    || self.peek() == &Token::Unique
                {
                    if self.peek() == &Token::Primary {
                        self.next();
                        self.expect(Token::Key)?;
                        is_primary_key = true;
                        nullable = false;
                    } else if self.peek() == &Token::Not {
                        self.next();
                        self.expect(Token::Null)?;
                        nullable = false;
                    } else if self.peek() == &Token::Unique {
                        self.next();
                    }
                }

                columns.push(ColumnDef {
                    name: col_name,
                    data_type,
                    is_primary_key,
                    nullable,
                });

                if self.peek() == &Token::Comma {
                    self.next();
                } else {
                    break;
                }
            }

            self.expect(Token::RParen)?;
            Ok(Statement::CreateTable {
                name,
                if_not_exists,
                columns,
            })
        } else if self.peek() == &Token::Index {
            self.next();
            let index_name = self.parse_ident()?;
            self.expect(Token::On)?;
            let table_name = self.parse_ident()?;
            self.expect(Token::LParen)?;

            let mut cols = Vec::new();
            while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                cols.push(self.parse_ident()?);
                if self.peek() == &Token::Comma {
                    self.next();
                } else {
                    break;
                }
            }
            self.expect(Token::RParen)?;

            Ok(Statement::CreateIndex {
                name: index_name,
                table_name,
                columns: cols,
                is_unique,
            })
        } else {
            Err(TypeError::ParseError("Expected TABLE or INDEX".into()))
        }
    }

    fn parse_drop(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Drop)?;
        if self.peek() == &Token::Table {
            self.next();
            let mut if_exists = false;
            if self.peek() == &Token::If {
                self.next();
                self.expect(Token::Exists)?;
                if_exists = true;
            }
            let name = self.parse_ident()?;
            Ok(Statement::DropTable { name, if_exists })
        } else if self.peek() == &Token::Index {
            self.next();
            let name = self.parse_ident()?;
            Ok(Statement::DropIndex { name })
        } else {
            Err(TypeError::ParseError(
                "Expected TABLE or INDEX after DROP".into(),
            ))
        }
    }

    fn parse_insert(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Insert)?;
        self.expect(Token::Into)?;
        let table_name = self.parse_ident()?;

        let mut columns = None;
        if self.peek() == &Token::LParen {
            self.next();
            let mut cols = Vec::new();
            while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                cols.push(self.parse_ident()?);
                if self.peek() == &Token::Comma {
                    self.next();
                } else {
                    break;
                }
            }
            self.expect(Token::RParen)?;
            columns = Some(cols);
        }

        self.expect(Token::Values)?;
        let mut all_values = Vec::new();

        while self.peek() == &Token::LParen {
            self.next();
            let mut row = Vec::new();
            while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                row.push(self.parse_expr()?);
                if self.peek() == &Token::Comma {
                    self.next();
                } else {
                    break;
                }
            }
            self.expect(Token::RParen)?;
            all_values.push(row);

            if self.peek() == &Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        Ok(Statement::Insert {
            table_name,
            columns,
            values: all_values,
        })
    }

    fn parse_select(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Select)?;
        let mut distinct = false;
        if matches!(self.peek(), Token::Ident(s) if s.eq_ignore_ascii_case("DISTINCT")) {
            self.next();
            distinct = true;
        }

        let mut projection = Vec::new();
        while self.peek() != &Token::From && self.peek() != &Token::Eof {
            if self.peek() == &Token::Star {
                self.next();
                projection.push(Expr::Wildcard);
            } else {
                projection.push(self.parse_expr()?);
            }

            if self.peek() == &Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        self.expect(Token::From)?;
        let table_name = self.parse_ident()?;

        let mut where_clause = None;
        if self.peek() == &Token::Where {
            self.next();
            where_clause = Some(self.parse_expr()?);
        }

        let mut order_by = None;
        if self.peek() == &Token::Order {
            self.next();
            self.expect(Token::By)?;
            let col = self.parse_ident()?;
            let dir = if self.peek() == &Token::Desc {
                self.next();
                OrderDirection::Desc
            } else {
                if self.peek() == &Token::Asc {
                    self.next();
                }
                OrderDirection::Asc
            };
            order_by = Some((col, dir));
        }

        let mut limit = None;
        if self.peek() == &Token::Limit {
            self.next();
            if let Token::NumberLiteral(s) = self.next() {
                limit = Some(
                    s.parse::<usize>()
                        .map_err(|e| TypeError::ParseError(e.to_string()))?,
                );
            }
        }

        let mut offset = None;
        if self.peek() == &Token::Offset {
            self.next();
            if let Token::NumberLiteral(s) = self.next() {
                offset = Some(
                    s.parse::<usize>()
                        .map_err(|e| TypeError::ParseError(e.to_string()))?,
                );
            }
        }

        Ok(Statement::Select {
            distinct,
            projection,
            table_name,
            where_clause,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_update(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Update)?;
        let table_name = self.parse_ident()?;
        self.expect(Token::Set)?;

        let mut assignments = Vec::new();
        while self.peek() != &Token::Where && self.peek() != &Token::Eof {
            let col = self.parse_ident()?;
            self.expect(Token::Eq)?;
            let expr = self.parse_expr()?;
            assignments.push((col, expr));

            if self.peek() == &Token::Comma {
                self.next();
            } else {
                break;
            }
        }

        let mut where_clause = None;
        if self.peek() == &Token::Where {
            self.next();
            where_clause = Some(self.parse_expr()?);
        }

        Ok(Statement::Update {
            table_name,
            assignments,
            where_clause,
        })
    }

    fn parse_delete(&mut self) -> Result<Statement, TypeError> {
        self.expect(Token::Delete)?;
        self.expect(Token::From)?;
        let table_name = self.parse_ident()?;

        let mut where_clause = None;
        if self.peek() == &Token::Where {
            self.next();
            where_clause = Some(self.parse_expr()?);
        }

        Ok(Statement::Delete {
            table_name,
            where_clause,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, TypeError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, TypeError> {
        let mut left = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.next();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, TypeError> {
        let mut left = self.parse_equality()?;
        while self.peek() == &Token::And {
            self.next();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, TypeError> {
        let mut left = self.parse_relational()?;
        while matches!(self.peek(), Token::Eq | Token::NotEq) {
            let op = match self.next() {
                Token::Eq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::NotEq,
                _ => unreachable!(),
            };
            let right = self.parse_relational()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, TypeError> {
        let mut left = self.parse_primary()?;
        while matches!(self.peek(), Token::Lt | Token::Lte | Token::Gt | Token::Gte) {
            let op = match self.next() {
                Token::Lt => BinaryOp::Lt,
                Token::Lte => BinaryOp::Lte,
                Token::Gt => BinaryOp::Gt,
                Token::Gte => BinaryOp::Gte,
                _ => unreachable!(),
            };
            let right = self.parse_primary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, TypeError> {
        match self.next() {
            Token::Ident(name) => Ok(Expr::Column(name)),
            Token::StringLiteral(s) => Ok(Expr::Literal(Value::Text(s))),
            Token::NumberLiteral(num) => {
                if num.contains('.') {
                    let dec = Decimal::from_str(&num)
                        .map_err(|e| TypeError::ParseError(e.to_string()))?;
                    Ok(Expr::Literal(Value::Decimal(dec)))
                } else {
                    let val = num
                        .parse::<i64>()
                        .map_err(|e| TypeError::ParseError(e.to_string()))?;
                    Ok(Expr::Literal(Value::BigInt(val)))
                }
            }
            Token::True => Ok(Expr::Literal(Value::Boolean(true))),
            Token::False => Ok(Expr::Literal(Value::Boolean(false))),
            Token::Null => Ok(Expr::Literal(Value::Null)),
            Token::LParen => {
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            other => Err(TypeError::ParseError(format!(
                "Unexpected token in expression: {:?}",
                other
            ))),
        }
    }

    fn parse_ident(&mut self) -> Result<String, TypeError> {
        match self.next() {
            Token::Ident(s) => Ok(s),
            other => Err(TypeError::ParseError(format!(
                "Expected identifier, found {:?}",
                other
            ))),
        }
    }

    fn parse_data_type(&mut self) -> Result<DataType, TypeError> {
        match self.next() {
            Token::TypeBoolean => Ok(DataType::Boolean),
            Token::TypeSmallInt => Ok(DataType::SmallInt),
            Token::TypeInteger => Ok(DataType::Integer),
            Token::TypeBigInt => Ok(DataType::BigInt),
            Token::TypeReal => Ok(DataType::Real),
            Token::TypeDouble => Ok(DataType::Double),
            Token::TypeDecimal => Ok(DataType::Decimal),
            Token::TypeText => Ok(DataType::Text),
            Token::TypeVarchar => {
                if self.peek() == &Token::LParen {
                    self.next();
                    let _len = self.next();
                    self.expect(Token::RParen)?;
                }
                Ok(DataType::Varchar)
            }
            Token::TypeBytea => Ok(DataType::Bytea),
            Token::TypeUuid => Ok(DataType::Uuid),
            Token::TypeTimestamp => Ok(DataType::Timestamp),
            Token::TypeDate => Ok(DataType::Date),
            Token::TypeJson => Ok(DataType::Json),
            Token::TypeJsonb => Ok(DataType::Jsonb),
            other => Err(TypeError::ParseError(format!(
                "Expected data type, found {:?}",
                other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_parser_statements() {
        // CREATE TABLE
        let sql1 = "CREATE TABLE users (id BIGINT PRIMARY KEY, name TEXT NOT NULL, age INTEGER);";
        let stmt1 = Parser::parse_sql(sql1).unwrap();
        assert!(matches!(stmt1, Statement::CreateTable { .. }));

        // INSERT
        let sql2 = "INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);";
        let stmt2 = Parser::parse_sql(sql2).unwrap();
        assert!(matches!(stmt2, Statement::Insert { .. }));

        // SELECT
        let sql3 = "SELECT id, name FROM users WHERE age >= 18 ORDER BY id ASC LIMIT 10;";
        let stmt3 = Parser::parse_sql(sql3).unwrap();
        assert!(matches!(stmt3, Statement::Select { .. }));

        // UPDATE
        let sql4 = "UPDATE users SET age = 31 WHERE id = 1;";
        let stmt4 = Parser::parse_sql(sql4).unwrap();
        assert!(matches!(stmt4, Statement::Update { .. }));

        // DELETE
        let sql5 = "DELETE FROM users WHERE id = 1;";
        let stmt5 = Parser::parse_sql(sql5).unwrap();
        assert!(matches!(stmt5, Statement::Delete { .. }));
    }
}
