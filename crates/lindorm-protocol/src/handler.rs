//! Lindorm command handler (HBase-like API)

use crate::storage::LindormStorage;
use std::collections::HashMap;
use std::sync::Arc;

pub struct LindormHandler {
    storage: Arc<LindormStorage>,
}

impl LindormHandler {
    pub fn new(storage: Arc<LindormStorage>) -> Self {
        Self { storage }
    }

    pub fn handle_command(&self, command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return "ERROR: Empty command".to_string();
        }

        match parts[0].to_uppercase().as_str() {
            "CREATE" => self.handle_create(&parts),
            "PUT" => self.handle_put(&parts),
            "GET" => self.handle_get(&parts),
            "DELETE" => self.handle_delete(&parts),
            "SCAN" => self.handle_scan(&parts),
            "LIST" => self.handle_list(&parts),
            "COUNT" => self.handle_count(&parts),
            _ => "ERROR: Unknown command".to_string(),
        }
    }

    fn handle_create(&self, parts: &[&str]) -> String {
        if parts.len() < 3 || parts[1].to_uppercase() != "TABLE" {
            return "ERROR: Syntax: CREATE TABLE <name>".to_string();
        }
        let table_name = parts[2];
        self.storage.create_table(table_name);
        format!("OK: Table {} created", table_name)
    }

    fn handle_put(&self, parts: &[&str]) -> String {
        if parts.len() < 6 {
            return "ERROR: Syntax: PUT <table> <rowkey> <family> <qualifier> <value>".to_string();
        }
        let table_name = parts[1];
        let rowkey = parts[2];
        let family = parts[3];
        let qualifier = parts[4];
        let value = parts[5];

        if let Some(table) = self.storage.get_table(table_name) {
            table.put(rowkey, family, qualifier, value);
            format!("OK: Row {} inserted", rowkey)
        } else {
            format!("ERROR: Table {} not found", table_name)
        }
    }

    fn handle_get(&self, parts: &[&str]) -> String {
        if parts.len() < 3 {
            return "ERROR: Syntax: GET <table> <rowkey>".to_string();
        }
        let table_name = parts[1];
        let rowkey = parts[2];

        if let Some(table) = self.storage.get_table(table_name) {
            if let Some(row) = table.get(rowkey) {
                let mut result = format!("ROW: {}\n", rowkey);
                for (family, qualifiers) in row {
                    for (qualifier, value) in qualifiers {
                        result.push_str(&format!("  {}: {} = {}\n", family, qualifier, value));
                    }
                }
                result
            } else {
                format!("ERROR: Row {} not found", rowkey)
            }
        } else {
            format!("ERROR: Table {} not found", table_name)
        }
    }

    fn handle_delete(&self, parts: &[&str]) -> String {
        if parts.len() < 3 {
            return "ERROR: Syntax: DELETE <table> <rowkey>".to_string();
        }
        let table_name = parts[1];
        let rowkey = parts[2];

        if let Some(table) = self.storage.get_table(table_name) {
            if table.delete(rowkey) {
                format!("OK: Row {} deleted", rowkey)
            } else {
                format!("ERROR: Row {} not found", rowkey)
            }
        } else {
            format!("ERROR: Table {} not found", table_name)
        }
    }

    fn handle_scan(&self, parts: &[&str]) -> String {
        if parts.len() < 4 {
            return "ERROR: Syntax: SCAN <table> <start_row> <end_row>".to_string();
        }
        let table_name = parts[1];
        let start_row = parts[2];
        let end_row = parts[3];

        if let Some(table) = self.storage.get_table(table_name) {
            let rows = table.scan(start_row, end_row);
            let mut result = String::new();
            for (rowkey, row) in rows {
                result.push_str(&format!("ROW: {}\n", rowkey));
                for (family, qualifiers) in row {
                    for (qualifier, value) in qualifiers {
                        result.push_str(&format!("  {}: {} = {}\n", family, qualifier, value));
                    }
                }
            }
            if result.is_empty() {
                "No rows found".to_string()
            } else {
                result
            }
        } else {
            format!("ERROR: Table {} not found", table_name)
        }
    }

    fn handle_list(&self, parts: &[&str]) -> String {
        let tables = self.storage.list_tables();
        if tables.is_empty() {
            "No tables found".to_string()
        } else {
            tables.join("\n")
        }
    }

    fn handle_count(&self, parts: &[&str]) -> String {
        if parts.len() < 2 {
            return "ERROR: Syntax: COUNT <table>".to_string();
        }
        let table_name = parts[1];

        if let Some(table) = self.storage.get_table(table_name) {
            format!("Count: {}", table.count())
        } else {
            format!("ERROR: Table {} not found", table_name)
        }
    }
}
