use theresav_error::TypeError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Create,
    Table,
    Drop,
    Index,
    On,
    Unique,
    Order,
    By,
    Asc,
    Desc,
    Limit,
    Offset,
    Primary,
    Key,
    Not,
    Null,
    And,
    Or,
    True,
    False,
    If,
    Exists,
    Begin,
    Commit,
    Rollback,

    TypeBoolean,
    TypeSmallInt,
    TypeInteger,
    TypeBigInt,
    TypeReal,
    TypeDouble,
    TypeDecimal,
    TypeText,
    TypeVarchar,
    TypeBytea,
    TypeUuid,
    TypeTimestamp,
    TypeDate,
    TypeJson,
    TypeJsonb,

    Ident(String),
    StringLiteral(String),
    NumberLiteral(String),

    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
    Plus,
    Minus,
    Star,
    Slash,
    Comma,
    Semicolon,
    LParen,
    RParen,

    Eof,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, TypeError> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token()? {
            let is_eof = tok == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Option<Token>, TypeError> {
        self.skip_whitespace();

        let (idx, ch) = match self.chars.next() {
            Some(pair) => pair,
            None => return Ok(Some(Token::Eof)),
        };

        match ch {
            ',' => Ok(Some(Token::Comma)),
            ';' => Ok(Some(Token::Semicolon)),
            '(' => Ok(Some(Token::LParen)),
            ')' => Ok(Some(Token::RParen)),
            '+' => Ok(Some(Token::Plus)),
            '-' => Ok(Some(Token::Minus)),
            '*' => Ok(Some(Token::Star)),
            '/' => Ok(Some(Token::Slash)),
            '=' => Ok(Some(Token::Eq)),
            '<' => {
                if let Some(&(_, '=')) = self.chars.peek() {
                    self.chars.next();
                    Ok(Some(Token::Lte))
                } else if let Some(&(_, '>')) = self.chars.peek() {
                    self.chars.next();
                    Ok(Some(Token::NotEq))
                } else {
                    Ok(Some(Token::Lt))
                }
            }
            '>' => {
                if let Some(&(_, '=')) = self.chars.peek() {
                    self.chars.next();
                    Ok(Some(Token::Gte))
                } else {
                    Ok(Some(Token::Gt))
                }
            }
            '!' => {
                if let Some(&(_, '=')) = self.chars.peek() {
                    self.chars.next();
                    Ok(Some(Token::NotEq))
                } else {
                    Err(TypeError::ParseError("Unexpected '!' character".into()))
                }
            }
            '\'' => {
                let start = idx + 1;
                let mut end = start;
                let mut closed = false;
                for (c_idx, c) in self.chars.by_ref() {
                    if c == '\'' {
                        closed = true;
                        end = c_idx;
                        break;
                    }
                }
                if !closed {
                    return Err(TypeError::ParseError("Unterminated string literal".into()));
                }
                Ok(Some(Token::StringLiteral(
                    self.input[start..end].to_string(),
                )))
            }
            c if c.is_ascii_digit() => {
                let start = idx;
                let mut end = idx + c.len_utf8();
                while let Some(&(c_idx, next_ch)) = self.chars.peek() {
                    if next_ch.is_ascii_digit() || next_ch == '.' {
                        end = c_idx + next_ch.len_utf8();
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                Ok(Some(Token::NumberLiteral(
                    self.input[start..end].to_string(),
                )))
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = idx;
                let mut end = idx + c.len_utf8();
                while let Some(&(c_idx, next_ch)) = self.chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        end = c_idx + next_ch.len_utf8();
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                let word = &self.input[start..end];
                Ok(Some(keyword_or_ident(word)))
            }
            other => Err(TypeError::ParseError(format!(
                "Unexpected character: '{}'",
                other
            ))),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }
}

fn keyword_or_ident(s: &str) -> Token {
    match s.to_ascii_uppercase().as_str() {
        "SELECT" => Token::Select,
        "FROM" => Token::From,
        "WHERE" => Token::Where,
        "INSERT" => Token::Insert,
        "INTO" => Token::Into,
        "VALUES" => Token::Values,
        "UPDATE" => Token::Update,
        "SET" => Token::Set,
        "DELETE" => Token::Delete,
        "CREATE" => Token::Create,
        "TABLE" => Token::Table,
        "DROP" => Token::Drop,
        "INDEX" => Token::Index,
        "ON" => Token::On,
        "UNIQUE" => Token::Unique,
        "ORDER" => Token::Order,
        "BY" => Token::By,
        "ASC" => Token::Asc,
        "DESC" => Token::Desc,
        "LIMIT" => Token::Limit,
        "OFFSET" => Token::Offset,
        "PRIMARY" => Token::Primary,
        "KEY" => Token::Key,
        "NOT" => Token::Not,
        "NULL" => Token::Null,
        "AND" => Token::And,
        "OR" => Token::Or,
        "TRUE" => Token::True,
        "FALSE" => Token::False,
        "IF" => Token::If,
        "EXISTS" => Token::Exists,
        "BEGIN" => Token::Begin,
        "COMMIT" => Token::Commit,
        "ROLLBACK" => Token::Rollback,

        "BOOLEAN" | "BOOL" => Token::TypeBoolean,
        "SMALLINT" | "INT2" => Token::TypeSmallInt,
        "INTEGER" | "INT" | "INT4" => Token::TypeInteger,
        "BIGINT" | "INT8" => Token::TypeBigInt,
        "REAL" | "FLOAT4" => Token::TypeReal,
        "DOUBLE" | "FLOAT8" => Token::TypeDouble,
        "DECIMAL" | "NUMERIC" => Token::TypeDecimal,
        "TEXT" => Token::TypeText,
        "VARCHAR" => Token::TypeVarchar,
        "BYTEA" => Token::TypeBytea,
        "UUID" => Token::TypeUuid,
        "TIMESTAMP" => Token::TypeTimestamp,
        "DATE" => Token::TypeDate,
        "JSON" => Token::TypeJson,
        "JSONB" => Token::TypeJsonb,

        _ => Token::Ident(s.to_string()),
    }
}
