use std::collections::HashMap;
use common::ProcedureError;
use tsql_parser::ast::{CursorScrollType, FetchOrientation};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorStatus {
    Declared,
    Open,
    Closed,
    Deallocated,
}

#[derive(Debug, Clone)]
pub struct TsqlCursor {
    pub name: String,
    pub status: CursorStatus,
    pub result_set: Vec<Vec<String>>,  // Materialized rows as strings
    pub columns: Vec<(String, String)>, // (name, type_name)
    pub position: isize,
    pub scroll_type: CursorScrollType,
    pub is_read_only: bool,
}

#[derive(Debug, Default)]
pub struct CursorStore {
    cursors: HashMap<String, TsqlCursor>,
}

impl CursorStore {
    pub fn new() -> Self { Self::default() }

    pub fn declare(&mut self, name: &str, scroll_type: CursorScrollType) -> Result<(), ProcedureError> {
        if self.cursors.contains_key(name) {
            return Err(ProcedureError::CursorAlreadyDeclared(name.to_string()));
        }
        self.cursors.insert(name.to_lowercase(), TsqlCursor {
            name: name.to_string(),
            status: CursorStatus::Declared,
            result_set: Vec::new(),
            columns: Vec::new(),
            position: -1,
            scroll_type,
            is_read_only: true,
        });
        Ok(())
    }

    pub fn open(&mut self, name: &str, result_set: Vec<Vec<String>>, columns: Vec<(String, String)>) -> Result<(), ProcedureError> {
        let cursor = self.cursors.get_mut(&name.to_lowercase())
            .ok_or_else(|| ProcedureError::CursorNotDeclared(name.to_string()))?;
        cursor.result_set = result_set;
        cursor.columns = columns;
        cursor.position = -1;
        cursor.status = CursorStatus::Open;
        Ok(())
    }

    pub fn fetch(&mut self, name: &str, orientation: &FetchOrientation) -> Result<(Option<Vec<String>>, i32), ProcedureError> {
        let cursor = self.cursors.get_mut(&name.to_lowercase())
            .ok_or_else(|| ProcedureError::CursorNotDeclared(name.to_string()))?;
        if cursor.status != CursorStatus::Open {
            return Err(ProcedureError::CursorNotOpen(name.to_string()));
        }
        let row_count = cursor.result_set.len() as isize;
        match orientation {
            FetchOrientation::Next => {
                cursor.position += 1;
                if cursor.position >= 0 && cursor.position < row_count {
                    Ok((Some(cursor.result_set[cursor.position as usize].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
            FetchOrientation::Prior => {
                cursor.position -= 1;
                if cursor.position >= 0 && cursor.position < row_count {
                    Ok((Some(cursor.result_set[cursor.position as usize].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
            FetchOrientation::First => {
                cursor.position = 0;
                if row_count > 0 {
                    Ok((Some(cursor.result_set[0].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
            FetchOrientation::Last => {
                cursor.position = row_count - 1;
                if cursor.position >= 0 {
                    Ok((Some(cursor.result_set[cursor.position as usize].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
            FetchOrientation::Absolute(n) => {
                cursor.position = *n as isize;
                if cursor.position >= 0 && cursor.position < row_count {
                    Ok((Some(cursor.result_set[cursor.position as usize].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
            FetchOrientation::Relative(n) => {
                cursor.position += *n as isize;
                if cursor.position >= 0 && cursor.position < row_count {
                    Ok((Some(cursor.result_set[cursor.position as usize].clone()), 0))
                } else {
                    Ok((None, -1))
                }
            }
        }
    }

    pub fn close(&mut self, name: &str) -> Result<(), ProcedureError> {
        let cursor = self.cursors.get_mut(&name.to_lowercase())
            .ok_or_else(|| ProcedureError::CursorNotDeclared(name.to_string()))?;
        cursor.status = CursorStatus::Closed;
        Ok(())
    }

    pub fn deallocate(&mut self, name: &str) -> Result<(), ProcedureError> {
        self.cursors.remove(&name.to_lowercase())
            .ok_or_else(|| ProcedureError::CursorNotDeclared(name.to_string()))?;
        Ok(())
    }
}
