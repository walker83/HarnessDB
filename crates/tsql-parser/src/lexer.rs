//! T-SQL Lexer — Tokenizes T-SQL input into a stream of tokens.
//!
//! Handles T-SQL-specific tokens like @variables, @@system_variables,
//! #temp_tables, money literals ($123.45), bracket-quoted identifiers [name],
//! and old-style join operators (*= and =*).

use crate::error::{TsqlParseError, TsqlResult};

#[derive(Debug, Clone, PartialEq)]
pub enum TsqlToken {
    // ── Keywords ──
    Select,
    From,
    Where,
    Insert,
    Into,
    Update,
    Delete,
    Create,
    Alter,
    Drop,
    Table,
    Database,
    View,
    Index,
    Procedure,
    Proc,
    Function,
    Trigger,
    Begin,
    End,
    If,
    Else,
    While,
    Return,
    Returns,
    Goto,
    WaitFor,
    Declare,
    Set,
    Print,
    Execute,
    Exec,
    Try,
    Catch,
    Throw,
    Raiserror,
    Cursor,
    Open,
    Fetch,
    Close,
    Deallocate,
    Commit,
    Rollback,
    Save,
    Transaction,
    Tran,
    Go,
    Use,
    Schema,
    And,
    Or,
    Not,
    In,
    Between,
    Like,
    Is,
    Null,
    Exists,
    Case,
    When,
    Then,
    As,
    On,
    Join,
    Left,
    Right,
    Inner,
    Outer,
    Full,
    Cross,
    Union,
    All,
    Any,
    Some,
    Having,
    Group,
    Order,
    By,
    Asc,
    Desc,
    Top,
    Percent,
    With,
    Ties,
    Distinct,
    Compute,
    Browse,
    For,
    Option,
    Output,
    Default,
    Values,
    Nulls,
    First,
    Last,
    Prior,
    Next,
    Absolute,
    Relative,
    ForwardOnly,
    Scroll,
    Keyset,
    Dynamic,
    Static,
    Insensitive,
    Sensitive,
    ReadOnly,
    Merge,
    Matched,
    Using,
    Constraint,
    Primary,
    Key,
    Foreign,
    References,
    Unique,
    Check,
    Clustered,
    NonClustered,
    Identity,
    Seed,
    Increment,
    Replication,
    Truncate,
    Add,
    Column,
    Enable,
    Disable,
    NoCheck,
    Cascade,
    NoCascade,
    Replicate,
    NotForReplication,
    Delimited,
    Varying,
    Datapages,
    Lock,
    TextImageOn,
    Log,
    Delay,
    Time,
    Break,
    Continue,
    Over,
    Partition,
    Row,
    Rows,
    Range,
    Preceding,
    Following,
    Unbounded,
    Current,
    Of,
    Offset,
    RowCount,
    NoCount,
    AnsiNulls,
    AnsiPadding,
    QuotedIdentifierKw,
    ConcatNullYieldsNull,
    ImplicitTransactions,
    XactAbort,
    ArithAbort,
    Nocount,
    ShowplanAll,
    ShowplanText,
    Language,
    Encrypted,
    Recompile,
    Deny,
    Grant,
    Revoke,
    TempDb,

    // ── Operators ──
    Equals,
    NotEquals,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    PercentOp,
    PlusEquals,
    MinusEquals,
    StarEquals,
    SlashEquals,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    AndAssign,
    OrAssign,
    XorAssign,
    Dot,
    Comma,
    Semicolon,
    Colon,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    DoubleColon,
    LeftOuterJoin,
    RightOuterJoin,
    StringConcat,

    // ── Literals ──
    IntLiteral(i64),
    BigIntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    NStringLiteral(String),
    BinaryLiteral(Vec<u8>),
    MoneyLiteral(String),
    DateTimeLiteral(String),

    // ── Identifiers ──
    Identifier(String),
    QuotedIdentifier(String),
    AtVariable(String),
    AtAtVariable(String),
    PoundTemp(String),
    DoublePoundTemp(String),

    // ── Special ──
    Eof,
    Newline,
    Whitespace,
}

impl TsqlToken {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Self::Select
                | Self::From
                | Self::Where
                | Self::Insert
                | Self::Into
                | Self::Update
                | Self::Delete
                | Self::Create
                | Self::Alter
                | Self::Drop
                | Self::Table
                | Self::Database
                | Self::View
                | Self::Index
                | Self::Procedure
                | Self::Proc
                | Self::Function
                | Self::Begin
                | Self::End
                | Self::If
                | Self::Else
                | Self::While
                | Self::Return
                | Self::Declare
                | Self::Set
                | Self::Execute
                | Self::Exec
                | Self::Go
                | Self::Use
                | Self::And
                | Self::Or
                | Self::Not
                | Self::In
                | Self::Between
                | Self::Like
                | Self::Is
                | Self::Null
                | Self::Exists
                | Self::Case
                | Self::When
                | Self::Then
                | Self::As
                | Self::On
                | Self::Join
                | Self::Left
                | Self::Right
                | Self::Inner
                | Self::Outer
                | Self::Union
                | Self::All
                | Self::Having
                | Self::Group
                | Self::Order
                | Self::By
                | Self::Top
                | Self::With
                | Self::Distinct
                | Self::Values
                | Self::Default
                | Self::Try
                | Self::Catch
                | Self::Throw
                | Self::Raiserror
                | Self::Open
                | Self::Fetch
                | Self::Close
                | Self::Deallocate
                | Self::Commit
                | Self::Rollback
                | Self::Save
                | Self::Transaction
                | Self::Tran
                | Self::Cursor
                | Self::Print
                | Self::Break
                | Self::Continue
                | Self::Goto
                | Self::WaitFor
        )
    }
}

pub struct TsqlLexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl TsqlLexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> TsqlResult<Vec<TsqlToken>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok == TsqlToken::Eof;
            match &tok {
                TsqlToken::Whitespace | TsqlToken::Newline => {}
                _ => tokens.push(tok),
            }
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn err(&self, msg: impl Into<String>) -> TsqlParseError {
        TsqlParseError::new(msg, self.pos, self.line, self.col)
    }

    fn next_token(&mut self) -> TsqlResult<TsqlToken> {
        let ch = match self.peek() {
            Some(c) => c,
            None => return Ok(TsqlToken::Eof),
        };

        // Whitespace
        if ch.is_whitespace() {
            self.advance();
            return if ch == '\n' {
                Ok(TsqlToken::Newline)
            } else {
                Ok(TsqlToken::Whitespace)
            };
        }

        // Line comment --
        if ch == '-' && self.peek_ahead(1) == Some('-') {
            self.advance();
            self.advance();
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.advance();
            }
            return Ok(TsqlToken::Whitespace);
        }

        // Block comment /* ... */
        if ch == '/' && self.peek_ahead(1) == Some('*') {
            self.advance();
            self.advance();
            let mut depth = 1;
            while depth > 0 {
                match self.advance() {
                    Some('*') if self.peek() == Some('/') => {
                        self.advance();
                        depth -= 1;
                    }
                    Some('/') if self.peek() == Some('*') => {
                        self.advance();
                        depth += 1;
                    }
                    None => return Err(self.err("unterminated block comment")),
                    _ => {}
                }
            }
            return Ok(TsqlToken::Whitespace);
        }

        // String literal 'text' or N'text'
        if ch == '\'' {
            return self.read_string_literal(false);
        }
        if (ch == 'N' || ch == 'n') && self.peek_ahead(1) == Some('\'') {
            self.advance();
            return self.read_string_literal(true);
        }

        // Binary literal 0x...
        if ch == '0'
            && matches!(self.peek_ahead(1), Some('x') | Some('X'))
        {
            return self.read_binary_literal();
        }

        // Money literal $123.45
        if ch == '$' {
            return self.read_money_literal();
        }

        // @variable
        if ch == '@' {
            return self.read_variable();
        }

        // #temp table
        if ch == '#' {
            return self.read_temp_table();
        }

        // Number
        if ch.is_ascii_digit() {
            return self.read_number();
        }

        // Bracket-quoted identifier [name]
        if ch == '[' {
            return self.read_bracket_identifier();
        }

        // Double-quoted identifier "name"
        if ch == '"' {
            return self.read_quoted_identifier();
        }

        // Identifier or keyword
        if ch.is_alphabetic() || ch == '_' || ch == '#' {
            return self.read_identifier_or_keyword();
        }

        // Operators and punctuation
        self.read_operator()
    }

    fn read_string_literal(&mut self, is_national: bool) -> TsqlResult<TsqlToken> {
        self.advance(); // consume opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\'') => {
                    // Check for escaped quote ''
                    if self.peek() == Some('\'') {
                        self.advance();
                        s.push('\'');
                    } else {
                        break;
                    }
                }
                Some(c) => s.push(c),
                None => return Err(self.err("unterminated string literal")),
            }
        }
        if is_national {
            Ok(TsqlToken::NStringLiteral(s))
        } else {
            Ok(TsqlToken::StringLiteral(s))
        }
    }

    fn read_binary_literal(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // '0'
        self.advance(); // 'x'
        let mut hex = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() {
                hex.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| {
                let end = (i + 2).min(hex.len());
                u8::from_str_radix(&hex[i..end], 16).unwrap_or(0)
            })
            .collect();
        Ok(TsqlToken::BinaryLiteral(bytes))
    }

    fn read_money_literal(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // '$'
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Ok(TsqlToken::MoneyLiteral(s))
    }

    fn read_variable(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // first '@'
        if self.peek() == Some('@') {
            // @@system_variable
            self.advance();
            let mut name = String::new();
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            Ok(TsqlToken::AtAtVariable(name))
        } else {
            // @local_variable
            let mut name = String::new();
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            Ok(TsqlToken::AtVariable(name))
        }
    }

    fn read_temp_table(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // first '#'
        if self.peek() == Some('#') {
            self.advance();
            let mut name = String::new();
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            Ok(TsqlToken::DoublePoundTemp(name))
        } else {
            let mut name = String::new();
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    name.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            Ok(TsqlToken::PoundTemp(name))
        }
    }

    fn read_number(&mut self) -> TsqlResult<TsqlToken> {
        let mut s = String::new();
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                is_float = true;
                s.push(c);
                self.advance();
            } else if (c == 'e' || c == 'E') && !s.is_empty() {
                is_float = true;
                s.push(c);
                self.advance();
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        s.push(sign);
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }

        if is_float {
            let val: f64 = s.parse().map_err(|_| self.err(format!("invalid float: {}", s)))?;
            Ok(TsqlToken::FloatLiteral(val))
        } else {
            let val: i64 = s.parse().map_err(|_| self.err(format!("invalid integer: {}", s)))?;
            Ok(TsqlToken::IntLiteral(val))
        }
    }

    fn read_bracket_identifier(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // '['
        let mut name = String::new();
        loop {
            match self.advance() {
                Some(']') => break,
                Some(c) => name.push(c),
                None => return Err(self.err("unterminated bracket identifier")),
            }
        }
        Ok(TsqlToken::QuotedIdentifier(name))
    }

    fn read_quoted_identifier(&mut self) -> TsqlResult<TsqlToken> {
        self.advance(); // '"'
        let mut name = String::new();
        loop {
            match self.advance() {
                Some('"') => {
                    if self.peek() == Some('"') {
                        self.advance();
                        name.push('"');
                    } else {
                        break;
                    }
                }
                Some(c) => name.push(c),
                None => return Err(self.err("unterminated quoted identifier")),
            }
        }
        Ok(TsqlToken::QuotedIdentifier(name))
    }

    fn read_identifier_or_keyword(&mut self) -> TsqlResult<TsqlToken> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '@' || c == '#' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let upper = name.to_uppercase();
        let token = match upper.as_str() {
            "SELECT" => TsqlToken::Select,
            "FROM" => TsqlToken::From,
            "WHERE" => TsqlToken::Where,
            "INSERT" => TsqlToken::Insert,
            "INTO" => TsqlToken::Into,
            "UPDATE" => TsqlToken::Update,
            "DELETE" => TsqlToken::Delete,
            "CREATE" => TsqlToken::Create,
            "ALTER" => TsqlToken::Alter,
            "DROP" => TsqlToken::Drop,
            "TABLE" => TsqlToken::Table,
            "DATABASE" => TsqlToken::Database,
            "VIEW" => TsqlToken::View,
            "INDEX" => TsqlToken::Index,
            "PROCEDURE" | "PROC" => {
                if upper == "PROCEDURE" {
                    TsqlToken::Procedure
                } else {
                    TsqlToken::Proc
                }
            }
            "FUNCTION" => TsqlToken::Function,
            "TRIGGER" => TsqlToken::Trigger,
            "BEGIN" => TsqlToken::Begin,
            "END" => TsqlToken::End,
            "IF" => TsqlToken::If,
            "ELSE" => TsqlToken::Else,
            "WHILE" => TsqlToken::While,
            "RETURN" => TsqlToken::Return,
            "RETURNS" => TsqlToken::Returns,
            "GOTO" => TsqlToken::Goto,
            "WAITFOR" => TsqlToken::WaitFor,
            "DECLARE" => TsqlToken::Declare,
            "SET" => TsqlToken::Set,
            "PRINT" => TsqlToken::Print,
            "EXECUTE" | "EXEC" => {
                if upper == "EXECUTE" {
                    TsqlToken::Execute
                } else {
                    TsqlToken::Exec
                }
            }
            "TRY" => TsqlToken::Try,
            "CATCH" => TsqlToken::Catch,
            "THROW" => TsqlToken::Throw,
            "RAISERROR" => TsqlToken::Raiserror,
            "CURSOR" => TsqlToken::Cursor,
            "OPEN" => TsqlToken::Open,
            "FETCH" => TsqlToken::Fetch,
            "CLOSE" => TsqlToken::Close,
            "DEALLOCATE" => TsqlToken::Deallocate,
            "COMMIT" => TsqlToken::Commit,
            "ROLLBACK" => TsqlToken::Rollback,
            "SAVE" => TsqlToken::Save,
            "TRANSACTION" => TsqlToken::Transaction,
            "TRAN" => TsqlToken::Tran,
            "GO" => TsqlToken::Go,
            "USE" => TsqlToken::Use,
            "SCHEMA" => TsqlToken::Schema,
            "AND" => TsqlToken::And,
            "OR" => TsqlToken::Or,
            "NOT" => TsqlToken::Not,
            "IN" => TsqlToken::In,
            "BETWEEN" => TsqlToken::Between,
            "LIKE" => TsqlToken::Like,
            "IS" => TsqlToken::Is,
            "NULL" => TsqlToken::Null,
            "EXISTS" => TsqlToken::Exists,
            "CASE" => TsqlToken::Case,
            "WHEN" => TsqlToken::When,
            "THEN" => TsqlToken::Then,
            "AS" => TsqlToken::As,
            "ON" => TsqlToken::On,
            "JOIN" => TsqlToken::Join,
            "LEFT" => TsqlToken::Left,
            "RIGHT" => TsqlToken::Right,
            "INNER" => TsqlToken::Inner,
            "OUTER" => TsqlToken::Outer,
            "FULL" => TsqlToken::Full,
            "CROSS" => TsqlToken::Cross,
            "UNION" => TsqlToken::Union,
            "ALL" => TsqlToken::All,
            "ANY" => TsqlToken::Any,
            "SOME" => TsqlToken::Some,
            "HAVING" => TsqlToken::Having,
            "GROUP" => TsqlToken::Group,
            "ORDER" => TsqlToken::Order,
            "BY" => TsqlToken::By,
            "ASC" => TsqlToken::Asc,
            "DESC" => TsqlToken::Desc,
            "TOP" => TsqlToken::Top,
            "PERCENT" => TsqlToken::Percent,
            "WITH" => TsqlToken::With,
            "TIES" => TsqlToken::Ties,
            "DISTINCT" => TsqlToken::Distinct,
            "COMPUTE" => TsqlToken::Compute,
            "BROWSE" => TsqlToken::Browse,
            "FOR" => TsqlToken::For,
            "OPTION" => TsqlToken::Option,
            "OUTPUT" => TsqlToken::Output,
            "DEFAULT" => TsqlToken::Default,
            "VALUES" => TsqlToken::Values,
            "NULLS" => TsqlToken::Nulls,
            "FIRST" => TsqlToken::First,
            "LAST" => TsqlToken::Last,
            "PRIOR" => TsqlToken::Prior,
            "NEXT" => TsqlToken::Next,
            "ABSOLUTE" => TsqlToken::Absolute,
            "RELATIVE" => TsqlToken::Relative,
            "FORWARD_ONLY" => TsqlToken::ForwardOnly,
            "SCROLL" => TsqlToken::Scroll,
            "KEYSET" => TsqlToken::Keyset,
            "DYNAMIC" => TsqlToken::Dynamic,
            "STATIC" => TsqlToken::Static,
            "INSENSITIVE" => TsqlToken::Insensitive,
            "SENSITIVE" => TsqlToken::Sensitive,
            "READONLY" | "READ_ONLY" => TsqlToken::ReadOnly,
            "MERGE" => TsqlToken::Merge,
            "MATCHED" => TsqlToken::Matched,
            "USING" => TsqlToken::Using,
            "CONSTRAINT" => TsqlToken::Constraint,
            "PRIMARY" => TsqlToken::Primary,
            "KEY" => TsqlToken::Key,
            "FOREIGN" => TsqlToken::Foreign,
            "REFERENCES" => TsqlToken::References,
            "UNIQUE" => TsqlToken::Unique,
            "CHECK" => TsqlToken::Check,
            "CLUSTERED" => TsqlToken::Clustered,
            "IDENTITY" => TsqlToken::Identity,
            "TRUNCATE" => TsqlToken::Truncate,
            "ADD" => TsqlToken::Add,
            "COLUMN" => TsqlToken::Column,
            "ENABLE" => TsqlToken::Enable,
            "DISABLE" => TsqlToken::Disable,
            "DELAY" => TsqlToken::Delay,
            "TIME" => TsqlToken::Time,
            "BREAK" => TsqlToken::Break,
            "CONTINUE" => TsqlToken::Continue,
            "OVER" => TsqlToken::Over,
            "PARTITION" => TsqlToken::Partition,
            "ROW" => TsqlToken::Row,
            "ROWS" => TsqlToken::Rows,
            "RANGE" => TsqlToken::Range,
            "PRECEDING" => TsqlToken::Preceding,
            "FOLLOWING" => TsqlToken::Following,
            "UNBOUNDED" => TsqlToken::Unbounded,
            "CURRENT" => TsqlToken::Current,
            "OF" => TsqlToken::Of,
            "OFFSET" => TsqlToken::Offset,
            "ROWCOUNT" => TsqlToken::RowCount,
            "NOCOUNT" => TsqlToken::NoCount,
            "LANGUAGE" => TsqlToken::Language,
            "ENCRYPTION" | "ENCRYPTED" => TsqlToken::Encrypted,
            "RECOMPILE" => TsqlToken::Recompile,
            _ => TsqlToken::Identifier(name),
        };
        Ok(token)
    }

    fn read_operator(&mut self) -> TsqlResult<TsqlToken> {
        let ch = self.advance().unwrap();
        match ch {
            '(' => Ok(TsqlToken::LeftParen),
            ')' => Ok(TsqlToken::RightParen),
            '{' => Ok(TsqlToken::LeftBrace),
            '}' => Ok(TsqlToken::RightBrace),
            ',' => Ok(TsqlToken::Comma),
            ';' => Ok(TsqlToken::Semicolon),
            '~' => Ok(TsqlToken::Tilde),
            '.' => Ok(TsqlToken::Dot),
            '+' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::PlusAssign)
                }
                _ => Ok(TsqlToken::Plus),
            },
            '-' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::MinusAssign)
                }
                _ => Ok(TsqlToken::Minus),
            },
            '*' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::StarEquals)
                }
                _ => Ok(TsqlToken::Star),
            },
            '/' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::SlashEquals)
                }
                _ => Ok(TsqlToken::Slash),
            },
            '%' => Ok(TsqlToken::PercentOp),
            '=' => match self.peek() {
                Some('*') => {
                    self.advance();
                    Ok(TsqlToken::RightOuterJoin) // =*
                }
                _ => Ok(TsqlToken::Equals),
            },
            '<' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::LtEq)
                }
                Some('>') => {
                    self.advance();
                    Ok(TsqlToken::NotEquals) // <>
                }
                _ => Ok(TsqlToken::Lt),
            },
            '>' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::GtEq)
                }
                _ => Ok(TsqlToken::Gt),
            },
            '!' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::NotEquals) // !=
                }
                Some('<') => {
                    self.advance();
                    Ok(TsqlToken::NotEquals) // !<
                }
                Some('>') => {
                    self.advance();
                    Ok(TsqlToken::NotEquals) // !>
                }
                _ => Err(self.err("unexpected character: !")),
            },
            '&' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::AndAssign)
                }
                _ => Ok(TsqlToken::Ampersand),
            },
            '|' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::OrAssign)
                }
                _ => Ok(TsqlToken::Pipe),
            },
            '^' => match self.peek() {
                Some('=') => {
                    self.advance();
                    Ok(TsqlToken::XorAssign)
                }
                _ => Ok(TsqlToken::Caret),
            },
            ':' => match self.peek() {
                Some(':') => {
                    self.advance();
                    Ok(TsqlToken::DoubleColon)
                }
                _ => Ok(TsqlToken::Colon),
            },
            '#' => {
                // Handle standalone # (shouldn't normally reach here)
                self.read_temp_table()
            }
            _ => Err(self.err(format!("unexpected character: {}", ch))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let mut lexer = TsqlLexer::new("SELECT 1");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::Select);
        assert_eq!(tokens[1], TsqlToken::IntLiteral(1));
        assert_eq!(tokens[2], TsqlToken::Eof);
    }

    #[test]
    fn test_variables() {
        let mut lexer = TsqlLexer::new("@myvar @@ROWCOUNT");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::AtVariable("myvar".to_string()));
        assert_eq!(tokens[1], TsqlToken::AtAtVariable("ROWCOUNT".to_string()));
    }

    #[test]
    fn test_temp_tables() {
        let mut lexer = TsqlLexer::new("#temp ##global");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::PoundTemp("temp".to_string()));
        assert_eq!(tokens[1], TsqlToken::DoublePoundTemp("global".to_string()));
    }

    #[test]
    fn test_money_literal() {
        let mut lexer = TsqlLexer::new("$123.45");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::MoneyLiteral("123.45".to_string()));
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = TsqlLexer::new("'hello' N'world'");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::StringLiteral("hello".to_string()));
        assert_eq!(tokens[1], TsqlToken::NStringLiteral("world".to_string()));
    }

    #[test]
    fn test_bracket_identifier() {
        let mut lexer = TsqlLexer::new("[my column]");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::QuotedIdentifier("my column".to_string()));
    }

    #[test]
    fn test_operators() {
        let mut lexer = TsqlLexer::new("=* *= <> !=");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::RightOuterJoin);
        assert_eq!(tokens[1], TsqlToken::StarEquals);
        assert_eq!(tokens[2], TsqlToken::NotEquals);
        assert_eq!(tokens[3], TsqlToken::NotEquals);
    }

    #[test]
    fn test_comments() {
        let mut lexer = TsqlLexer::new("SELECT -- comment\n1 /* block */ 2");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0], TsqlToken::Select);
        assert_eq!(tokens[1], TsqlToken::IntLiteral(1));
        assert_eq!(tokens[2], TsqlToken::IntLiteral(2));
    }
}
