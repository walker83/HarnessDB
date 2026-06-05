use std::fmt;

/// Error type for T-SQL parsing failures.
#[derive(Debug, Clone)]
pub struct TsqlParseError {
    pub message: String,
    pub position: usize,
    pub line: usize,
    pub column: usize,
}

impl TsqlParseError {
    pub fn new(message: impl Into<String>, position: usize, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            position,
            line,
            column,
        }
    }

    pub fn syntax(message: impl Into<String>, position: usize, line: usize, column: usize) -> Self {
        Self::new(message, position, line, column)
    }

    pub fn unexpected_token(
        expected: &str,
        found: &str,
        position: usize,
        line: usize,
        column: usize,
    ) -> Self {
        Self::new(
            format!("expected {}, found '{}'", expected, found),
            position,
            line,
            column,
        )
    }

    pub fn unexpected_eof(expected: &str) -> Self {
        Self::new(format!("unexpected end of input, expected {}", expected), 0, 0, 0)
    }
}

impl fmt::Display for TsqlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "T-SQL parse error at line {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TsqlParseError {}

impl From<TsqlParseError> for common::DharnessError {
    fn from(e: TsqlParseError) -> Self {
        common::DharnessError::TsqlParse(e.to_string())
    }
}

pub type TsqlResult<T> = std::result::Result<T, TsqlParseError>;
