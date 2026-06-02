//! Oracle command handler

use crate::storage::OracleStorage;
use crate::tns::{TnsPacket, TnsPacketType};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for handling Oracle commands
pub trait OracleCommandHandler: Send + Sync {
    fn handle_connect(&self, service: &str) -> Vec<u8>;
    fn handle_query(&self, schema: &str, sql: &str) -> Vec<u8>;
}

/// Default Oracle command handler
pub struct DefaultOracleHandler {
    storage: Arc<OracleStorage>,
}

impl DefaultOracleHandler {
    pub fn new(storage: Arc<OracleStorage>) -> Self {
        Self { storage }
    }

    fn execute_sql(&self, schema: &str, sql: &str) -> String {
        let sql = sql.trim().trim_end_matches(';');
        let upper = sql.to_uppercase();

        if upper.starts_with("SELECT") {
            // Simple SELECT simulation
            if upper.contains("FROM DUAL") {
                return "DUMMY\n-----\nX\n".to_string();
            }

            if upper.contains("USER") {
                return format!("USER\n-----\n{}\n", schema);
            }

            if upper.contains("SYSDATE") {
                let now = chrono::Utc::now();
                return format!("SYSDATE\n-------\n{}\n", now.format("%Y-%m-%d %H:%M:%S"));
            }

            if upper.contains("VERSION") {
                return "VERSION\n-------\n19.0.0.0.0\n".to_string();
            }
        }

        if upper.starts_with("SHOW") && upper.contains("USER") {
            return format!("USER is \"{}\"", schema);
        }

        "Statement processed".to_string()
    }
}

impl OracleCommandHandler for DefaultOracleHandler {
    fn handle_connect(&self, service: &str) -> Vec<u8> {
        // Return ACCEPT packet
        let accept_data = vec![
            0x00, 0x00, // Version
            0x00, 0x00, // Compatible
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Options
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let packet = TnsPacket::new(TnsPacketType::Accept, accept_data);
        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf);
        buf.to_vec()
    }

    fn handle_query(&self, schema: &str, sql: &str) -> Vec<u8> {
        let result = self.execute_sql(schema, sql);

        // Return DATA packet with result
        let packet = TnsPacket::new(TnsPacketType::Data, result.into_bytes());
        let mut buf = bytes::BytesMut::new();
        packet.encode(&mut buf);
        buf.to_vec()
    }
}
