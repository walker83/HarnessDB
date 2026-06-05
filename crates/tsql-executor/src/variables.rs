use std::collections::HashMap;
use tsql_parser::ast::TsqlDataType;
use tsql_parser::ast::TsqlLiteral;
use common::ProcedureError;

#[derive(Debug, Clone)]
pub struct VariableStore {
    locals: HashMap<String, (TsqlDataType, TsqlLiteral)>,
}

impl VariableStore {
    pub fn new() -> Self {
        Self { locals: HashMap::new() }
    }

    pub fn declare(&mut self, name: &str, data_type: TsqlDataType, default: Option<TsqlLiteral>) {
        let val = default.unwrap_or_else(|| Self::default_for_type(&data_type));
        self.locals.insert(name.to_lowercase(), (data_type, val));
    }

    pub fn set(&mut self, name: &str, value: TsqlLiteral) -> Result<(), ProcedureError> {
        let key = name.to_lowercase();
        if let Some((dt, _)) = self.locals.get(&key) {
            let dt = dt.clone();
            let coerced = Self::coerce_literal(value, &dt);
            self.locals.insert(key, (dt, coerced));
            Ok(())
        } else {
            Err(ProcedureError::VariableNotDeclared(name.to_string()))
        }
    }

    pub fn get(&self, name: &str) -> Result<&TsqlLiteral, ProcedureError> {
        self.locals
            .get(&name.to_lowercase())
            .map(|(_, v)| v)
            .ok_or_else(|| ProcedureError::VariableNotDeclared(name.to_string()))
    }

    fn default_for_type(dt: &TsqlDataType) -> TsqlLiteral {
        match dt {
            TsqlDataType::Int | TsqlDataType::SmallInt | TsqlDataType::TinyInt | TsqlDataType::BigInt => TsqlLiteral::Int(0),
            TsqlDataType::Float(_) | TsqlDataType::Real => TsqlLiteral::Float(0.0),
            TsqlDataType::Bit => TsqlLiteral::Bit(false),
            TsqlDataType::Money | TsqlDataType::SmallMoney => TsqlLiteral::Money("0.00".to_string()),
            TsqlDataType::Char(_) | TsqlDataType::Varchar(_) | TsqlDataType::NChar(_) | TsqlDataType::NVarchar(_)
            | TsqlDataType::Text | TsqlDataType::NText | TsqlDataType::Xml => TsqlLiteral::String(String::new()),
            TsqlDataType::Binary(_) | TsqlDataType::VarBinary(_) | TsqlDataType::Image => TsqlLiteral::Binary(vec![]),
            TsqlDataType::Date | TsqlDataType::DateTime | TsqlDataType::SmallDateTime | TsqlDataType::DateTime2(_) => TsqlLiteral::DateTime(String::new()),
            _ => TsqlLiteral::Null,
        }
    }

    fn coerce_literal(value: TsqlLiteral, _target: &TsqlDataType) -> TsqlLiteral {
        // Simple pass-through for now — full coercion is Phase 6
        value
    }

    pub fn snapshot(&self) -> HashMap<String, (TsqlDataType, TsqlLiteral)> {
        self.locals.clone()
    }

    pub fn restore(&mut self, snapshot: HashMap<String, (TsqlDataType, TsqlLiteral)>) {
        self.locals = snapshot;
    }
}

impl Default for VariableStore {
    fn default() -> Self { Self::new() }
}
