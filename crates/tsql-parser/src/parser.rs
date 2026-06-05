//! T-SQL Recursive Descent Parser
//!
//! Parses T-SQL tokens into an AST. Supports the full SAP ASE 16 T-SQL dialect
//! including stored procedures, control flow, cursors, error handling, and
//! all SAP ASE-specific syntax.

use crate::ast::*;
use crate::error::{TsqlParseError, TsqlResult};
use crate::lexer::{TsqlLexer, TsqlToken};

pub struct TsqlParser {
    tokens: Vec<TsqlToken>,
    pos: usize,
}

impl TsqlParser {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            pos: 0,
        }
    }

    /// Parse a T-SQL string into a list of statements.
    pub fn parse(&mut self, input: &str) -> TsqlResult<Vec<TsqlStatement>> {
        let mut lexer = TsqlLexer::new(input);
        self.tokens = lexer.tokenize()?;
        self.pos = 0;

        let mut stmts = Vec::new();
        while !self.is_at_end() {
            self.skip_semicolons();
            if self.is_at_end() {
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_semicolons();
        }
        Ok(stmts)
    }

    // ── Token Helpers ──

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos] == TsqlToken::Eof
    }

    fn peek(&self) -> &TsqlToken {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &TsqlToken::Eof
        }
    }

    fn peek_ahead(&self, offset: usize) -> &TsqlToken {
        let idx = self.pos + offset;
        if idx < self.tokens.len() {
            &self.tokens[idx]
        } else {
            &TsqlToken::Eof
        }
    }

    fn advance(&mut self) -> TsqlToken {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            TsqlToken::Eof
        }
    }

    fn expect(&mut self, expected: &TsqlToken) -> TsqlResult<TsqlToken> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(self.err_at(format!("expected {:?}, found {:?}", expected, self.peek())))
        }
    }

    fn expect_identifier(&mut self) -> TsqlResult<String> {
        match self.advance() {
            TsqlToken::Identifier(name) => Ok(name),
            TsqlToken::QuotedIdentifier(name) => Ok(name),
            // Allow keywords to be used as identifiers in certain contexts
            tok if tok.is_keyword() => Ok(self.token_to_string(&tok)),
            other => Err(self.err_at(format!("expected identifier, found {:?}", other))),
        }
    }

    fn check(&self, expected: &TsqlToken) -> bool {
        self.peek() == expected
    }

    #[allow(dead_code)]
    fn check_keyword(&self) -> bool {
        self.peek().is_keyword()
    }

    fn match_token(&mut self, expected: &TsqlToken) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn match_any(&mut self, tokens: &[TsqlToken]) -> bool {
        for t in tokens {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn skip_semicolons(&mut self) {
        while self.match_token(&TsqlToken::Semicolon) {}
    }

    /// Check if current token is an identifier or quoted identifier and return its value.
    #[allow(dead_code)]
    fn peek_identifier_value(&self) -> Option<String> {
        match self.peek() {
            TsqlToken::Identifier(s) | TsqlToken::QuotedIdentifier(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Check if current token is a specific identifier (case-insensitive).
    #[allow(dead_code)]
    fn check_identifier_ci(&self, expected: &str) -> bool {
        match self.peek() {
            TsqlToken::Identifier(s) | TsqlToken::QuotedIdentifier(s) => {
                s.eq_ignore_ascii_case(expected)
            }
            _ => false,
        }
    }

    /// Check if current token is an identifier matching the given name (case-insensitive).
    fn peek_ident_ci(&self, name: &str) -> bool {
        match self.peek() {
            TsqlToken::Identifier(s) | TsqlToken::QuotedIdentifier(s) => {
                s.eq_ignore_ascii_case(name)
            }
            _ => false,
        }
    }

    /// Check if current token is any identifier (Identifier or QuotedIdentifier).
    fn peek_is_ident(&self) -> bool {
        matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::QuotedIdentifier(_))
    }

    /// Check if current token is an @variable.
    fn peek_is_at_variable(&self) -> bool {
        matches!(self.peek(), TsqlToken::AtVariable(_))
    }

    /// Consume current token as a string value.
    fn consume_token_string(&mut self) -> String {
        let tok = self.advance();
        self.token_to_string_owned(&tok)
    }

    fn err_at(&self, msg: impl Into<String>) -> TsqlParseError {
        TsqlParseError::new(msg, self.pos, 0, 0)
    }

    fn token_to_string(&self, tok: &TsqlToken) -> String {
        self.token_to_string_owned(tok)
    }

    fn token_to_string_owned(&self, tok: &TsqlToken) -> String {
        match tok {
            TsqlToken::Identifier(s) | TsqlToken::QuotedIdentifier(s) => s.clone(),
            TsqlToken::Select => "SELECT".to_string(),
            TsqlToken::From => "FROM".to_string(),
            TsqlToken::Where => "WHERE".to_string(),
            TsqlToken::Table => "TABLE".to_string(),
            TsqlToken::Index => "INDEX".to_string(),
            TsqlToken::View => "VIEW".to_string(),
            TsqlToken::Procedure => "PROCEDURE".to_string(),
            TsqlToken::Proc => "PROC".to_string(),
            TsqlToken::Function => "FUNCTION".to_string(),
            TsqlToken::Trigger => "TRIGGER".to_string(),
            TsqlToken::Begin => "BEGIN".to_string(),
            TsqlToken::End => "END".to_string(),
            TsqlToken::If => "IF".to_string(),
            TsqlToken::Else => "ELSE".to_string(),
            TsqlToken::While => "WHILE".to_string(),
            TsqlToken::Return => "RETURN".to_string(),
            TsqlToken::Declare => "DECLARE".to_string(),
            TsqlToken::Set => "SET".to_string(),
            TsqlToken::Print => "PRINT".to_string(),
            TsqlToken::Execute => "EXECUTE".to_string(),
            TsqlToken::Exec => "EXEC".to_string(),
            TsqlToken::Cursor => "CURSOR".to_string(),
            TsqlToken::Open => "OPEN".to_string(),
            TsqlToken::Fetch => "FETCH".to_string(),
            TsqlToken::Close => "CLOSE".to_string(),
            TsqlToken::Deallocate => "DEALLOCATE".to_string(),
            TsqlToken::Commit => "COMMIT".to_string(),
            TsqlToken::Rollback => "ROLLBACK".to_string(),
            TsqlToken::Save => "SAVE".to_string(),
            TsqlToken::Transaction => "TRANSACTION".to_string(),
            TsqlToken::Tran => "TRAN".to_string(),
            TsqlToken::Go => "GO".to_string(),
            TsqlToken::Use => "USE".to_string(),
            TsqlToken::Database => "DATABASE".to_string(),
            TsqlToken::Try => "TRY".to_string(),
            TsqlToken::Catch => "CATCH".to_string(),
            TsqlToken::Throw => "THROW".to_string(),
            TsqlToken::Raiserror => "RAISERROR".to_string(),
            TsqlToken::Break => "BREAK".to_string(),
            TsqlToken::Continue => "CONTINUE".to_string(),
            TsqlToken::Goto => "GOTO".to_string(),
            TsqlToken::WaitFor => "WAITFOR".to_string(),
            TsqlToken::Create => "CREATE".to_string(),
            TsqlToken::Alter => "ALTER".to_string(),
            TsqlToken::Drop => "DROP".to_string(),
            TsqlToken::Insert => "INSERT".to_string(),
            TsqlToken::Update => "UPDATE".to_string(),
            TsqlToken::Delete => "DELETE".to_string(),
            TsqlToken::Merge => "MERGE".to_string(),
            TsqlToken::Truncate => "TRUNCATE".to_string(),
            _ => format!("{:?}", tok),
        }
    }

    // ── Statement Parsing ──

    fn parse_statement(&mut self) -> TsqlResult<TsqlStatement> {
        match self.peek().clone() {
            TsqlToken::Select => self.parse_select_statement(),
            TsqlToken::Insert => self.parse_insert(),
            TsqlToken::Update => self.parse_update(),
            TsqlToken::Delete => self.parse_delete(),
            TsqlToken::Merge => self.parse_merge(),
            TsqlToken::Create => self.parse_create(),
            TsqlToken::Alter => self.parse_alter(),
            TsqlToken::Drop => self.parse_drop(),
            TsqlToken::Truncate => self.parse_truncate(),
            TsqlToken::Begin => self.parse_begin(),
            TsqlToken::If => self.parse_if(),
            TsqlToken::While => self.parse_while(),
            TsqlToken::Return => self.parse_return(),
            TsqlToken::Goto => self.parse_goto(),
            TsqlToken::WaitFor => self.parse_waitfor(),
            TsqlToken::Break => { self.advance(); Ok(TsqlStatement::Break) }
            TsqlToken::Continue => { self.advance(); Ok(TsqlStatement::Continue) }
            TsqlToken::Declare => self.parse_declare(),
            TsqlToken::Set => self.parse_set(),
            TsqlToken::Print => self.parse_print(),
            TsqlToken::Execute | TsqlToken::Exec => self.parse_execute(),
            TsqlToken::Open => self.parse_open_cursor(),
            TsqlToken::Fetch => self.parse_fetch_cursor(),
            TsqlToken::Close => self.parse_close_cursor(),
            TsqlToken::Deallocate => self.parse_deallocate_cursor(),
            TsqlToken::Commit => self.parse_commit(),
            TsqlToken::Rollback => self.parse_rollback(),
            TsqlToken::Save => self.parse_save_transaction(),
            TsqlToken::Throw => self.parse_throw(),
            TsqlToken::Raiserror => self.parse_raiserror(),
            TsqlToken::Use => self.parse_use(),
            TsqlToken::AtVariable(name) => {
                // @var = expr (shorthand assignment without SET)
                // or @label: (label definition)
                if *self.peek_ahead(1) == TsqlToken::Colon {
                    self.advance(); // consume @var
                    self.advance(); // consume :
                    Ok(TsqlStatement::Label(name))
                } else {
                    self.parse_select_into_vars()
                }
            }
            TsqlToken::PoundTemp(_) | TsqlToken::DoublePoundTemp(_) => {
                // #temp table creation
                self.parse_create_temp_table()
            }
            // Try to detect label: identifier followed by colon
            TsqlToken::Identifier(_) => {
                if *self.peek_ahead(1) == TsqlToken::Colon {
                    let name = self.expect_identifier()?;
                    self.advance(); // consume ':'
                    Ok(TsqlStatement::Label(name))
                } else {
                    // Could be a procedure call without EXEC
                    // or a statement starting with an identifier
                    self.parse_identifier_statement()
                }
            }
            TsqlToken::LeftParen => {
                // Subquery as statement
                self.parse_select_statement()
            }
            TsqlToken::Semicolon => {
                self.advance();
                Ok(TsqlStatement::NoOp)
            }
            _ => {
                // Collect as passthrough SQL until semicolon or end
                self.parse_passthrough()
            }
        }
    }

    // ── SELECT ──

    fn parse_select_statement(&mut self) -> TsqlResult<TsqlStatement> {
        // Peek ahead past SELECT [DISTINCT] [TOP ...] to detect @var = pattern
        let mut peek_pos = 1; // past SELECT
        if peek_pos < self.tokens.len() && self.tokens[peek_pos] == TsqlToken::Distinct {
            peek_pos += 1;
        }
        if peek_pos < self.tokens.len() && self.tokens[peek_pos] == TsqlToken::Top {
            peek_pos += 1;
            // Skip TOP clause contents (simplified: skip to next non-paren token)
            while peek_pos < self.tokens.len()
                && self.tokens[peek_pos] != TsqlToken::From
                && self.tokens[peek_pos] != TsqlToken::AtVariable(String::new())
                && !matches!(self.tokens[peek_pos], TsqlToken::IntLiteral(_))
            {
                peek_pos += 1;
            }
        }
        if peek_pos < self.tokens.len() {
            if let TsqlToken::AtVariable(_) = &self.tokens[peek_pos] {
                if peek_pos + 1 < self.tokens.len() && self.tokens[peek_pos + 1] == TsqlToken::Equals {
                    // This is SELECT @var = expr ... (SELECT INTO VARS)
                    return self.parse_select_into_vars_statement();
                }
            }
        }

        let sel = self.parse_select()?;
        Ok(TsqlStatement::Select(sel))
    }

    fn parse_select_into_vars_statement(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Select)?;

        let mut assignments = Vec::new();
        loop {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => return Err(self.err_at("expected @variable")),
            };
            self.expect(&TsqlToken::Equals)?;
            let expr = self.parse_expr()?;
            assignments.push((name, expr));
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
            if !self.peek_is_at_variable() {
                break;
            }
            if *self.peek_ahead(1) != TsqlToken::Equals {
                break;
            }
        }

        let from = if self.match_token(&TsqlToken::From) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };
        let where_clause = if self.match_token(&TsqlToken::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let group_by = if self.match_token(&TsqlToken::Group) {
            self.expect(&TsqlToken::By)?;
            self.parse_expr_list()?
        } else {
            Vec::new()
        };
        let having = if self.match_token(&TsqlToken::Having) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let order_by = if self.match_token(&TsqlToken::Order) {
            self.expect(&TsqlToken::By)?;
            self.parse_order_by_list()?
        } else {
            Vec::new()
        };

        Ok(TsqlStatement::SelectIntoVars(SelectIntoVarsStmt {
            assignments,
            from,
            where_clause,
            group_by,
            having,
            order_by,
        }))
    }

    fn parse_select(&mut self) -> TsqlResult<TsqlSelect> {
        self.expect(&TsqlToken::Select)?;

        let distinct = self.match_token(&TsqlToken::Distinct);

        // TOP clause
        let top = if self.match_token(&TsqlToken::Top) {
            Some(self.parse_top_clause()?)
        } else {
            None
        };

        // Select list
        let select_list = self.parse_select_list()?;

        // INTO #temp
        let into_table = if self.match_token(&TsqlToken::Into) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        // FROM
        let from = if self.match_token(&TsqlToken::From) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        // WHERE
        let where_clause = if self.match_token(&TsqlToken::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // GROUP BY
        let group_by = if self.match_token(&TsqlToken::Group) {
            self.expect(&TsqlToken::By)?;
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        // HAVING
        let having = if self.match_token(&TsqlToken::Having) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        // ORDER BY
        let order_by = if self.match_token(&TsqlToken::Order) {
            self.expect(&TsqlToken::By)?;
            self.parse_order_by_list()?
        } else {
            Vec::new()
        };

        // COMPUTE
        let compute = if self.match_token(&TsqlToken::Compute) {
            Some(self.parse_compute_clause()?)
        } else {
            None
        };

        // FOR BROWSE
        let for_browse = if self.match_token(&TsqlToken::For) {
            self.match_token(&TsqlToken::Browse)
        } else {
            false
        };

        // UNION
        let (union, union_all) = if self.match_token(&TsqlToken::Union) {
            let all = self.match_token(&TsqlToken::All);
            (Some(Box::new(self.parse_select()?)), all)
        } else {
            (None, false)
        };

        Ok(TsqlSelect {
            distinct,
            top,
            select_list,
            into_table,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            compute,
            for_browse,
            option_hints: Vec::new(),
            union,
            union_all,
        })
    }

    fn parse_top_clause(&mut self) -> TsqlResult<TopClause> {
        let count = if self.match_token(&TsqlToken::LeftParen) {
            let expr = self.parse_expr()?;
            self.expect(&TsqlToken::RightParen)?;
            expr
        } else {
            match self.advance() {
                TsqlToken::IntLiteral(n) => TsqlExpr::Literal(TsqlLiteral::Int(n)),
                TsqlToken::AtVariable(name) => TsqlExpr::Variable(name),
                _ => return Err(self.err_at("expected TOP count")),
            }
        };

        let percent = self.match_token(&TsqlToken::Percent);
        let with_ties = if self.match_token(&TsqlToken::With) {
            self.expect(&TsqlToken::Ties)?;
            true
        } else {
            false
        };

        Ok(TopClause {
            count,
            percent,
            with_ties,
        })
    }

    fn parse_select_list(&mut self) -> TsqlResult<Vec<TsqlSelectItem>> {
        let mut items = Vec::new();
        loop {
            let expr = self.parse_expr()?;

            // Check for alias (AS or just identifier)
            let alias = if self.match_token(&TsqlToken::As) {
                Some(self.expect_identifier()?)
            } else if matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::QuotedIdentifier(_)) {
                if !self.is_statement_keyword(self.peek()) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                }
            } else {
                None
            };

            items.push(TsqlSelectItem { expr, alias });

            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        Ok(items)
    }

    fn parse_order_by_list(&mut self) -> TsqlResult<Vec<TsqlOrderBy>> {
        let mut list = Vec::new();
        loop {
            let expr = self.parse_expr()?;
            let ascending = if self.match_token(&TsqlToken::Desc) {
                false
            } else {
                self.match_token(&TsqlToken::Asc);
                true
            };
            list.push(TsqlOrderBy { expr, ascending });
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        Ok(list)
    }

    fn parse_compute_clause(&mut self) -> TsqlResult<ComputeClause> {
        let aggregates = self.parse_expr_list()?;
        let by = if self.match_token(&TsqlToken::By) {
            self.parse_expr_list()?
        } else {
            Vec::new()
        };
        Ok(ComputeClause { aggregates, by })
    }

    // ── Table References ──

    fn parse_table_ref(&mut self) -> TsqlResult<TsqlTableRef> {
        let left = self.parse_primary_table_ref()?;
        self.parse_joins(left)
    }

    fn parse_primary_table_ref(&mut self) -> TsqlResult<TsqlTableRef> {
        match self.peek().clone() {
            TsqlToken::LeftParen => {
                self.advance();
                // Subquery or derived table
                if self.check(&TsqlToken::Select) {
                    let query = self.parse_select()?;
                    self.expect(&TsqlToken::RightParen)?;
                    let alias = if self.match_token(&TsqlToken::As) {
                        self.expect_identifier()?
                    } else {
                        self.expect_identifier()?
                    };
                    Ok(TsqlTableRef::Subquery {
                        query: Box::new(query),
                        alias,
                    })
                } else {
                    let inner = self.parse_table_ref()?;
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(inner)
                }
            }
            TsqlToken::PoundTemp(name) => {
                self.advance();
                let alias = self.parse_optional_alias()?;
                Ok(TsqlTableRef::TempTable { name, alias })
            }
            TsqlToken::DoublePoundTemp(name) => {
                self.advance();
                let alias = self.parse_optional_alias()?;
                Ok(TsqlTableRef::TempTable {
                    name: format!("##{}", name),
                    alias,
                })
            }
            _ => {
                let name = self.expect_identifier()?;
                // Check for db.schema.table or schema.table
                let (database, schema, table_name) = if self.match_token(&TsqlToken::Dot) {
                    let second = self.expect_identifier()?;
                    if self.match_token(&TsqlToken::Dot) {
                        let third = self.expect_identifier()?;
                        (Some(name), Some(second), third)
                    } else {
                        (None, Some(name), second)
                    }
                } else {
                    (None, None, name)
                };

                let alias = self.parse_optional_alias()?;

                // Parse table hints: WITH (NOLOCK), (INDEX(idx)), etc.
                let hints = self.parse_table_hints()?;

                Ok(TsqlTableRef::Table {
                    database,
                    schema,
                    name: table_name,
                    alias,
                    hints,
                })
            }
        }
    }

    fn parse_optional_alias(&mut self) -> TsqlResult<Option<String>> {
        if self.match_token(&TsqlToken::As) {
            Ok(Some(self.expect_identifier()?))
        } else if matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::QuotedIdentifier(_)) {
            if !self.is_statement_keyword(self.peek()) {
                Ok(Some(self.expect_identifier()?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn parse_table_hints(&mut self) -> TsqlResult<Vec<String>> {
        let mut hints = Vec::new();
        if self.match_token(&TsqlToken::With) {
            self.expect(&TsqlToken::LeftParen)?;
            loop {
                let hint = self.expect_identifier()?;
                // Handle INDEX(idx1, idx2) or specific hint with args
                if self.match_token(&TsqlToken::LeftParen) {
                    let mut h = format!("{}(", hint);
                    let mut first = true;
                    while !self.check(&TsqlToken::RightParen) {
                        if !first {
                            h.push(',');
                        }
                        h.push_str(&self.expect_identifier()?);
                        first = false;
                    }
                    self.expect(&TsqlToken::RightParen)?;
                    h.push(')');
                    hints.push(h);
                } else {
                    hints.push(hint);
                }
                if !self.match_token(&TsqlToken::Comma) {
                    break;
                }
            }
            self.expect(&TsqlToken::RightParen)?;
        }
        Ok(hints)
    }

    fn parse_joins(&mut self, left: TsqlTableRef) -> TsqlResult<TsqlTableRef> {
        let mut result = left;
        loop {
            let join_type = match self.peek() {
                TsqlToken::Inner => {
                    self.advance();
                    self.expect(&TsqlToken::Join)?;
                    Some(TsqlJoinType::Inner)
                }
                TsqlToken::Left => {
                    self.advance();
                    self.match_token(&TsqlToken::Outer);
                    self.expect(&TsqlToken::Join)?;
                    Some(TsqlJoinType::LeftOuter)
                }
                TsqlToken::Right => {
                    self.advance();
                    self.match_token(&TsqlToken::Outer);
                    self.expect(&TsqlToken::Join)?;
                    Some(TsqlJoinType::RightOuter)
                }
                TsqlToken::Full => {
                    self.advance();
                    self.match_token(&TsqlToken::Outer);
                    self.expect(&TsqlToken::Join)?;
                    Some(TsqlJoinType::FullOuter)
                }
                TsqlToken::Cross => {
                    self.advance();
                    self.expect(&TsqlToken::Join)?;
                    Some(TsqlJoinType::Cross)
                }
                TsqlToken::Join => {
                    self.advance();
                    Some(TsqlJoinType::Inner)
                }
                _ => None,
            };

            if let Some(jt) = join_type {
                let right = self.parse_primary_table_ref()?;
                let condition = if jt != TsqlJoinType::Cross && self.match_token(&TsqlToken::On) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                result = TsqlTableRef::Join {
                    left: Box::new(result),
                    right: Box::new(right),
                    join_type: jt,
                    condition,
                };
            } else {
                break;
            }
        }
        Ok(result)
    }

    fn is_statement_keyword(&self, tok: &TsqlToken) -> bool {
        matches!(
            tok,
            TsqlToken::Select
                | TsqlToken::Insert
                | TsqlToken::Update
                | TsqlToken::Delete
                | TsqlToken::Create
                | TsqlToken::Alter
                | TsqlToken::Drop
                | TsqlToken::Begin
                | TsqlToken::If
                | TsqlToken::While
                | TsqlToken::Return
                | TsqlToken::Declare
                | TsqlToken::Set
                | TsqlToken::Print
                | TsqlToken::Execute
                | TsqlToken::Exec
                | TsqlToken::Open
                | TsqlToken::Fetch
                | TsqlToken::Close
                | TsqlToken::Deallocate
                | TsqlToken::Commit
                | TsqlToken::Rollback
                | TsqlToken::Where
                | TsqlToken::Group
                | TsqlToken::Having
                | TsqlToken::Order
                | TsqlToken::Union
                | TsqlToken::Compute
                | TsqlToken::From
                | TsqlToken::End
                | TsqlToken::Go
                | TsqlToken::On
                | TsqlToken::And
                | TsqlToken::Or
        )
    }

    // ── Expressions ──

    fn parse_expr(&mut self) -> TsqlResult<TsqlExpr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> TsqlResult<TsqlExpr> {
        let mut left = self.parse_and_expr()?;
        while self.match_token(&TsqlToken::Or) {
            let right = self.parse_and_expr()?;
            left = TsqlExpr::BinaryOp {
                left: Box::new(left),
                op: TsqlBinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> TsqlResult<TsqlExpr> {
        let mut left = self.parse_not_expr()?;
        while self.match_token(&TsqlToken::And) {
            let right = self.parse_not_expr()?;
            left = TsqlExpr::BinaryOp {
                left: Box::new(left),
                op: TsqlBinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> TsqlResult<TsqlExpr> {
        if self.match_token(&TsqlToken::Not) {
            let expr = self.parse_comparison()?;
            Ok(TsqlExpr::UnaryOp {
                op: TsqlUnaryOp::Not,
                expr: Box::new(expr),
            })
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> TsqlResult<TsqlExpr> {
        let left = self.parse_addition()?;

        // IS [NOT] NULL
        if self.match_token(&TsqlToken::Is) {
            let not = self.match_token(&TsqlToken::Not);
            self.expect(&TsqlToken::Null)?;
            let is_null = TsqlExpr::BinaryOp {
                left: Box::new(left),
                op: TsqlBinaryOp::Eq,
                right: Box::new(TsqlExpr::Literal(TsqlLiteral::Null)),
            };
            return if not {
                Ok(TsqlExpr::UnaryOp {
                    op: TsqlUnaryOp::Not,
                    expr: Box::new(is_null),
                })
            } else {
                Ok(is_null)
            };
        }

        // [NOT] IN (list) or [NOT] IN (subquery)
        let negated = if self.match_token(&TsqlToken::Not) { true } else { false };

        if self.match_token(&TsqlToken::In) {
            self.expect(&TsqlToken::LeftParen)?;
            if self.check(&TsqlToken::Select) {
                let query = self.parse_select()?;
                self.expect(&TsqlToken::RightParen)?;
                return Ok(TsqlExpr::InSubquery {
                    expr: Box::new(left),
                    query: Box::new(query),
                    negated,
                });
            } else {
                let list = self.parse_expr_list()?;
                self.expect(&TsqlToken::RightParen)?;
                return Ok(TsqlExpr::InList {
                    expr: Box::new(left),
                    list,
                    negated,
                });
            }
        }

        // [NOT] BETWEEN ... AND ...
        if self.match_token(&TsqlToken::Between) {
            let low = self.parse_addition()?;
            self.expect(&TsqlToken::And)?;
            let high = self.parse_addition()?;
            return Ok(TsqlExpr::Between {
                expr: Box::new(left),
                low: Box::new(low),
                high: Box::new(high),
                negated,
            });
        }

        // [NOT] LIKE pattern [ESCAPE char]
        if self.match_token(&TsqlToken::Like) {
            let pattern = self.parse_addition()?;
            let escape = if self.peek_ident_ci("ESCAPE") {
                self.advance();
                Some(Box::new(self.parse_addition()?))
            } else {
                None
            };
            return Ok(TsqlExpr::Like {
                expr: Box::new(left),
                pattern: Box::new(pattern),
                negated,
                escape,
            });
        }

        // If we consumed NOT but didn't match IN/BETWEEN/LIKE, we need to handle it
        // For now, comparison operators
        let op = match self.peek() {
            TsqlToken::Equals => Some(TsqlBinaryOp::Eq),
            TsqlToken::NotEquals => Some(TsqlBinaryOp::NotEq),
            TsqlToken::Lt => Some(TsqlBinaryOp::Lt),
            TsqlToken::Gt => Some(TsqlBinaryOp::Gt),
            TsqlToken::LtEq => Some(TsqlBinaryOp::LtEq),
            TsqlToken::GtEq => Some(TsqlBinaryOp::GtEq),
            TsqlToken::RightOuterJoin => Some(TsqlBinaryOp::Assign), // =* in expression context
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let right = self.parse_addition()?;

            // Check for old-style join operators: *= and =*
            if op == TsqlBinaryOp::Eq {
                if self.match_token(&TsqlToken::Star) {
                    return Ok(TsqlExpr::OldStyleJoin {
                        left: Box::new(left),
                        right: Box::new(right),
                        join_type: OldStyleJoinType::LeftOuter, // =* means left outer
                    });
                }
            }

            return Ok(TsqlExpr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        // If we consumed NOT, wrap in NOT
        if negated {
            return Err(self.err_at("NOT must be followed by IN, BETWEEN, LIKE, or EXISTS"));
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> TsqlResult<TsqlExpr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek() {
                TsqlToken::Plus => TsqlBinaryOp::Add,
                TsqlToken::Minus => TsqlBinaryOp::Subtract,
                TsqlToken::Ampersand => TsqlBinaryOp::BitwiseAnd,
                TsqlToken::Pipe => TsqlBinaryOp::BitwiseOr,
                TsqlToken::Caret => TsqlBinaryOp::BitwiseXor,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = TsqlExpr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> TsqlResult<TsqlExpr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TsqlToken::Star => TsqlBinaryOp::Multiply,
                TsqlToken::Slash => TsqlBinaryOp::Divide,
                TsqlToken::PercentOp => TsqlBinaryOp::Modulo,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = TsqlExpr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> TsqlResult<TsqlExpr> {
        match self.peek() {
            TsqlToken::Minus => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(TsqlExpr::UnaryOp {
                    op: TsqlUnaryOp::Negate,
                    expr: Box::new(expr),
                })
            }
            TsqlToken::Plus => {
                self.advance();
                self.parse_primary()
            }
            TsqlToken::Tilde => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(TsqlExpr::UnaryOp {
                    op: TsqlUnaryOp::BitwiseNot,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> TsqlResult<TsqlExpr> {
        match self.peek().clone() {
            // Literals
            TsqlToken::IntLiteral(n) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Int(n)))
            }
            TsqlToken::BigIntLiteral(n) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Int(n)))
            }
            TsqlToken::FloatLiteral(f) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Float(f)))
            }
            TsqlToken::StringLiteral(s) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::String(s)))
            }
            TsqlToken::NStringLiteral(s) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::String(s)))
            }
            TsqlToken::BinaryLiteral(b) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Binary(b)))
            }
            TsqlToken::MoneyLiteral(s) => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Money(s)))
            }
            TsqlToken::Null => {
                self.advance();
                Ok(TsqlExpr::Literal(TsqlLiteral::Null))
            }

            // Variables
            TsqlToken::AtVariable(name) => {
                self.advance();
                // Check for @var = expr (assignment in SELECT context)
                Ok(TsqlExpr::Variable(name))
            }
            TsqlToken::AtAtVariable(name) => {
                self.advance();
                Ok(TsqlExpr::SystemVariable(name))
            }

            // Wildcards
            TsqlToken::Star => {
                self.advance();
                Ok(TsqlExpr::Wildcard)
            }

            // Subquery
            TsqlToken::LeftParen => {
                self.advance();
                if self.check(&TsqlToken::Select) {
                    let query = self.parse_select()?;
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(TsqlExpr::Subquery(Box::new(query)))
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(expr)
                }
            }

            // CASE expression
            TsqlToken::Case => self.parse_case_expr(),

            // EXISTS
            TsqlToken::Exists => {
                self.advance();
                self.expect(&TsqlToken::LeftParen)?;
                let query = self.parse_select()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::Exists(Box::new(query)))
            }

            // Column reference or function call
            TsqlToken::Identifier(_) | TsqlToken::QuotedIdentifier(_) => {
                self.parse_column_ref_or_function()
            }

            // Temp table reference in expression
            TsqlToken::PoundTemp(name) => {
                self.advance();
                if self.match_token(&TsqlToken::Dot) {
                    let column = self.expect_identifier()?;
                    Ok(TsqlExpr::ColumnRef {
                        database: None,
                        schema: None,
                        table: Some(format!("#{}", name)),
                        column,
                    })
                } else {
                    Ok(TsqlExpr::ColumnRef {
                        database: None,
                        schema: None,
                        table: None,
                        column: format!("#{}", name),
                    })
                }
            }

            _ => Err(self.err_at(format!("unexpected token in expression: {:?}", self.peek()))),
        }
    }

    fn parse_column_ref_or_function(&mut self) -> TsqlResult<TsqlExpr> {
        let name = self.expect_identifier()?;
        let upper_name = name.to_uppercase();

        // Check for multi-part names: db.schema.table.column or schema.table.column
        if self.match_token(&TsqlToken::Dot) {
            let second = self.expect_identifier()?;
            if self.match_token(&TsqlToken::Dot) {
                let third = self.expect_identifier()?;
                if self.match_token(&TsqlToken::Dot) {
                    let fourth = self.expect_identifier()?;
                    return Ok(TsqlExpr::ColumnRef {
                        database: Some(name),
                        schema: Some(second),
                        table: Some(third),
                        column: fourth,
                    });
                }
                return Ok(TsqlExpr::ColumnRef {
                    database: None,
                    schema: None,
                    table: Some(second),
                    column: third,
                });
            }
            // Check for table.* wildcard
            if self.check(&TsqlToken::Star) {
                self.advance();
                return Ok(TsqlExpr::TableWildcard { table: name });
            }
            return Ok(TsqlExpr::ColumnRef {
                database: None,
                schema: None,
                table: Some(name),
                column: second,
            });
        }

        // Function call
        if self.check(&TsqlToken::LeftParen) {
            return self.parse_function_call(&name, &upper_name);
        }

        // Simple column reference
        Ok(TsqlExpr::ColumnRef {
            database: None,
            schema: None,
            table: None,
            column: name,
        })
    }

    fn parse_function_call(&mut self, _name: &str, upper_name: &str) -> TsqlResult<TsqlExpr> {
        self.expect(&TsqlToken::LeftParen)?;

        match upper_name {
            "CAST" => {
                let expr = self.parse_expr()?;
                self.expect(&TsqlToken::As)?;
                let dt = self.parse_tsql_data_type()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::Cast {
                    expr: Box::new(expr),
                    data_type: dt,
                })
            }
            "CONVERT" => {
                let dt = self.parse_tsql_data_type()?;
                self.expect(&TsqlToken::Comma)?;
                let expr = self.parse_expr()?;
                let style = if self.match_token(&TsqlToken::Comma) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::Convert {
                    data_type: dt,
                    expr: Box::new(expr),
                    style,
                })
            }
            "ISNULL" => {
                let expr = self.parse_expr()?;
                self.expect(&TsqlToken::Comma)?;
                let replacement = self.parse_expr()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::IsNull {
                    expr: Box::new(expr),
                    replacement: Box::new(replacement),
                })
            }
            "COALESCE" => {
                let args = self.parse_expr_list()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::Coalesce(args))
            }
            "NULLIF" => {
                let expr = self.parse_expr()?;
                self.expect(&TsqlToken::Comma)?;
                let other = self.parse_expr()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::NullIf {
                    expr: Box::new(expr),
                    other: Box::new(other),
                })
            }
            "TOP" => {
                let expr = self.parse_expr()?;
                let with_ties = false; // handled in select list context
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::Top {
                    count: Box::new(expr),
                    with_ties,
                })
            }
            _ => {
                // Generic function call
                let args = if self.check(&TsqlToken::RightParen) {
                    Vec::new()
                } else {
                    self.parse_expr_list()?
                };
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlExpr::FunctionCall {
                    name: _name.to_string(),
                    args,
                })
            }
        }
    }

    fn parse_case_expr(&mut self) -> TsqlResult<TsqlExpr> {
        self.expect(&TsqlToken::Case)?;

        // Simple CASE vs searched CASE
        let operand = if !self.check(&TsqlToken::When) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        let mut when_clauses = Vec::new();
        while self.match_token(&TsqlToken::When) {
            let when_expr = self.parse_expr()?;
            self.expect(&TsqlToken::Then)?;
            let then_expr = self.parse_expr()?;
            when_clauses.push((when_expr, then_expr));
        }

        let else_expr = if self.match_token(&TsqlToken::Else) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        self.expect(&TsqlToken::End)?;

        Ok(TsqlExpr::CaseWhen {
            operand,
            when_clauses,
            else_expr,
        })
    }

    fn parse_expr_list(&mut self) -> TsqlResult<Vec<TsqlExpr>> {
        let mut list = Vec::new();
        loop {
            list.push(self.parse_expr()?);
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        Ok(list)
    }

    // ── T-SQL Data Type Parsing ──

    fn parse_tsql_data_type(&mut self) -> TsqlResult<TsqlDataType> {
        let name = self.expect_identifier()?;
        let upper = name.to_uppercase();

        let base_type = match upper.as_str() {
            "INT" | "INTEGER" => TsqlDataType::Int,
            "SMALLINT" => TsqlDataType::SmallInt,
            "TINYINT" => TsqlDataType::TinyInt,
            "BIGINT" => TsqlDataType::BigInt,
            "BIT" => TsqlDataType::Bit,
            "MONEY" => TsqlDataType::Money,
            "SMALLMONEY" => TsqlDataType::SmallMoney,
            "REAL" => TsqlDataType::Real,
            "DATE" => TsqlDataType::Date,
            "DATETIME" => TsqlDataType::DateTime,
            "SMALLDATETIME" => TsqlDataType::SmallDateTime,
            "UNIQUEIDENTIFIER" => TsqlDataType::UniqueIdentifier,
            "XML" => TsqlDataType::Xml,
            "TEXT" => TsqlDataType::Text,
            "NTEXT" => TsqlDataType::NText,
            "IMAGE" => TsqlDataType::Image,
            "TABLE" => TsqlDataType::Table,
            "CURSOR" => TsqlDataType::CursorType,
            "SQL_VARIANT" => TsqlDataType::SqlVariant,
            "VARCHAR" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::Varchar(n)
            }
            "CHAR" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::Char(n)
            }
            "NVARCHAR" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::NVarchar(n)
            }
            "NCHAR" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::NChar(n)
            }
            "VARBINARY" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::VarBinary(n)
            }
            "BINARY" => {
                let n = self.parse_optional_size_param()?;
                TsqlDataType::Binary(n)
            }
            "DECIMAL" => {
                let (p, s) = self.parse_precision_scale_params()?;
                TsqlDataType::Decimal(p, s)
            }
            "NUMERIC" => {
                let (p, s) = self.parse_precision_scale_params()?;
                TsqlDataType::Numeric(p, s)
            }
            "FLOAT" => {
                let p = self.parse_optional_size_param()?.map(|n| n as u8);
                TsqlDataType::Float(p)
            }
            "DATETIME2" => {
                let p = self.parse_optional_size_param()?.map(|n| n as u8);
                TsqlDataType::DateTime2(p)
            }
            "DATETIMEOFFSET" => {
                let p = self.parse_optional_size_param()?.map(|n| n as u8);
                TsqlDataType::DateTimeOffset(p)
            }
            "TIME" => {
                let p = self.parse_optional_size_param()?.map(|n| n as u8);
                TsqlDataType::Time(p)
            }
            _ => TsqlDataType::UserDefined(name),
        };

        Ok(base_type)
    }

    fn parse_optional_size_param(&mut self) -> TsqlResult<Option<usize>> {
        if self.match_token(&TsqlToken::LeftParen) {
            match self.advance() {
                TsqlToken::Identifier(ref s) if s.to_uppercase() == "MAX" => {
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(Some(8000))
                }
                TsqlToken::IntLiteral(n) => {
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(Some(n as usize))
                }
                _ => {
                    self.expect(&TsqlToken::RightParen)?;
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn parse_precision_scale_params(&mut self) -> TsqlResult<(Option<u8>, Option<u8>)> {
        if self.match_token(&TsqlToken::LeftParen) {
            let p = match self.advance() {
                TsqlToken::IntLiteral(n) => Some(n as u8),
                _ => None,
            };
            let s = if self.match_token(&TsqlToken::Comma) {
                match self.advance() {
                    TsqlToken::IntLiteral(n) => Some(n as u8),
                    _ => None,
                }
            } else {
                None
            };
            self.expect(&TsqlToken::RightParen)?;
            Ok((p, s))
        } else {
            Ok((None, None))
        }
    }

    // ── INSERT ──

    fn parse_insert(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Insert)?;
        self.match_token(&TsqlToken::Into);

        let table = self.expect_identifier()?;

        // Optional column list
        let columns = if self.match_token(&TsqlToken::LeftParen) {
            let mut cols = Vec::new();
            loop {
                cols.push(self.expect_identifier()?);
                if !self.match_token(&TsqlToken::Comma) {
                    break;
                }
            }
            self.expect(&TsqlToken::RightParen)?;
            cols
        } else {
            Vec::new()
        };

        // Source
        let source = if self.check(&TsqlToken::Select) || self.check(&TsqlToken::LeftParen) {
            if self.check(&TsqlToken::LeftParen) {
                self.advance();
                let sel = self.parse_select()?;
                self.expect(&TsqlToken::RightParen)?;
                InsertSource::Select(Box::new(sel))
            } else {
                InsertSource::Select(Box::new(self.parse_select()?))
            }
        } else if self.match_token(&TsqlToken::Execute) || self.match_token(&TsqlToken::Exec) {
            let exec = self.parse_execute_body()?;
            InsertSource::Execute(Box::new(exec))
        } else if self.match_token(&TsqlToken::Default) {
            self.expect(&TsqlToken::Values)?;
            InsertSource::DefaultValues
        } else {
            self.expect(&TsqlToken::Values)?;
            let mut rows = Vec::new();
            loop {
                self.expect(&TsqlToken::LeftParen)?;
                let values = self.parse_expr_list()?;
                self.expect(&TsqlToken::RightParen)?;
                rows.push(values);
                if !self.match_token(&TsqlToken::Comma) {
                    break;
                }
            }
            InsertSource::Values(rows)
        };

        Ok(TsqlStatement::Insert(TsqlInsert {
            table,
            columns,
            source,
            output_clause: None,
        }))
    }

    // ── UPDATE ──

    fn parse_update(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Update)?;
        let table = self.expect_identifier()?;
        let alias = self.parse_optional_alias()?;

        self.expect(&TsqlToken::Set)?;

        let mut assignments = Vec::new();
        loop {
            let col = self.expect_identifier()?;
            self.expect(&TsqlToken::Equals)?;
            let value = self.parse_expr()?;
            assignments.push((col, value));
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        let from = if self.match_token(&TsqlToken::From) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let where_clause = if self.match_token(&TsqlToken::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(TsqlStatement::Update(TsqlUpdate {
            table,
            alias,
            assignments,
            from,
            where_clause,
            output_clause: None,
        }))
    }

    // ── DELETE ──

    fn parse_delete(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Delete)?;

        // Optional FROM
        self.match_token(&TsqlToken::From);

        let table = self.expect_identifier()?;
        let alias = self.parse_optional_alias()?;

        let from = if self.match_token(&TsqlToken::From) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let where_clause = if self.match_token(&TsqlToken::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(TsqlStatement::Delete(TsqlDelete {
            table,
            alias,
            from,
            where_clause,
            output_clause: None,
        }))
    }

    // ── MERGE (placeholder) ──

    fn parse_merge(&mut self) -> TsqlResult<TsqlStatement> {
        // Simplified MERGE parsing
        self.expect(&TsqlToken::Merge)?;
        self.expect(&TsqlToken::Into)?;
        let target = self.parse_table_ref()?;
        let target_alias = self.parse_optional_alias()?;

        self.expect(&TsqlToken::Using)?;
        let source = Box::new(self.parse_table_ref()?);
        let source_alias = self.parse_optional_alias()?;

        self.expect(&TsqlToken::On)?;
        let on_condition = self.parse_expr()?;

        Ok(TsqlStatement::Merge(TsqlMerge {
            target,
            target_alias,
            source,
            source_alias,
            on_condition,
            when_matched: None,
            when_not_matched: None,
            when_not_matched_by_source: None,
        }))
    }

    // ── CREATE ──

    fn parse_create(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Create)?;

        match self.peek().clone() {
            TsqlToken::Table => self.parse_create_table(),
            TsqlToken::Database => self.parse_create_database(),
            TsqlToken::View => self.parse_create_view(),
            TsqlToken::Index | TsqlToken::Unique | TsqlToken::Clustered => self.parse_create_index(),
            TsqlToken::Procedure | TsqlToken::Proc => self.parse_create_procedure(),
            _ if self.peek_ident_ci("NONCLUSTERED") => {
                self.parse_create_index()
            }
            _ => {
                // Collect rest as passthrough
                let mut sql = String::from("CREATE ");
                while !self.is_at_end() && !self.check(&TsqlToken::Go) && !self.check(&TsqlToken::Semicolon) {
                    let s = self.consume_token_string();
                    sql.push_str(&s);
                    sql.push(' ');
                }
                Ok(TsqlStatement::Passthrough(sql))
            }
        }
    }

    fn parse_create_table(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Table)?;
        let name = self.expect_identifier()?;

        self.expect(&TsqlToken::LeftParen)?;

        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        loop {
            // Check for table constraint
            if self.check(&TsqlToken::Primary) || self.check(&TsqlToken::Unique)
                || self.check(&TsqlToken::Foreign) || self.check(&TsqlToken::Check)
                || self.check(&TsqlToken::Constraint)
            {
                constraints.push(self.parse_table_constraint()?);
            } else {
                columns.push(self.parse_column_def()?);
            }
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        self.expect(&TsqlToken::RightParen)?;

        let is_temp = name.starts_with('#');

        let stmt = TsqlCreateTable {
            name: name.clone(),
            database: None,
            columns,
            constraints,
            on_clause: None,
            with_clause: None,
            text_image_on: None,
            lock_datapages: None,
            is_temp,
        };

        if is_temp {
            Ok(TsqlStatement::CreateTempTable(stmt))
        } else {
            Ok(TsqlStatement::CreateTable(stmt))
        }
    }

    fn parse_column_def(&mut self) -> TsqlResult<TsqlColumnDef> {
        let name = self.expect_identifier()?;
        let data_type = self.parse_tsql_data_type()?;

        let nullable = if self.match_token(&TsqlToken::Not) {
            self.expect(&TsqlToken::Null)?;
            Some(false)
        } else if self.match_token(&TsqlToken::Null) {
            Some(true)
        } else {
            None
        };

        let identity = if self.match_token(&TsqlToken::Identity) {
            let (seed, increment) = if self.match_token(&TsqlToken::LeftParen) {
                let seed = match self.advance() {
                    TsqlToken::IntLiteral(n) => n,
                    _ => return Err(self.err_at("expected identity seed")),
                };
                self.expect(&TsqlToken::Comma)?;
                let increment = match self.advance() {
                    TsqlToken::IntLiteral(n) => n,
                    _ => return Err(self.err_at("expected identity increment")),
                };
                self.expect(&TsqlToken::RightParen)?;
                (seed, increment)
            } else {
                (1, 1)
            };
            Some(IdentityDef {
                seed,
                increment,
                not_for_replication: false,
            })
        } else {
            None
        };

        let default = if self.match_token(&TsqlToken::Default) {
            Some(self.parse_primary()?)
        } else {
            None
        };

        Ok(TsqlColumnDef {
            name,
            data_type,
            nullable,
            default,
            identity,
            not_for_replication: false,
            constraint: None,
        })
    }

    fn parse_table_constraint(&mut self) -> TsqlResult<TsqlTableConstraint> {
        let name = if self.match_token(&TsqlToken::Constraint) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        match self.peek().clone() {
            TsqlToken::Primary => {
                self.advance();
                self.expect(&TsqlToken::Key)?;
                let _clustered = if self.match_token(&TsqlToken::Clustered) {
                    Some(true)
                } else if self.peek_ident_ci("NONCLUSTERED") {
                    self.advance();
                    Some(false)
                } else {
                    None
                };
                self.expect(&TsqlToken::LeftParen)?;
                let columns = self.parse_identifier_list()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlTableConstraint::PrimaryKey {
                    name,
                    columns,
                    clustered: _clustered,
                })
            }
            TsqlToken::Unique => {
                self.advance();
                self.expect(&TsqlToken::Key)?;
                self.expect(&TsqlToken::LeftParen)?;
                let columns = self.parse_identifier_list()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlTableConstraint::Unique {
                    name,
                    columns,
                    clustered: None,
                })
            }
            TsqlToken::Foreign => {
                self.advance();
                self.expect(&TsqlToken::Key)?;
                self.expect(&TsqlToken::LeftParen)?;
                let columns = self.parse_identifier_list()?;
                self.expect(&TsqlToken::RightParen)?;
                self.expect(&TsqlToken::References)?;
                let ref_table = self.expect_identifier()?;
                self.expect(&TsqlToken::LeftParen)?;
                let ref_columns = self.parse_identifier_list()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlTableConstraint::ForeignKey {
                    name,
                    columns,
                    ref_table,
                    ref_columns,
                })
            }
            TsqlToken::Check => {
                self.advance();
                self.expect(&TsqlToken::LeftParen)?;
                let expr = self.parse_expr()?;
                self.expect(&TsqlToken::RightParen)?;
                Ok(TsqlTableConstraint::Check { name, expr })
            }
            _ => Err(self.err_at("expected constraint keyword")),
        }
    }

    fn parse_identifier_list(&mut self) -> TsqlResult<Vec<String>> {
        let mut list = Vec::new();
        loop {
            list.push(self.expect_identifier()?);
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        Ok(list)
    }

    fn parse_create_database(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Database)?;
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::CreateDatabase(TsqlCreateDatabase {
            name,
            on_clause: None,
            log_on: None,
            with_clause: None,
        }))
    }

    fn parse_create_view(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::View)?;
        let name = self.expect_identifier()?;

        let columns = if self.match_token(&TsqlToken::LeftParen) {
            let cols = self.parse_identifier_list()?;
            self.expect(&TsqlToken::RightParen)?;
            cols
        } else {
            Vec::new()
        };

        self.expect(&TsqlToken::As)?;
        let query = self.parse_select()?;

        Ok(TsqlStatement::CreateView(TsqlCreateView {
            name,
            columns,
            query: Box::new(query),
            with_check: false,
            with_encryption: false,
        }))
    }

    fn parse_create_index(&mut self) -> TsqlResult<TsqlStatement> {
        let unique = self.match_token(&TsqlToken::Unique);
        let _clustered = if self.match_token(&TsqlToken::Clustered) {
            Some(true)
        } else if self.peek_ident_ci("NONCLUSTERED") {
            self.advance();
            Some(false)
        } else {
            None
        };

        self.expect(&TsqlToken::Index)?;
        let name = self.expect_identifier()?;
        self.expect(&TsqlToken::On)?;
        let table = self.expect_identifier()?;

        self.expect(&TsqlToken::LeftParen)?;
        let mut columns = Vec::new();
        loop {
            let col_name = self.expect_identifier()?;
            let ascending = if self.match_token(&TsqlToken::Desc) {
                false
            } else {
                self.match_token(&TsqlToken::Asc);
                true
            };
            columns.push(TsqlIndexColumn {
                name: col_name,
                ascending,
            });
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        self.expect(&TsqlToken::RightParen)?;

        Ok(TsqlStatement::CreateIndex(TsqlCreateIndex {
            name,
            table,
            columns,
            unique,
            clustered: _clustered,
            with_clause: None,
        }))
    }

    fn parse_create_procedure(&mut self) -> TsqlResult<TsqlStatement> {
        self.advance(); // PROCEDURE or PROC

        let name = self.expect_identifier()?;
        let (database, proc_name) = if self.match_token(&TsqlToken::Dot) {
            let second = self.expect_identifier()?;
            (Some(name), second)
        } else {
            (None, name)
        };

        // Parameters
        let params = self.parse_procedure_params()?;

        // WITH options
        let mut with_recompile = false;
        let mut with_encryption = false;
        if self.match_token(&TsqlToken::With) {
            loop {
                match self.peek() {
                    TsqlToken::Recompile => {
                        self.advance();
                        with_recompile = true;
                    }
                    TsqlToken::Encrypted => {
                        self.advance();
                        with_encryption = true;
                    }
                    _ => {
                        self.expect_identifier()?;
                    }
                }
                if !self.match_token(&TsqlToken::Comma) {
                    break;
                }
            }
        }

        self.expect(&TsqlToken::As)?;

        // Parse body until matching END
        let body = self.parse_procedure_body()?;

        Ok(TsqlStatement::CreateProcedure(CreateProcedureStmt {
            database,
            name: proc_name,
            params,
            body,
            with_recompile,
            with_encryption,
        }))
    }

    fn parse_procedure_params(&mut self) -> TsqlResult<Vec<ProcedureParam>> {
        let mut params = Vec::new();
        while self.peek_is_at_variable() {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => unreachable!(),
            };
            let data_type = self.parse_tsql_data_type()?;
            let output = self.match_token(&TsqlToken::Output);

            let default = if self.match_token(&TsqlToken::Equals) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            params.push(ProcedureParam {
                name,
                data_type,
                direction: if output { ParamDirection::Output } else { ParamDirection::Input },
                default,
                output,
            });

            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_procedure_body(&mut self) -> TsqlResult<Vec<TsqlStatement>> {
        let mut stmts = Vec::new();
        let mut depth = 0;

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for GO (end of batch/procedure)
            if self.check(&TsqlToken::Go) {
                self.advance();
                break;
            }

            self.skip_semicolons();
            if self.is_at_end() || self.check(&TsqlToken::Go) {
                break;
            }

            // Track BEGIN/END depth
            if self.check(&TsqlToken::Begin) {
                // Check if this is BEGIN TRY/CATCH or plain BEGIN
                if *self.peek_ahead(1) == TsqlToken::Try || *self.peek_ahead(1) == TsqlToken::Catch {
                    // Will be handled by parse_statement
                } else {
                    depth += 1;
                }
            }

            if self.check(&TsqlToken::End) && depth > 0 {
                depth -= 1;
            }

            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    // ── ALTER / DROP / TRUNCATE ──

    fn parse_alter(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Alter)?;
        match self.peek().clone() {
            TsqlToken::Table => {
                self.advance();
                let name = self.expect_identifier()?;
                let action = self.parse_alter_table_action()?;
                Ok(TsqlStatement::AlterTable(TsqlAlterTable { name, action }))
            }
            TsqlToken::Procedure | TsqlToken::Proc => {
                // ALTER PROCEDURE → treat same as CREATE for now
                self.advance();
                let name = self.expect_identifier()?;
                let params = self.parse_procedure_params()?;
                let mut with_recompile = false;
                let mut with_encryption = false;
                if self.match_token(&TsqlToken::With) {
                    loop {
                        match self.peek() {
                            TsqlToken::Recompile => { self.advance(); with_recompile = true; }
                            TsqlToken::Encrypted => { self.advance(); with_encryption = true; }
                            _ => { self.expect_identifier()?; }
                        }
                        if !self.match_token(&TsqlToken::Comma) { break; }
                    }
                }
                self.expect(&TsqlToken::As)?;
                let body = self.parse_procedure_body()?;
                Ok(TsqlStatement::AlterProcedure(AlterProcedureStmt {
                    database: None,
                    name,
                    params,
                    body,
                    with_recompile,
                    with_encryption,
                }))
            }
            _ => self.parse_passthrough_from("ALTER"),
        }
    }

    fn parse_alter_table_action(&mut self) -> TsqlResult<AlterTableAction> {
        if self.match_token(&TsqlToken::Add) {
            if self.check(&TsqlToken::Constraint) || self.check(&TsqlToken::Primary)
                || self.check(&TsqlToken::Unique) || self.check(&TsqlToken::Foreign)
                || self.check(&TsqlToken::Check)
            {
                Ok(AlterTableAction::AddConstraint(self.parse_table_constraint()?))
            } else {
                Ok(AlterTableAction::Add(self.parse_column_def()?))
            }
        } else if self.match_token(&TsqlToken::Drop) {
            if self.match_token(&TsqlToken::Constraint) {
                let name = self.expect_identifier()?;
                Ok(AlterTableAction::DropConstraint(name))
            } else {
                self.match_token(&TsqlToken::Column);
                let name = self.expect_identifier()?;
                Ok(AlterTableAction::DropColumn(name))
            }
        } else if self.match_token(&TsqlToken::Column) || (self.peek_is_ident() && self.peek().is_keyword()) {
            // ALTER COLUMN
            let col = self.parse_column_def()?;
            Ok(AlterTableAction::AlterColumn(col))
        } else {
            Err(self.err_at("expected ALTER TABLE action"))
        }
    }

    fn parse_drop(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Drop)?;
        match self.peek().clone() {
            TsqlToken::Table => {
                self.advance();
                let if_exists = self.parse_if_exists();
                let name = self.expect_identifier()?;
                Ok(TsqlStatement::DropTable(TsqlDropTable { name, if_exists }))
            }
            TsqlToken::Database => {
                self.advance();
                let if_exists = self.parse_if_exists();
                let name = self.expect_identifier()?;
                Ok(TsqlStatement::DropDatabase(TsqlDropDatabase { name, if_exists }))
            }
            TsqlToken::View => {
                self.advance();
                let if_exists = self.parse_if_exists();
                let name = self.expect_identifier()?;
                Ok(TsqlStatement::DropView(TsqlDropView { name, if_exists }))
            }
            TsqlToken::Index => {
                self.advance();
                let if_exists = self.parse_if_exists();
                let name = self.expect_identifier()?;
                let table = if self.match_token(&TsqlToken::On) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                Ok(TsqlStatement::DropIndex(TsqlDropIndex { name, table, if_exists }))
            }
            TsqlToken::Procedure | TsqlToken::Proc => {
                self.advance();
                let if_exists = self.parse_if_exists();
                let name = self.expect_identifier()?;
                Ok(TsqlStatement::DropProcedure(DropProcedureStmt { name, if_exists }))
            }
            _ => self.parse_passthrough_from("DROP"),
        }
    }

    fn parse_if_exists(&mut self) -> bool {
        if self.check(&TsqlToken::If) {
            if let TsqlToken::Not = self.peek_ahead(1) {
                // IF NOT EXISTS — but we only handle IF EXISTS here
                false
            } else {
                // Check for IF EXISTS pattern
                false
            }
        } else {
            false
        }
    }

    fn parse_truncate(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Truncate)?;
        self.match_token(&TsqlToken::Table);
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::TruncateTable(TsqlTruncateTable { name }))
    }

    // ── Control Flow ──

    fn parse_begin(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Begin)?;

        // BEGIN TRY...END TRY / BEGIN CATCH...END CATCH
        if self.match_token(&TsqlToken::Try) {
            return self.parse_try_catch();
        }
        if self.match_token(&TsqlToken::Catch) {
            // This shouldn't be reached as it's part of TRY/CATCH
            return Err(self.err_at("unexpected BEGIN CATCH without BEGIN TRY"));
        }

        // BEGIN TRAN[SACTION]
        if self.check(&TsqlToken::Transaction) || self.check(&TsqlToken::Tran) {
            self.advance();
            let name = if matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::AtVariable(_)) {
                Some(match self.advance() {
                    TsqlToken::Identifier(n) => n,
                    TsqlToken::AtVariable(n) => format!("@{}", n),
                    _ => unreachable!(),
                })
            } else {
                None
            };
            return Ok(TsqlStatement::BeginTransaction(name));
        }

        // BEGIN...END block
        let mut stmts = Vec::new();
        while !self.check(&TsqlToken::End) && !self.is_at_end() {
            self.skip_semicolons();
            if self.check(&TsqlToken::End) {
                break;
            }
            stmts.push(self.parse_statement()?);
            self.skip_semicolons();
        }
        self.expect(&TsqlToken::End)?;

        Ok(TsqlStatement::BeginEnd(stmts))
    }

    fn parse_try_catch(&mut self) -> TsqlResult<TsqlStatement> {
        // We've already consumed BEGIN TRY
        let mut try_body = Vec::new();
        while !self.check(&TsqlToken::End) && !self.is_at_end() {
            self.skip_semicolons();
            if self.check(&TsqlToken::End) {
                break;
            }
            try_body.push(self.parse_statement()?);
            self.skip_semicolons();
        }
        self.expect(&TsqlToken::End)?;
        self.expect(&TsqlToken::Try)?;

        // BEGIN CATCH...END CATCH
        self.expect(&TsqlToken::Begin)?;
        self.expect(&TsqlToken::Catch)?;

        let mut catch_body = Vec::new();
        while !self.check(&TsqlToken::End) && !self.is_at_end() {
            self.skip_semicolons();
            if self.check(&TsqlToken::End) {
                break;
            }
            catch_body.push(self.parse_statement()?);
            self.skip_semicolons();
        }
        self.expect(&TsqlToken::End)?;
        self.expect(&TsqlToken::Catch)?;

        Ok(TsqlStatement::TryCatch(TryCatchStmt {
            try_body,
            catch_body,
        }))
    }

    fn parse_if(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::If)?;
        let condition = self.parse_expr()?;

        let then_body = if self.check(&TsqlToken::Begin) {
            match self.parse_begin()? {
                TsqlStatement::BeginEnd(stmts) => stmts,
                other => vec![other],
            }
        } else {
            vec![self.parse_statement()?]
        };

        let else_body = if self.match_token(&TsqlToken::Else) {
            if self.check(&TsqlToken::Begin) {
                match self.parse_begin()? {
                    TsqlStatement::BeginEnd(stmts) => Some(stmts),
                    other => Some(vec![other]),
                }
            } else {
                Some(vec![self.parse_statement()?])
            }
        } else {
            None
        };

        Ok(TsqlStatement::IfElse {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::While)?;
        let condition = self.parse_expr()?;

        let body = if self.check(&TsqlToken::Begin) {
            match self.parse_begin()? {
                TsqlStatement::BeginEnd(stmts) => stmts,
                other => vec![other],
            }
        } else {
            vec![self.parse_statement()?]
        };

        Ok(TsqlStatement::While { condition, body })
    }

    fn parse_return(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Return)?;
        let expr = if !self.is_at_end()
            && !self.check(&TsqlToken::Semicolon)
            && !self.check(&TsqlToken::End)
            && !self.check(&TsqlToken::Go)
        {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(TsqlStatement::Return(expr))
    }

    fn parse_goto(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Goto)?;
        let label = self.expect_identifier()?;
        Ok(TsqlStatement::Goto(label))
    }

    fn parse_waitfor(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::WaitFor)?;
        if self.match_token(&TsqlToken::Delay) {
            let time = match self.advance() {
                TsqlToken::StringLiteral(s) => s,
                _ => return Err(self.err_at("expected time string after WAITFOR DELAY")),
            };
            Ok(TsqlStatement::WaitFor(WaitForType::Delay(time)))
        } else if self.match_token(&TsqlToken::Time) {
            let time = match self.advance() {
                TsqlToken::StringLiteral(s) => s,
                _ => return Err(self.err_at("expected time string after WAITFOR TIME")),
            };
            Ok(TsqlStatement::WaitFor(WaitForType::Time(time)))
        } else {
            Err(self.err_at("expected DELAY or TIME after WAITFOR"))
        }
    }

    // ── Variables & SET ──

    fn parse_declare(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Declare)?;

        // Check for DECLARE cursor_name CURSOR FOR ...
        if matches!(self.peek(), TsqlToken::Identifier(_)) && *self.peek_ahead(1) == TsqlToken::Cursor {
            return self.parse_declare_cursor();
        }

        let mut variables = Vec::new();
        loop {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => return Err(self.err_at("expected @variable in DECLARE")),
            };
            let data_type = self.parse_tsql_data_type()?;

            let default = if self.match_token(&TsqlToken::Equals) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            variables.push(VariableDecl {
                name,
                data_type,
                default,
            });

            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        Ok(TsqlStatement::Declare(DeclareStmt { variables }))
    }

    fn parse_declare_cursor(&mut self) -> TsqlResult<TsqlStatement> {
        let name = self.expect_identifier()?;
        self.expect(&TsqlToken::Cursor)?;

        // Parse optional scroll/sensitivity keywords
        let mut scroll_type = CursorScrollType::ForwardOnly;
        let mut sensitivity = CursorSensitivity::Unspecified;

        loop {
            match self.peek() {
                TsqlToken::ForwardOnly => { self.advance(); scroll_type = CursorScrollType::ForwardOnly; }
                TsqlToken::Scroll => { self.advance(); scroll_type = CursorScrollType::Scroll; }
                TsqlToken::Keyset => { self.advance(); scroll_type = CursorScrollType::Keyset; }
                TsqlToken::Dynamic => { self.advance(); scroll_type = CursorScrollType::Dynamic; }
                TsqlToken::Static => { self.advance(); scroll_type = CursorScrollType::Static; }
                TsqlToken::Insensitive => { self.advance(); sensitivity = CursorSensitivity::Insensitive; }
                TsqlToken::Sensitive => { self.advance(); sensitivity = CursorSensitivity::Sensitive; }
                _ => break,
            }
        }

        self.expect(&TsqlToken::For)?;
        let query = self.parse_select()?;

        let for_read_only = if self.match_token(&TsqlToken::For) {
            let _ = self.expect(&TsqlToken::ReadOnly);
            true
        } else {
            false
        };

        Ok(TsqlStatement::DeclareCursor(DeclareCursorStmt {
            name,
            scroll_type,
            sensitivity,
            query: Box::new(query),
            for_read_only,
        }))
    }

    fn parse_set(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Set)?;

        // SET @var = expr [, @var2 = expr2, ...]
        if self.peek_is_at_variable() {
            return self.parse_set_variable();
        }

        // SET option ON/OFF/value
        let option = self.expect_identifier()?;
        let upper = option.to_uppercase();

        match upper.as_str() {
            "NOCOUNT" | "ANSI_NULLS" | "ANSI_PADDING" | "QUOTED_IDENTIFIER"
            | "CONCAT_NULL_YIELDS_NULL" | "IMPLICIT_TRANSACTIONS" | "XACT_ABORT"
            | "ARITHABORT" | "SHOWPLAN_ALL" | "SHOWPLAN_TEXT" => {
                let value = if self.peek_ident_ci("ON") || self.peek_ident_ci("OFF") {
                    let val = self.consume_token_string().to_uppercase();
                    TsqlExpr::Literal(TsqlLiteral::String(val))
                } else {
                    self.parse_expr()?
                };
                Ok(TsqlStatement::SetOption(upper, value))
            }
            _ => {
                // SET @var = expr fallback
                let value = self.parse_expr()?;
                Ok(TsqlStatement::SetVariable(SetVariableStmt {
                    assignments: vec![(format!("@{}", option), value)],
                }))
            }
        }
    }

    fn parse_set_variable(&mut self) -> TsqlResult<TsqlStatement> {
        let mut assignments = Vec::new();
        loop {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => return Err(self.err_at("expected @variable")),
            };

            // Check for compound assignment: @var += expr
            let op = match self.peek() {
                TsqlToken::PlusAssign => Some(TsqlBinaryOp::Add),
                TsqlToken::MinusAssign => Some(TsqlBinaryOp::Subtract),
                TsqlToken::StarEquals => Some(TsqlBinaryOp::Multiply),
                TsqlToken::SlashEquals => Some(TsqlBinaryOp::Divide),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let value = self.parse_expr()?;
                assignments.push((name.clone(), TsqlExpr::CompoundAssign {
                    var: name,
                    op,
                    value: Box::new(value),
                }));
            } else {
                self.expect(&TsqlToken::Equals)?;
                let value = self.parse_expr()?;
                assignments.push((name, value));
            }

            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        Ok(TsqlStatement::SetVariable(SetVariableStmt { assignments }))
    }

    fn parse_select_into_vars(&mut self) -> TsqlResult<TsqlStatement> {
        // @var1 = expr1, @var2 = expr2 [, ...] FROM ... WHERE ...
        let mut assignments = Vec::new();
        loop {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => return Err(self.err_at("expected @variable")),
            };
            self.expect(&TsqlToken::Equals)?;
            let expr = self.parse_expr()?;
            assignments.push((name, expr));
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
            // Check if next is @variable (assignment) or not (then it's a normal SELECT)
            if !self.peek_is_at_variable() {
                break;
            }
            // Check if after @variable there's an = sign
            if let TsqlToken::Equals = self.peek_ahead(1) {
                continue;
            } else {
                break;
            }
        }

        let from = if self.match_token(&TsqlToken::From) {
            Some(self.parse_table_ref()?)
        } else {
            None
        };

        let where_clause = if self.match_token(&TsqlToken::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let group_by = if self.match_token(&TsqlToken::Group) {
            self.expect(&TsqlToken::By)?;
            self.parse_expr_list()?
        } else {
            Vec::new()
        };

        let having = if self.match_token(&TsqlToken::Having) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let order_by = if self.match_token(&TsqlToken::Order) {
            self.expect(&TsqlToken::By)?;
            self.parse_order_by_list()?
        } else {
            Vec::new()
        };

        Ok(TsqlStatement::SelectIntoVars(SelectIntoVarsStmt {
            assignments,
            from,
            where_clause,
            group_by,
            having,
            order_by,
        }))
    }

    fn parse_print(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Print)?;
        let expr = self.parse_expr()?;
        Ok(TsqlStatement::Print(expr))
    }

    // ── EXECUTE ──

    fn parse_execute(&mut self) -> TsqlResult<TsqlStatement> {
        self.advance(); // EXECUTE or EXEC
        let exec = self.parse_execute_body()?;
        Ok(TsqlStatement::Execute(exec))
    }

    fn parse_execute_body(&mut self) -> TsqlResult<ExecuteStmt> {
        // Optional: @return_status = EXEC ...
        let return_status = if self.peek_is_at_variable()
            && *self.peek_ahead(1) == TsqlToken::Equals
        {
            let name = match self.advance() {
                TsqlToken::AtVariable(n) => format!("@{}", n),
                _ => unreachable!(),
            };
            self.advance(); // consume =
            Some(name)
        } else {
            None
        };

        let procedure = self.expect_identifier()?;

        // Parse parameters
        let mut params = Vec::new();
        while !self.is_at_end()
            && !self.check(&TsqlToken::Semicolon)
            && !self.check(&TsqlToken::Go)
            && !self.check(&TsqlToken::End)
        {
            // Check for named parameter: @name = value
            if self.peek_is_at_variable() && *self.peek_ahead(1) == TsqlToken::Equals {
                let name = match self.advance() {
                    TsqlToken::AtVariable(n) => format!("@{}", n),
                    _ => unreachable!(),
                };
                self.advance(); // consume =
                let value = self.parse_expr()?;
                let output = self.match_token(&TsqlToken::Output);
                params.push(ExecuteParam::Named { name, value, output });
            } else {
                let value = self.parse_expr()?;
                let output = self.match_token(&TsqlToken::Output);
                if output {
                    // Positional with OUTPUT
                    params.push(ExecuteParam::Positional(value));
                } else {
                    params.push(ExecuteParam::Positional(value));
                }
            }

            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        Ok(ExecuteStmt {
            procedure,
            params,
            return_status,
        })
    }

    // ── Cursors ──

    fn parse_open_cursor(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Open)?;
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::OpenCursor(name))
    }

    fn parse_fetch_cursor(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Fetch)?;

        // Optional fetch orientation
        let orientation = match self.peek() {
            TsqlToken::Next => { self.advance(); FetchOrientation::Next }
            TsqlToken::Prior => { self.advance(); FetchOrientation::Prior }
            TsqlToken::First => { self.advance(); FetchOrientation::First }
            TsqlToken::Last => { self.advance(); FetchOrientation::Last }
            TsqlToken::Absolute => {
                self.advance();
                let n = match self.advance() {
                    TsqlToken::IntLiteral(n) => n,
                    TsqlToken::AtVariable(_name) => {
                        // @@variable — treat as 0 for now
                        0
                    }
                    _ => return Err(self.err_at("expected number after ABSOLUTE")),
                };
                FetchOrientation::Absolute(n)
            }
            TsqlToken::Relative => {
                self.advance();
                let n = match self.advance() {
                    TsqlToken::IntLiteral(n) => n,
                    _ => return Err(self.err_at("expected number after RELATIVE")),
                };
                FetchOrientation::Relative(n)
            }
            _ => FetchOrientation::Next,
        };

        self.expect(&TsqlToken::From)?;
        let cursor_name = self.expect_identifier()?;

        let into_variables = if self.match_token(&TsqlToken::Into) {
            let mut vars = Vec::new();
            loop {
                let name = match self.advance() {
                    TsqlToken::AtVariable(n) => format!("@{}", n),
                    _ => return Err(self.err_at("expected @variable in FETCH INTO")),
                };
                vars.push(name);
                if !self.match_token(&TsqlToken::Comma) {
                    break;
                }
            }
            vars
        } else {
            Vec::new()
        };

        Ok(TsqlStatement::FetchCursor(FetchCursorStmt {
            cursor_name,
            fetch_orientation: orientation,
            into_variables,
        }))
    }

    fn parse_close_cursor(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Close)?;
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::CloseCursor(name))
    }

    fn parse_deallocate_cursor(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Deallocate)?;
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::DeallocateCursor(name))
    }

    // ── Transactions ──

    fn parse_commit(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Commit)?;
        if self.match_token(&TsqlToken::Transaction) || self.match_token(&TsqlToken::Tran) {
            let name = if matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::AtVariable(_)) {
                Some(match self.advance() {
                    TsqlToken::Identifier(n) => n,
                    TsqlToken::AtVariable(n) => format!("@{}", n),
                    _ => unreachable!(),
                })
            } else {
                None
            };
            Ok(TsqlStatement::CommitTransaction(name))
        } else {
            Ok(TsqlStatement::CommitTransaction(None))
        }
    }

    fn parse_rollback(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Rollback)?;
        if self.match_token(&TsqlToken::Transaction) || self.match_token(&TsqlToken::Tran) {
            let name = if matches!(self.peek(), TsqlToken::Identifier(_) | TsqlToken::AtVariable(_)) {
                Some(match self.advance() {
                    TsqlToken::Identifier(n) => n,
                    TsqlToken::AtVariable(n) => format!("@{}", n),
                    _ => unreachable!(),
                })
            } else {
                None
            };
            Ok(TsqlStatement::RollbackTransaction(name))
        } else {
            Ok(TsqlStatement::RollbackTransaction(None))
        }
    }

    fn parse_save_transaction(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Save)?;
        let _ = self.match_token(&TsqlToken::Transaction) || self.match_token(&TsqlToken::Tran);
        let name = self.expect_identifier()?;
        Ok(TsqlStatement::SaveTransaction(name))
    }

    // ── Error Handling ──

    fn parse_throw(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Throw)?;
        if self.check(&TsqlToken::Semicolon) || self.is_at_end() {
            return Ok(TsqlStatement::Throw {
                error_number: None,
                message: TsqlExpr::Literal(TsqlLiteral::Null),
                state: None,
            });
        }
        let error_number = match self.advance() {
            TsqlToken::IntLiteral(n) => Some(n as i32),
            TsqlToken::AtVariable(_) => None,
            _ => return Err(self.err_at("expected error number")),
        };
        self.expect(&TsqlToken::Comma)?;
        let message = self.parse_expr()?;
        self.expect(&TsqlToken::Comma)?;
        let state = match self.advance() {
            TsqlToken::IntLiteral(n) => Some(n as i32),
            _ => return Err(self.err_at("expected state number")),
        };
        Ok(TsqlStatement::Throw {
            error_number,
            message,
            state,
        })
    }

    fn parse_raiserror(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Raiserror)?;
        self.expect(&TsqlToken::LeftParen)?;
        let message_or_id = self.parse_expr()?;
        self.expect(&TsqlToken::Comma)?;
        let severity = self.parse_expr()?;
        self.expect(&TsqlToken::Comma)?;
        let state = self.parse_expr()?;
        self.expect(&TsqlToken::RightParen)?;

        let with_log = if self.match_token(&TsqlToken::With) {
            if self.peek_ident_ci("LOG") { self.advance(); }
            true
        } else {
            false
        };

        Ok(TsqlStatement::Raiserror(RaiserrorStmt {
            message_or_id,
            severity,
            state,
            with_log,
        }))
    }

    // ── USE ──

    fn parse_use(&mut self) -> TsqlResult<TsqlStatement> {
        self.expect(&TsqlToken::Use)?;
        let db = self.expect_identifier()?;
        Ok(TsqlStatement::UseDatabase(db))
    }

    // ── Temp Table Creation ──

    fn parse_create_temp_table(&mut self) -> TsqlResult<TsqlStatement> {
        let name = match self.advance() {
            TsqlToken::PoundTemp(n) => format!("#{}", n),
            TsqlToken::DoublePoundTemp(n) => format!("##{}", n),
            _ => unreachable!(),
        };

        self.expect(&TsqlToken::LeftParen)?;
        let mut columns = Vec::new();
        loop {
            columns.push(self.parse_column_def()?);
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }
        self.expect(&TsqlToken::RightParen)?;

        Ok(TsqlStatement::CreateTempTable(TsqlCreateTable {
            name,
            database: None,
            columns,
            constraints: Vec::new(),
            on_clause: None,
            with_clause: None,
            text_image_on: None,
            lock_datapages: None,
            is_temp: true,
        }))
    }

    // ── Fallback / Passthrough ──

    fn parse_identifier_statement(&mut self) -> TsqlResult<TsqlStatement> {
        // An identifier at statement start could be:
        // 1. A system procedure call (sp_help, etc.)
        // 2. A regular procedure call without EXEC
        // 3. Something else
        let name = self.expect_identifier()?;
        let upper = name.to_uppercase();

        // System procedures
        if upper.starts_with("SP_") || upper.starts_with("SP") {
            return self.parse_system_procedure_call(&name);
        }

        // Treat as procedure call without EXEC keyword
        let mut params = Vec::new();
        while !self.is_at_end()
            && !self.check(&TsqlToken::Semicolon)
            && !self.check(&TsqlToken::Go)
            && !self.check(&TsqlToken::End)
        {
            params.push(ExecuteParam::Positional(self.parse_expr()?));
            if !self.match_token(&TsqlToken::Comma) {
                break;
            }
        }

        if params.is_empty() {
            // Unknown identifier statement — passthrough
            Ok(TsqlStatement::Passthrough(name))
        } else {
            Ok(TsqlStatement::Execute(ExecuteStmt {
                procedure: name,
                params,
                return_status: None,
            }))
        }
    }

    fn parse_system_procedure_call(&mut self, name: &str) -> TsqlResult<TsqlStatement> {
        let upper = name.to_uppercase();
        let sp = match upper.as_str() {
            "SP_HELP" => {
                let object = if !self.is_at_end() && !self.check(&TsqlToken::Semicolon) && !self.check(&TsqlToken::Go) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                SystemProcStmt::SpHelp { object }
            }
            "SP_WHO" => {
                let login_name = if !self.is_at_end() && !self.check(&TsqlToken::Semicolon) && !self.check(&TsqlToken::Go) {
                    Some(match self.advance() {
                        TsqlToken::StringLiteral(s) => s,
                        TsqlToken::Identifier(s) => s,
                        _ => return Err(self.err_at("expected login name")),
                    })
                } else {
                    None
                };
                SystemProcStmt::SpWho { login_name }
            }
            "SP_HELPDB" => {
                let database = if !self.is_at_end() && !self.check(&TsqlToken::Semicolon) && !self.check(&TsqlToken::Go) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                SystemProcStmt::SpHelpDb { database }
            }
            "SP_TABLES" => {
                SystemProcStmt::SpTables {
                    table_name: None,
                    table_owner: None,
                    table_type: None,
                }
            }
            "SP_COLUMNS" => {
                let table_name = self.expect_identifier()?;
                SystemProcStmt::SpColumns {
                    table_name,
                    column_name: None,
                    table_owner: None,
                }
            }
            "SP_DATABASES" => SystemProcStmt::SpDatabases,
            "SP_SPACEUSED" => {
                let table = if !self.is_at_end() && !self.check(&TsqlToken::Semicolon) && !self.check(&TsqlToken::Go) {
                    Some(self.expect_identifier()?)
                } else {
                    None
                };
                SystemProcStmt::SpSpaceUsed { table }
            }
            _ => {
                // Custom system procedure
                let mut params = Vec::new();
                while !self.is_at_end() && !self.check(&TsqlToken::Semicolon) && !self.check(&TsqlToken::Go) {
                    params.push(self.parse_expr()?);
                    if !self.match_token(&TsqlToken::Comma) {
                        break;
                    }
                }
                SystemProcStmt::Custom {
                    name: name.to_string(),
                    params,
                }
            }
        };
        Ok(TsqlStatement::SystemProcedure(sp))
    }

    fn parse_passthrough(&mut self) -> TsqlResult<TsqlStatement> {
        let mut sql = String::new();
        let mut depth = 0;
        while !self.is_at_end() {
            match self.peek() {
                TsqlToken::Go | TsqlToken::Semicolon if depth == 0 => {
                    self.advance();
                    break;
                }
                TsqlToken::LeftParen => { depth += 1; let s = self.consume_token_string(); sql.push_str(&s); sql.push('('); }
                TsqlToken::RightParen => { depth -= 1; let s = self.consume_token_string(); sql.push_str(&s); sql.push(')'); }
                _ => {
                    let s = self.consume_token_string();
                    sql.push_str(&s);
                    sql.push(' ');
                }
            }
        }
        Ok(TsqlStatement::Passthrough(sql.trim().to_string()))
    }

    fn parse_passthrough_from(&mut self, prefix: &str) -> TsqlResult<TsqlStatement> {
        let mut sql = String::from(prefix);
        sql.push(' ');
        while !self.is_at_end() && !self.check(&TsqlToken::Go) && !self.check(&TsqlToken::Semicolon) {
            let s = self.consume_token_string();
            sql.push_str(&s);
            sql.push(' ');
        }
        Ok(TsqlStatement::Passthrough(sql.trim().to_string()))
    }
}

impl Default for TsqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(sql: &str) -> Vec<TsqlStatement> {
        TsqlParser::new().parse(sql).unwrap()
    }

    #[test]
    fn test_simple_select() {
        let stmts = parse("SELECT 1");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::Select(_)));
    }

    #[test]
    fn test_create_procedure() {
        let stmts = parse("CREATE PROC test_proc @p1 INT, @p2 VARCHAR(50) AS BEGIN SELECT @p1 END");
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            TsqlStatement::CreateProcedure(cp) => {
                assert_eq!(cp.name, "test_proc");
                assert_eq!(cp.params.len(), 2);
            }
            _ => panic!("expected CreateProcedure"),
        }
    }

    #[test]
    fn test_declare_and_set() {
        let stmts = parse("DECLARE @x INT SET @x = 10");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(stmts[0], TsqlStatement::Declare(_)));
        assert!(matches!(stmts[1], TsqlStatement::SetVariable(_)));
    }

    #[test]
    fn test_if_else() {
        let stmts = parse("IF @x > 0 PRINT 'positive' ELSE PRINT 'non-positive'");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::IfElse { .. }));
    }

    #[test]
    fn test_while_loop() {
        let stmts = parse("WHILE @i < 10 BEGIN SET @i = @i + 1 END");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::While { .. }));
    }

    #[test]
    fn test_try_catch() {
        let stmts = parse("BEGIN TRY SELECT 1 END TRY BEGIN CATCH PRINT 'error' END CATCH");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::TryCatch(_)));
    }

    #[test]
    fn test_cursor() {
        let stmts = parse("DECLARE cur CURSOR FOR SELECT id FROM t1 OPEN cur FETCH NEXT FROM cur INTO @id CLOSE cur DEALLOCATE cur");
        assert_eq!(stmts.len(), 5);
    }

    #[test]
    fn test_execute() {
        let stmts = parse("EXEC my_proc @p1 = 1, @p2 = 'hello' OUTPUT");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::Execute(_)));
    }

    #[test]
    fn test_transaction() {
        let stmts = parse("BEGIN TRAN SAVE TRAN sp1 COMMIT TRAN");
        assert_eq!(stmts.len(), 3);
    }

    #[test]
    fn test_select_into_vars() {
        let stmts = parse("SELECT @x = col1, @y = col2 FROM t1 WHERE id = 1");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], TsqlStatement::SelectIntoVars(_)));
    }
}
