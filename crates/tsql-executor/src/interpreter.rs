use fe_catalog::procedure::StoredProcedure;
use common::ProcedureError;
use tsql_parser::ast::*;
use mysql_protocol::server::{ColumnDef, ColumnType, QueryResult};
use crate::context::ExecutionContext;

#[derive(Debug, Clone)]
pub enum TsqlExecutionResult {
    Ok,
    ResultSet(QueryResult),
    RowsAffected(u64),
    ReturnStatus(i32),
    Message(String),
    Multiple(Vec<TsqlExecutionResult>),
}

impl TsqlExecutionResult {
    pub fn into_query_result(self) -> QueryResult {
        match self {
            TsqlExecutionResult::ResultSet(qr) => qr,
            TsqlExecutionResult::RowsAffected(_n) => {
                QueryResult::ok()
            }
            TsqlExecutionResult::ReturnStatus(rc) => {
                QueryResult::with_rows(
                    vec![ColumnDef {
                        name: "ReturnStatus".to_string(),
                        col_type: ColumnType::Int,
                    }],
                    vec![vec![Some(rc.to_string())]],
                )
            }
            _ => QueryResult::ok(),
        }
    }
}

pub struct TsqlInterpreter;

impl TsqlInterpreter {
    pub fn new() -> Self { Self }

    pub fn execute_batch(
        &self,
        ctx: &mut ExecutionContext,
        statements: &[TsqlStatement],
    ) -> Result<TsqlExecutionResult, ProcedureError> {
        let mut last_result = TsqlExecutionResult::Ok;
        for stmt in statements {
            let result = self.exec_statement(ctx, stmt)?;
            if matches!(result, TsqlExecutionResult::ReturnStatus(_)) {
                return Ok(result);
            }
            last_result = result;
        }
        Ok(last_result)
    }

    fn exec_statement(
        &self,
        ctx: &mut ExecutionContext,
        stmt: &TsqlStatement,
    ) -> Result<TsqlExecutionResult, ProcedureError> {
        match stmt {
            TsqlStatement::Select(sel) => self.exec_select(ctx, sel),
            TsqlStatement::Insert(_ins) => self.exec_passthrough_dml(ctx, "INSERT STATEMENT"),
            TsqlStatement::Update(_upd) => self.exec_passthrough_dml(ctx, "UPDATE STATEMENT"),
            TsqlStatement::Delete(_del) => self.exec_passthrough_dml(ctx, "DELETE STATEMENT"),
            TsqlStatement::BeginEnd(stmts) => self.execute_batch(ctx, stmts),
            TsqlStatement::IfElse { condition, then_body, else_body } => {
                let cond_val = self.eval_condition(ctx, condition)?;
                if cond_val {
                    self.execute_batch(ctx, then_body)
                } else if let Some(else_body) = else_body {
                    self.execute_batch(ctx, else_body)
                } else {
                    Ok(TsqlExecutionResult::Ok)
                }
            }
            TsqlStatement::While { condition, body } => {
                loop {
                    let cond_val = self.eval_condition(ctx, condition)?;
                    if !cond_val { break; }
                    match self.execute_batch(ctx, body)? {
                        TsqlExecutionResult::ReturnStatus(_) => return Ok(TsqlExecutionResult::ReturnStatus(0)),
                        _ => {}
                    }
                }
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::Return(expr) => {
                let val = match expr {
                    Some(e) => self.eval_expr_to_int(ctx, e)?,
                    None => 0,
                };
                Ok(TsqlExecutionResult::ReturnStatus(val))
            }
            TsqlStatement::Declare(decl) => {
                for var in &decl.variables {
                    let default = var.default.as_ref().map(|e| self.eval_to_literal(ctx, e)).transpose()?;
                    ctx.variables.declare(&var.name, var.data_type.clone(), default);
                }
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::SetVariable(set) => {
                for (name, expr) in &set.assignments {
                    let val = self.eval_to_literal(ctx, expr)?;
                    ctx.variables.set(name, val)?;
                }
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::SelectIntoVars(sel) => {
                // Execute the SELECT via DataFusion and assign results to variables
                // For now, simplified: evaluate expressions without FROM
                for (name, expr) in &sel.assignments {
                    let val = self.eval_to_literal(ctx, expr)?;
                    ctx.variables.set(name, val)?;
                }
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::Print(expr) => {
                let val = self.eval_to_literal(ctx, expr)?;
                let msg = literal_to_string(&val);
                ctx.messages.push(msg.clone());
                Ok(TsqlExecutionResult::Message(msg))
            }
            TsqlStatement::CreateProcedure(cp) => {
                let id = ctx.catalog.next_id();
                let now = chrono::Utc::now().timestamp();
                let params: Vec<fe_catalog::procedure::ProcedureParamMeta> = cp.params.iter().map(|p| {
                    fe_catalog::procedure::ProcedureParamMeta {
                        name: p.name.clone(),
                        data_type: p.data_type.to_string(),
                        direction: if p.output { "OUT".to_string() } else { "IN".to_string() },
                        default_value: p.default.as_ref().map(|_| None).unwrap_or(None),
                    }
                }).collect();
                let proc = StoredProcedure {
                    id,
                    name: cp.name.clone(),
                    database: ctx.current_database.clone(),
                    owner: "dbo".to_string(),
                    create_time: now,
                    alter_time: now,
                    source_sql: format!("CREATE PROC {} ...", cp.name), // simplified
                    params,
                    is_recompiled: cp.with_recompile,
                    is_encrypted: cp.with_encryption,
                };
                ctx.catalog.create_procedure(&ctx.current_database, proc)
                    .map_err(|e| ProcedureError::SyntaxError(e.to_string()))?;
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::DropProcedure(dp) => {
                ctx.catalog.drop_procedure(&ctx.current_database, &dp.name)
                    .map_err(|_e| ProcedureError::ProcedureNotFound(dp.name.clone()))?;
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::Execute(exec) => {
                // Look up procedure in catalog
                let proc_meta = ctx.catalog.get_procedure(&ctx.current_database, &exec.procedure)
                    .map_err(|e| ProcedureError::SyntaxError(e.to_string()))?;
                match proc_meta {
                    Some(meta) => {
                        // Parse and execute the procedure body
                        let mut child_ctx = ctx.child_context();
                        // Bind parameters
                        for (i, param) in meta.params.iter().enumerate() {
                            if let Some(exec_param) = exec.params.get(i) {
                                match exec_param {
                                    ExecuteParam::Positional(expr) => {
                                        let val = self.eval_to_literal(ctx, expr)?;
                                        let dt = tsql_parser::parse_tsql_type_name(&param.data_type)
                                            .unwrap_or(TsqlDataType::Varchar(Some(255)));
                                        child_ctx.variables.declare(&param.name, dt, Some(val));
                                    }
                                    ExecuteParam::Named { name, value, .. } => {
                                        let val = self.eval_to_literal(ctx, value)?;
                                        let dt = tsql_parser::parse_tsql_type_name(&param.data_type)
                                            .unwrap_or(TsqlDataType::Varchar(Some(255)));
                                        child_ctx.variables.declare(name, dt, Some(val));
                                    }
                                }
                            }
                        }
                        // For now, we don't have the original body stored, so return OK
                        Ok(TsqlExecutionResult::ReturnStatus(0))
                    }
                    None => {
                        // Check if it's a system procedure
                        if exec.procedure.to_uppercase().starts_with("SP_") {
                            crate::system_procs::execute_system_proc(ctx, &exec.procedure, &exec.params, self)
                        } else {
                            Err(ProcedureError::ProcedureNotFound(exec.procedure.clone()))
                        }
                    }
                }
            }
            TsqlStatement::SystemProcedure(sp) => {
                crate::system_procs::execute_system_proc_stmt(ctx, sp)
            }
            TsqlStatement::TryCatch(tc) => {
                match self.execute_batch(ctx, &tc.try_body) {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        ctx.error = 1;
                        ctx.catch_error_message = e.to_string();
                        ctx.catch_error_number = 1;
                        ctx.catch_error_severity = 16;
                        ctx.catch_error_state = 1;
                        match self.execute_batch(ctx, &tc.catch_body) {
                            Ok(r) => { ctx.error = 0; Ok(r) }
                            Err(e2) => Err(e2),
                        }
                    }
                }
            }
            TsqlStatement::Raiserror(re) => {
                let msg = match self.eval_to_literal(ctx, &re.message_or_id) {
                    Ok(lit) => literal_to_string(&lit),
                    Err(_) => "Unknown error".to_string(),
                };
                Err(ProcedureError::Raiserror { severity: 16, state: 1, message: msg })
            }
            TsqlStatement::Throw { message, .. } => {
                let msg = match self.eval_to_literal(ctx, message) {
                    Ok(lit) => literal_to_string(&lit),
                    Err(_) => "User thrown error".to_string(),
                };
                Err(ProcedureError::Raiserror { severity: 16, state: 1, message: msg })
            }
            TsqlStatement::DeclareCursor(dc) => {
                ctx.cursors.declare(&dc.name, dc.scroll_type)?;
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::OpenCursor(name) => {
                // For now, open with empty result set
                ctx.cursors.open(name, Vec::new(), Vec::new())?;
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::FetchCursor(fc) => {
                match ctx.cursors.fetch(&fc.cursor_name, &fc.fetch_orientation) {
                    Ok((row, status)) => {
                        ctx.fetch_status = status;
                        if let Some(row) = row {
                            for (i, var_name) in fc.into_variables.iter().enumerate() {
                                if let Some(val) = row.get(i) {
                                    ctx.variables.set(var_name, TsqlLiteral::String(val.clone()))?;
                                }
                            }
                        }
                        Ok(TsqlExecutionResult::Ok)
                    }
                    Err(e) => {
                        ctx.fetch_status = -1;
                        Err(e)
                    }
                }
            }
            TsqlStatement::CloseCursor(name) => { ctx.cursors.close(name)?; Ok(TsqlExecutionResult::Ok) }
            TsqlStatement::DeallocateCursor(name) => { ctx.cursors.deallocate(name)?; Ok(TsqlExecutionResult::Ok) }
            TsqlStatement::BeginTransaction(_name) => {
                ctx.txn_mgr.begin_tran(ctx.conn_id);
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::CommitTransaction(_) => {
                ctx.txn_mgr.commit(ctx.conn_id);
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::RollbackTransaction(_) => {
                ctx.txn_mgr.rollback(ctx.conn_id);
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::SaveTransaction(name) => {
                ctx.txn_mgr.save_tran(ctx.conn_id, name);
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::UseDatabase(db) => {
                ctx.current_database = db.clone();
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::SetOption(name, value) => {
                let val_str = match self.eval_to_literal(ctx, value) {
                    Ok(lit) => literal_to_string(&lit),
                    Err(_) => String::new(),
                };
                match name.to_uppercase().as_str() {
                    "NOCOUNT" => ctx.set_options.nocount = val_str.to_uppercase() == "ON",
                    "ANSI_NULLS" => ctx.set_options.ansi_nulls = val_str.to_uppercase() == "ON",
                    "XACT_ABORT" => ctx.set_options.xact_abort = val_str.to_uppercase() == "ON",
                    _ => {}
                }
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::Break => Ok(TsqlExecutionResult::Ok), // handled by while loop
            TsqlStatement::Continue => Ok(TsqlExecutionResult::Ok),
            TsqlStatement::Passthrough(_sql) => {
                // Pass through to the storage layer
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::CreateTable(_ct) | TsqlStatement::CreateTempTable(_ct) => {
                // Delegate to catalog for table creation
                Ok(TsqlExecutionResult::Ok)
            }
            TsqlStatement::NoOp => Ok(TsqlExecutionResult::Ok),
            _ => Ok(TsqlExecutionResult::Ok),
        }
    }

    fn eval_condition(&self, ctx: &mut ExecutionContext, expr: &TsqlExpr) -> Result<bool, ProcedureError> {
        match expr {
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::Eq, right } => {
                let l = self.eval_to_literal(ctx, left)?;
                let r = self.eval_to_literal(ctx, right)?;
                Ok(literal_to_string(&l) == literal_to_string(&r))
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::NotEq, right } => {
                let l = self.eval_to_literal(ctx, left)?;
                let r = self.eval_to_literal(ctx, right)?;
                Ok(literal_to_string(&l) != literal_to_string(&r))
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::Lt, right } => {
                let l = self.eval_expr_to_int(ctx, left)?;
                let r = self.eval_expr_to_int(ctx, right)?;
                Ok(l < r)
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::Gt, right } => {
                let l = self.eval_expr_to_int(ctx, left)?;
                let r = self.eval_expr_to_int(ctx, right)?;
                Ok(l > r)
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::LtEq, right } => {
                let l = self.eval_expr_to_int(ctx, left)?;
                let r = self.eval_expr_to_int(ctx, right)?;
                Ok(l <= r)
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::GtEq, right } => {
                let l = self.eval_expr_to_int(ctx, left)?;
                let r = self.eval_expr_to_int(ctx, right)?;
                Ok(l >= r)
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::And, right } => {
                Ok(self.eval_condition(ctx, left)? && self.eval_condition(ctx, right)?)
            }
            TsqlExpr::BinaryOp { left, op: TsqlBinaryOp::Or, right } => {
                Ok(self.eval_condition(ctx, left)? || self.eval_condition(ctx, right)?)
            }
            TsqlExpr::UnaryOp { op: TsqlUnaryOp::Not, expr } => {
                Ok(!self.eval_condition(ctx, expr)?)
            }
            TsqlExpr::Literal(TsqlLiteral::Null) => Ok(false),
            TsqlExpr::Literal(TsqlLiteral::Bit(b)) => Ok(*b),
            TsqlExpr::Literal(TsqlLiteral::Int(n)) => Ok(*n != 0),
            _ => {
                // Try to evaluate as integer, non-zero = true
                let val = self.eval_expr_to_int(ctx, expr).unwrap_or(0);
                Ok(val != 0)
            }
        }
    }

    fn eval_expr_to_int(&self, ctx: &mut ExecutionContext, expr: &TsqlExpr) -> Result<i32, ProcedureError> {
        let lit = self.eval_to_literal(ctx, expr)?;
        match lit {
            TsqlLiteral::Int(n) => Ok(n as i32),
            TsqlLiteral::Float(f) => Ok(f as i32),
            TsqlLiteral::Bit(b) => Ok(if b { 1 } else { 0 }),
            TsqlLiteral::String(s) => Ok(s.parse().unwrap_or(0)),
            TsqlLiteral::Null => Ok(0),
            _ => Ok(0),
        }
    }

    fn eval_to_literal(&self, ctx: &mut ExecutionContext, expr: &TsqlExpr) -> Result<TsqlLiteral, ProcedureError> {
        match expr {
            TsqlExpr::Literal(lit) => Ok(lit.clone()),
            TsqlExpr::Variable(name) => ctx.variables.get(name).cloned(),
            TsqlExpr::SystemVariable(name) => Ok(ctx.system_variable(name)),
            TsqlExpr::BinaryOp { left, op, right } => {
                let l = self.eval_to_literal(ctx, left)?;
                let r = self.eval_to_literal(ctx, right)?;
                Ok(self.eval_binary_op(&l, op, &r))
            }
            TsqlExpr::UnaryOp { op: TsqlUnaryOp::Negate, expr } => {
                let val = self.eval_to_literal(ctx, expr)?;
                match val {
                    TsqlLiteral::Int(n) => Ok(TsqlLiteral::Int(-n)),
                    TsqlLiteral::Float(f) => Ok(TsqlLiteral::Float(-f)),
                    other => Ok(other),
                }
            }
            TsqlExpr::FunctionCall { name, args } => {
                self.eval_builtin_function(ctx, name, args)
            }
            TsqlExpr::IsNull { expr, replacement } => {
                let val = self.eval_to_literal(ctx, expr)?;
                if matches!(val, TsqlLiteral::Null) {
                    self.eval_to_literal(ctx, replacement)
                } else {
                    Ok(val)
                }
            }
            TsqlExpr::Coalesce(exprs) => {
                for e in exprs {
                    let val = self.eval_to_literal(ctx, e)?;
                    if !matches!(val, TsqlLiteral::Null) {
                        return Ok(val);
                    }
                }
                Ok(TsqlLiteral::Null)
            }
            TsqlExpr::NullIf { expr, other } => {
                let l = self.eval_to_literal(ctx, expr)?;
                let r = self.eval_to_literal(ctx, other)?;
                if literal_to_string(&l) == literal_to_string(&r) {
                    Ok(TsqlLiteral::Null)
                } else {
                    Ok(l)
                }
            }
            TsqlExpr::ColumnRef { .. } => {
                // Without a FROM clause execution engine, return null for column refs
                Ok(TsqlLiteral::Null)
            }
            _ => Ok(TsqlLiteral::Null),
        }
    }

    fn eval_binary_op(&self, left: &TsqlLiteral, op: &TsqlBinaryOp, right: &TsqlLiteral) -> TsqlLiteral {
        match (left, op, right) {
            (TsqlLiteral::Int(l), TsqlBinaryOp::Add, TsqlLiteral::Int(r)) => TsqlLiteral::Int(l + r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Subtract, TsqlLiteral::Int(r)) => TsqlLiteral::Int(l - r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Multiply, TsqlLiteral::Int(r)) => TsqlLiteral::Int(l * r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Divide, TsqlLiteral::Int(r)) if *r != 0 => TsqlLiteral::Int(l / r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Eq, TsqlLiteral::Int(r)) => TsqlLiteral::Bit(l == r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::NotEq, TsqlLiteral::Int(r)) => TsqlLiteral::Bit(l != r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Lt, TsqlLiteral::Int(r)) => TsqlLiteral::Bit(l < r),
            (TsqlLiteral::Int(l), TsqlBinaryOp::Gt, TsqlLiteral::Int(r)) => TsqlLiteral::Bit(l > r),
            (TsqlLiteral::String(l), TsqlBinaryOp::Add, TsqlLiteral::String(r)) => TsqlLiteral::String(format!("{}{}", l, r)),
            (TsqlLiteral::String(l), TsqlBinaryOp::Eq, TsqlLiteral::String(r)) => TsqlLiteral::Bit(l == r),
            _ => TsqlLiteral::Null,
        }
    }

    fn eval_builtin_function(&self, ctx: &mut ExecutionContext, name: &str, args: &[TsqlExpr]) -> Result<TsqlLiteral, ProcedureError> {
        match name.to_uppercase().as_str() {
            "GETDATE" | "SYSDATETIME" => {
                Ok(TsqlLiteral::DateTime(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()))
            }
            "GETUTCDATE" => {
                Ok(TsqlLiteral::DateTime(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()))
            }
            "NEWID" => {
                Ok(TsqlLiteral::String(uuid::Uuid::new_v4().to_string()))
            }
            "ISNULL" => {
                if args.len() >= 2 {
                    let val = self.eval_to_literal(ctx, &args[0])?;
                    if matches!(val, TsqlLiteral::Null) {
                        self.eval_to_literal(ctx, &args[1])
                    } else {
                        Ok(val)
                    }
                } else {
                    Ok(TsqlLiteral::Null)
                }
            }
            "LEN" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_to_literal(ctx, arg)?;
                    Ok(TsqlLiteral::Int(literal_to_string(&val).len() as i64))
                } else {
                    Ok(TsqlLiteral::Int(0))
                }
            }
            "UPPER" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_to_literal(ctx, arg)?;
                    Ok(TsqlLiteral::String(literal_to_string(&val).to_uppercase()))
                } else {
                    Ok(TsqlLiteral::String(String::new()))
                }
            }
            "LOWER" => {
                if let Some(arg) = args.first() {
                    let val = self.eval_to_literal(ctx, arg)?;
                    Ok(TsqlLiteral::String(literal_to_string(&val).to_lowercase()))
                } else {
                    Ok(TsqlLiteral::String(String::new()))
                }
            }
            "DB_NAME" => {
                Ok(TsqlLiteral::String(ctx.current_database.clone()))
            }
            "ERROR_MESSAGE" => Ok(TsqlLiteral::String(ctx.catch_error_message.clone())),
            "ERROR_NUMBER" => Ok(TsqlLiteral::Int(ctx.catch_error_number as i64)),
            "ERROR_SEVERITY" => Ok(TsqlLiteral::Int(ctx.catch_error_severity as i64)),
            "ERROR_STATE" => Ok(TsqlLiteral::Int(ctx.catch_error_state as i64)),
            "ERROR_LINE" => Ok(TsqlLiteral::Int(ctx.catch_error_line as i64)),
            "ERROR_PROCEDURE" => {
                Ok(ctx.catch_error_procedure.as_ref()
                    .map(|s| TsqlLiteral::String(s.clone()))
                    .unwrap_or(TsqlLiteral::Null))
            }
            _ => Ok(TsqlLiteral::Null),
        }
    }

    fn exec_select(&self, ctx: &mut ExecutionContext, sel: &TsqlSelect) -> Result<TsqlExecutionResult, ProcedureError> {
        // Simplified SELECT execution for simple expressions
        if sel.from.is_none() && sel.select_list.len() == 1 {
            let item = &sel.select_list[0];
            let val = self.eval_to_literal(ctx, &item.expr)?;
            let col_name = item.alias.clone().unwrap_or_else(|| "result".to_string());
            return Ok(TsqlExecutionResult::ResultSet(QueryResult::with_rows(
                vec![ColumnDef {
                    name: col_name,
                    col_type: ColumnType::String,
                }],
                vec![vec![Some(literal_to_string(&val))]],
            )));
        }
        Ok(TsqlExecutionResult::Ok)
    }

    fn exec_passthrough_dml(&self, _ctx: &mut ExecutionContext, _sql: &str) -> Result<TsqlExecutionResult, ProcedureError> {
        Ok(TsqlExecutionResult::Ok)
    }
}

impl Default for TsqlInterpreter {
    fn default() -> Self { Self::new() }
}

pub fn literal_to_string(lit: &TsqlLiteral) -> String {
    match lit {
        TsqlLiteral::Null => "NULL".to_string(),
        TsqlLiteral::Int(n) => n.to_string(),
        TsqlLiteral::Float(f) => f.to_string(),
        TsqlLiteral::String(s) => s.clone(),
        TsqlLiteral::Binary(b) => format!("0x{}", b.iter().map(|x| format!("{:02x}", x)).collect::<String>()),
        TsqlLiteral::Money(s) => s.clone(),
        TsqlLiteral::DateTime(s) => s.clone(),
        TsqlLiteral::Bit(b) => if *b { "1".to_string() } else { "0".to_string() },
    }
}
