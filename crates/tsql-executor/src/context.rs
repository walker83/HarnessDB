use std::sync::Arc;
use fe_catalog::catalog::CatalogManager;
use fe_storage::ParquetStorage;
use tsql_parser::ast::TsqlLiteral;
use crate::variables::VariableStore;
use crate::cursor_engine::CursorStore;
use crate::transaction::TsqlTransactionManager;

#[derive(Debug, Clone)]
pub struct SetOptions {
    pub nocount: bool,
    pub ansi_nulls: bool,
    pub xact_abort: bool,
}

impl Default for SetOptions {
    fn default() -> Self {
        Self { nocount: false, ansi_nulls: true, xact_abort: false }
    }
}

pub struct ExecutionContext {
    pub conn_id: u32,
    pub current_database: String,
    pub variables: VariableStore,
    pub cursors: CursorStore,
    pub set_options: SetOptions,
    // @@system variables
    pub rowcount: u64,
    pub error: i32,
    pub fetch_status: i32,
    pub nest_level: u32,
    pub identity: i64,
    // Output
    pub messages: Vec<String>,
    // References
    pub catalog: Arc<CatalogManager>,
    pub storage: Arc<ParquetStorage>,
    pub txn_mgr: Arc<TsqlTransactionManager>,
    pub max_nest_level: u32,
    // Error context for CATCH block
    pub catch_error_number: i32,
    pub catch_error_message: String,
    pub catch_error_severity: u8,
    pub catch_error_state: u8,
    pub catch_error_line: u32,
    pub catch_error_procedure: Option<String>,
}

impl ExecutionContext {
    pub fn new(
        conn_id: u32,
        database: String,
        catalog: Arc<CatalogManager>,
        storage: Arc<ParquetStorage>,
        txn_mgr: Arc<TsqlTransactionManager>,
    ) -> Self {
        Self {
            conn_id,
            current_database: database,
            variables: VariableStore::new(),
            cursors: CursorStore::new(),
            set_options: SetOptions::default(),
            rowcount: 0,
            error: 0,
            fetch_status: -9, // not yet fetched
            nest_level: 0,
            identity: 0,
            messages: Vec::new(),
            catalog,
            storage,
            txn_mgr,
            max_nest_level: 32,
            catch_error_number: 0,
            catch_error_message: String::new(),
            catch_error_severity: 0,
            catch_error_state: 0,
            catch_error_line: 0,
            catch_error_procedure: None,
        }
    }

    pub fn system_variable(&self, name: &str) -> TsqlLiteral {
        match name.to_uppercase().as_str() {
            "ERROR" => TsqlLiteral::Int(self.error as i64),
            "ROWCOUNT" => TsqlLiteral::Int(self.rowcount as i64),
            "FETCH_STATUS" => TsqlLiteral::Int(self.fetch_status as i64),
            "TRANCOUNT" => TsqlLiteral::Int(self.txn_mgr.get_tran_count(self.conn_id) as i64),
            "NESTLEVEL" => TsqlLiteral::Int(self.nest_level as i64),
            "SPID" => TsqlLiteral::Int(self.conn_id as i64),
            "IDENTITY" => TsqlLiteral::Int(self.identity),
            "SERVERNAME" => TsqlLiteral::String("HarnessDB".to_string()),
            "VERSION" => TsqlLiteral::String("16.0".to_string()),
            "LANGUAGE" => TsqlLiteral::String("us_english".to_string()),
            "DATEFORMAT" => TsqlLiteral::String("mdy".to_string()),
            "MAX_CONNECTIONS" => TsqlLiteral::Int(100),
            "CPU_BUSY" => TsqlLiteral::Int(0),
            "IDLE" => TsqlLiteral::Int(0),
            "IO_BUSY" => TsqlLiteral::Int(0),
            "PACK_RECEIVED" => TsqlLiteral::Int(0),
            "PACK_SENT" => TsqlLiteral::Int(0),
            "PACKET_ERRORS" => TsqlLiteral::Int(0),
            "TOTAL_ERRORS" => TsqlLiteral::Int(0),
            "TIMETICKS" => TsqlLiteral::Int(31250),
            "CONNECTIONS" => TsqlLiteral::Int(1),
            _ => TsqlLiteral::Null,
        }
    }

    pub fn child_context(&self) -> Self {
        Self {
            conn_id: self.conn_id,
            current_database: self.current_database.clone(),
            variables: VariableStore::new(),
            cursors: CursorStore::new(),
            set_options: self.set_options.clone(),
            rowcount: 0,
            error: 0,
            fetch_status: -9,
            nest_level: self.nest_level + 1,
            identity: self.identity,
            messages: Vec::new(),
            catalog: self.catalog.clone(),
            storage: self.storage.clone(),
            txn_mgr: self.txn_mgr.clone(),
            max_nest_level: self.max_nest_level,
            catch_error_number: 0,
            catch_error_message: String::new(),
            catch_error_severity: 0,
            catch_error_state: 0,
            catch_error_line: 0,
            catch_error_procedure: None,
        }
    }
}
