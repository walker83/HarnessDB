//! T-SQL Parser for SAP ASE 16 (Sybase) compatibility.
//!
//! A hand-rolled recursive descent parser that supports the complete T-SQL dialect
//! including stored procedures, control flow, cursors, error handling, and all
//! SAP ASE-specific syntax extensions.

pub mod ast;
pub mod batch;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod types;

pub use ast::*;
pub use batch::split_batches;
pub use error::{TsqlParseError, TsqlResult};
pub use lexer::{TsqlLexer, TsqlToken};
pub use parser::TsqlParser;
pub use types::{datatype_to_tsql_type, parse_tsql_type_name, tsql_type_to_datatype};
