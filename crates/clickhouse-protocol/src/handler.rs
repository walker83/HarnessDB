//! ClickHouse HTTP command handler

use crate::storage::ClickHouseStorage;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for handling ClickHouse commands
pub trait ClickHouseCommandHandler: Send + Sync {
    fn handle_query(&self, database: &str, query: &str) -> String;
}

/// Default ClickHouse command handler
pub struct DefaultClickHouseHandler {
    storage: Arc<ClickHouseStorage>,
}

impl DefaultClickHouseHandler {
    pub fn new(storage: Arc<ClickHouseStorage>) -> Self {
        Self { storage }
    }

    fn execute_query(&self, database: &str, query: &str) -> String {
        let query = query.trim().trim_end_matches(';');
        let upper = query.to_uppercase();

        // Simple SQL parser
        if upper.starts_with("SELECT") {
            self.handle_select(database, query)
        } else if upper.starts_with("INSERT") {
            self.handle_insert(database, query)
        } else if upper.starts_with("CREATE") {
            self.handle_create(database, query)
        } else if upper.starts_with("DROP") {
            self.handle_drop(database, query)
        } else if upper.starts_with("SHOW") {
            self.handle_show(database, query)
        } else if upper.starts_with("DESCRIBE") || upper.starts_with("DESC") {
            self.handle_describe(database, query)
        } else {
            "Error: Unsupported query".to_string()
        }
    }

    fn handle_select(&self, database: &str, query: &str) -> String {
        // Simple SELECT parser - extract table name
        let parts: Vec<&str> = query.split_whitespace().collect();
        if parts.len() < 4 {
            return "Error: Invalid SELECT syntax".to_string();
        }

        // Find FROM clause
        let from_idx = parts.iter().position(|&p| p.to_uppercase() == "FROM");
        if from_idx.is_none() {
            // System queries
            if query.to_uppercase().contains("SELECT 1") {
                return "1\n".to_string();
            }
            if query.to_uppercase().contains("VERSION()") {
                return "23.8.1.1\n".to_string();
            }
            return "Error: Missing FROM clause".to_string();
        }

        let table_name = parts[from_idx.unwrap() + 1];
        let db = self.storage.get_database(database);

        if let Some(table) = db.get_table(table_name) {
            let rows = table.select_all();
            let columns = table.columns();

            if rows.is_empty() {
                return String::new();
            }

            // Format as TSV (ClickHouse default format)
            let mut result = String::new();
            for row in rows {
                let values: Vec<String> = columns
                    .keys()
                    .filter_map(|col| row.get(col))
                    .cloned()
                    .collect();
                result.push_str(&values.join("\t"));
                result.push('\n');
            }
            result
        } else {
            format!("Error: Table {} not found", table_name)
        }
    }

    fn handle_insert(&self, database: &str, query: &str) -> String {
        // Simple INSERT parser
        // INSERT INTO table (col1, col2) VALUES (val1, val2)
        let upper = query.to_uppercase();
        if !upper.contains("INTO") || !upper.contains("VALUES") {
            return "Error: Invalid INSERT syntax".to_string();
        }

        // Extract table name
        let into_idx = upper.find("INTO").unwrap() + 4;
        let after_into = &query[into_idx..].trim();
        let parts: Vec<&str> = after_into.split_whitespace().collect();
        if parts.is_empty() {
            return "Error: Missing table name".to_string();
        }
        let table_name = parts[0].trim_end_matches('(');

        let db = self.storage.get_database(database);
        if db.get_table(table_name).is_none() {
            return format!("Error: Table {} not found", table_name);
        }

        // For now, just return OK
        "OK\n".to_string()
    }

    fn handle_create(&self, database: &str, query: &str) -> String {
        let upper = query.to_uppercase();
        if !upper.contains("TABLE") {
            return "Error: Only CREATE TABLE supported".to_string();
        }

        // Extract table name
        let table_idx = upper.find("TABLE").unwrap() + 5;
        let after_table = &query[table_idx..].trim();
        let parts: Vec<&str> = after_table.split_whitespace().collect();
        if parts.is_empty() {
            return "Error: Missing table name".to_string();
        }

        // Handle IF NOT EXISTS
        let table_name = if parts[0].to_uppercase() == "IF" {
            if parts.len() > 3 {
                parts[3]
            } else {
                return "Error: Invalid CREATE TABLE syntax".to_string();
            }
        } else {
            parts[0]
        };

        let db = self.storage.get_database(database);
        db.create_table(table_name);

        "OK\n".to_string()
    }

    fn handle_drop(&self, database: &str, query: &str) -> String {
        let upper = query.to_uppercase();
        if !upper.contains("TABLE") {
            return "Error: Only DROP TABLE supported".to_string();
        }

        // Extract table name
        let table_idx = upper.find("TABLE").unwrap() + 5;
        let after_table = &query[table_idx..].trim();
        let parts: Vec<&str> = after_table.split_whitespace().collect();
        if parts.is_empty() {
            return "Error: Missing table name".to_string();
        }

        // Handle IF EXISTS
        let table_name = if parts[0].to_uppercase() == "IF" {
            if parts.len() > 2 {
                parts[2]
            } else {
                return "Error: Invalid DROP TABLE syntax".to_string();
            }
        } else {
            parts[0]
        };

        let db = self.storage.get_database(database);
        if db.drop_table(table_name) {
            "OK\n".to_string()
        } else {
            format!("Error: Table {} not found", table_name)
        }
    }

    fn handle_show(&self, database: &str, query: &str) -> String {
        let upper = query.to_uppercase();

        if upper.contains("DATABASES") {
            let dbs = self.storage.list_databases();
            dbs.join("\n") + "\n"
        } else if upper.contains("TABLES") {
            let db = self.storage.get_database(database);
            let tables = db.list_tables();
            tables.join("\n") + "\n"
        } else {
            "Error: Unsupported SHOW command".to_string()
        }
    }

    fn handle_describe(&self, database: &str, query: &str) -> String {
        // DESCRIBE TABLE table_name
        let parts: Vec<&str> = query.split_whitespace().collect();
        if parts.len() < 2 {
            return "Error: Invalid DESCRIBE syntax".to_string();
        }

        let table_name = parts.last().unwrap();
        let db = self.storage.get_database(database);

        if let Some(table) = db.get_table(table_name) {
            let mut result = String::new();
            for (col_name, col_type) in table.column_types() {
                result.push_str(&format!("{}\t{}\n", col_name, col_type));
            }
            result
        } else {
            format!("Error: Table {} not found", table_name)
        }
    }
}

impl ClickHouseCommandHandler for DefaultClickHouseHandler {
    fn handle_query(&self, database: &str, query: &str) -> String {
        self.execute_query(database, query)
    }
}
