//! Cassandra command handler

use crate::frame::{Frame, Opcode};
use crate::storage::CassandraStorage;
use std::sync::Arc;

/// Trait for handling Cassandra commands
pub trait CassandraCommandHandler: Send + Sync {
    fn handle_startup(&self) -> Vec<u8>;
    fn handle_query(&self, keyspace: &str, cql: &str) -> Vec<u8>;
}

/// Default Cassandra command handler
pub struct DefaultCassandraHandler {
    storage: Arc<CassandraStorage>,
}

impl DefaultCassandraHandler {
    pub fn new(storage: Arc<CassandraStorage>) -> Self {
        Self { storage }
    }

    fn execute_cql(&self, keyspace: &str, cql: &str) -> String {
        let cql = cql.trim().trim_end_matches(';');
        let upper = cql.to_uppercase();

        if upper.starts_with("SELECT") {
            if upper.contains("FROM SYSTEM.LOCAL") {
                return "key|cluster_name|cql_version\n-----|------------|-------------\nlocal|HarnessDB|3.3.1\n".to_string();
            }
        }

        if upper.starts_with("CREATE KEYSPACE") {
            let parts: Vec<&str> = cql.split_whitespace().collect();
            if parts.len() >= 3 {
                let ks_name = parts[2];
                self.storage.create_keyspace(ks_name);
                return "Keyspace created".to_string();
            }
        }

        if upper.starts_with("USE") {
            return "Keyspace set".to_string();
        }

        "Statement executed".to_string()
    }
}

impl CassandraCommandHandler for DefaultCassandraHandler {
    fn handle_startup(&self) -> Vec<u8> {
        // Return READY frame
        let frame = Frame::new(0x84, 0, Opcode::Ready, vec![]);
        let mut buf = bytes::BytesMut::new();
        frame.encode(&mut buf);
        buf.to_vec()
    }

    fn handle_query(&self, keyspace: &str, cql: &str) -> Vec<u8> {
        let result = self.execute_cql(keyspace, cql);

        // Return RESULT frame (simplified)
        let result_body = result.into_bytes();
        let frame = Frame::new(0x84, 0, Opcode::Result, result_body);
        let mut buf = bytes::BytesMut::new();
        frame.encode(&mut buf);
        buf.to_vec()
    }
}
