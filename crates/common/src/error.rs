use thiserror::Error;

#[derive(Error, Debug)]
pub enum DrorisError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("storage error: {kind} - {message}")]
    Storage {
        #[source]
        kind: StorageError,
        tablet_id: Option<u64>,
        message: String,
    },

    #[error("query error: {kind} - {message}")]
    Query {
        #[source]
        kind: QueryError,
        query_id: Option<String>,
        message: String,
    },

    #[error("catalog error: {kind} - {message}")]
    Catalog {
        #[source]
        kind: CatalogError,
        database_name: Option<String>,
        table_name: Option<String>,
        message: String,
    },

    #[error("parse error: {kind} - {message}")]
    Parse {
        #[source]
        kind: ParseError,
        position: Option<usize>,
        message: String,
    },

    #[error("plan error: {kind} - {message}")]
    Plan {
        #[source]
        kind: PlanError,
        message: String,
    },

    #[error("rpc error: {kind} - {message}")]
    Rpc {
        #[source]
        kind: RpcError,
        endpoint: Option<String>,
        message: String,
    },

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug, Clone)]
pub enum StorageError {
    #[error("tablet not found")]
    TabletNotFound,
    #[error("tablet already exists")]
    TabletAlreadyExists,
    #[error("flush failed")]
    FlushFailed,
    #[error("compaction failed")]
    CompactionFailed,
    #[error("segment not found")]
    SegmentNotFound,
    #[error("rowset not found")]
    RowsetNotFound,
    #[error("write failed")]
    WriteFailed,
    #[error("read failed")]
    ReadFailed,
    #[error("invalid schema")]
    InvalidSchema,
    #[error("memory limit exceeded")]
    MemoryLimitExceeded,
}

#[derive(Error, Debug, Clone)]
pub enum QueryError {
    #[error("not found")]
    NotFound,
    #[error("timeout")]
    Timeout,
    #[error("cancelled")]
    Cancelled,
    #[error("syntax error")]
    SyntaxError,
    #[error("execution failed")]
    ExecutionFailed,
    #[error("resource exhausted")]
    ResourceExhausted,
    #[error("invalid plan")]
    InvalidPlan,
}

#[derive(Error, Debug, Clone)]
pub enum CatalogError {
    #[error("database not found")]
    DatabaseNotFound,
    #[error("database already exists")]
    DatabaseAlreadyExists,
    #[error("table not found")]
    TableNotFound,
    #[error("table already exists")]
    TableAlreadyExists,
    #[error("invalid name")]
    InvalidName,
    #[error("permission denied")]
    PermissionDenied,
}

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("invalid syntax")]
    InvalidSyntax,
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("invalid literal")]
    InvalidLiteral,
    #[error("incomplete statement")]
    IncompleteStatement,
}

#[derive(Error, Debug, Clone)]
pub enum PlanError {
    #[error("invalid expression")]
    InvalidExpression,
    #[error("type mismatch")]
    TypeMismatch,
    #[error("ambiguous reference")]
    AmbiguousReference,
    #[error("unsupported operation")]
    UnsupportedOperation,
    #[error("invalid join")]
    InvalidJoin,
    #[error("invalid aggregation")]
    InvalidAggregation,
}

#[derive(Error, Debug, Clone)]
pub enum RpcError {
    #[error("connection failed")]
    ConnectionFailed,
    #[error("timeout")]
    Timeout,
    #[error("service unavailable")]
    ServiceUnavailable,
    #[error("invalid request")]
    InvalidRequest,
    #[error("internal error")]
    InternalError,
}

impl DrorisError {
    pub fn storage(kind: StorageError, message: impl Into<String>) -> Self {
        DrorisError::Storage {
            kind,
            tablet_id: None,
            message: message.into(),
        }
    }

    pub fn storage_with_tablet(
        kind: StorageError,
        tablet_id: u64,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Storage {
            kind,
            tablet_id: Some(tablet_id),
            message: message.into(),
        }
    }

    pub fn query(kind: QueryError, message: impl Into<String>) -> Self {
        DrorisError::Query {
            kind,
            query_id: None,
            message: message.into(),
        }
    }

    pub fn query_with_id(
        kind: QueryError,
        query_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Query {
            kind,
            query_id: Some(query_id.into()),
            message: message.into(),
        }
    }

    pub fn catalog(kind: CatalogError, message: impl Into<String>) -> Self {
        DrorisError::Catalog {
            kind,
            database_name: None,
            table_name: None,
            message: message.into(),
        }
    }

    pub fn catalog_with_db(
        kind: CatalogError,
        database_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Catalog {
            kind,
            database_name: Some(database_name.into()),
            table_name: None,
            message: message.into(),
        }
    }

    pub fn catalog_with_table(
        kind: CatalogError,
        database_name: impl Into<String>,
        table_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Catalog {
            kind,
            database_name: Some(database_name.into()),
            table_name: Some(table_name.into()),
            message: message.into(),
        }
    }

    pub fn parse(kind: ParseError, message: impl Into<String>) -> Self {
        DrorisError::Parse {
            kind,
            position: None,
            message: message.into(),
        }
    }

    pub fn parse_with_position(
        kind: ParseError,
        position: usize,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Parse {
            kind,
            position: Some(position),
            message: message.into(),
        }
    }

    pub fn plan(kind: PlanError, message: impl Into<String>) -> Self {
        DrorisError::Plan {
            kind,
            message: message.into(),
        }
    }

    pub fn rpc(kind: RpcError, message: impl Into<String>) -> Self {
        DrorisError::Rpc {
            kind,
            endpoint: None,
            message: message.into(),
        }
    }

    pub fn rpc_with_endpoint(
        kind: RpcError,
        endpoint: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        DrorisError::Rpc {
            kind,
            endpoint: Some(endpoint.into()),
            message: message.into(),
        }
    }
}
