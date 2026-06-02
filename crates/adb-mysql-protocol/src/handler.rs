//! AnalyticDB MySQL command handler

use crate::storage::AdbMysqlStorage;
use std::sync::Arc;

pub struct AdbMysqlHandler {
    storage: Arc<AdbMysqlStorage>,
}

impl AdbMysqlHandler {
    pub fn new(storage: Arc<AdbMysqlStorage>) -> Self {
        Self { storage }
    }

    pub fn handle_query(&self, database: &str, query: &str) -> String {
        let query_upper = query.to_uppercase();

        if query_upper.starts_with("SELECT") {
            self.handle_select(database, query)
        } else if query_upper.starts_with("INSERT") {
            self.handle_insert(database, query)
        } else if query_upper.starts_with("CREATE") {
            self.handle_create(database, query)
        } else if query_upper.starts_with("SHOW") {
            self.handle_show(database, query)
        } else {
            "OK".to_string()
        }
    }

    fn handle_select(&self, database: &str, query: &str) -> String {
        // Parse table name from query (simplified)
        if let Some(table_name) = self.extract_table_name(query) {
            if let Some(db) = self.storage.get_database(database) {
                if let Some(table) = db.get_table(&table_name) {
                    let rows = table.select_all();
                    let columns = table.columns();

                    let mut result = String::new();
                    result.push_str(&columns.join("\t"));
                    result.push('\n');

                    for row in rows {
                        result.push_str(&row.join("\t"));
                        result.push('\n');
                    }
                    return result;
                }
            }
        }
        String::new()
    }

    fn handle_insert(&self, database: &str, query: &str) -> String {
        "OK".to_string()
    }

    fn handle_create(&self, database: &str, query: &str) -> String {
        let query_upper = query.to_uppercase();
        if query_upper.contains("DATABASE") {
            if let Some(db_name) = self.extract_name(query, "DATABASE") {
                self.storage.create_database(&db_name);
            }
        } else if query_upper.contains("TABLE") {
            if let Some(db) = self.storage.get_database(database) {
                if let Some(table_name) = self.extract_name(query, "TABLE") {
                    // Extract columns (simplified)
                    let columns = vec!["id".to_string(), "data".to_string()];
                    db.create_table(&table_name, columns);
                }
            }
        }
        "OK".to_string()
    }

    fn handle_show(&self, database: &str, query: &str) -> String {
        let query_upper = query.to_uppercase();
        if query_upper.contains("DATABASES") {
            self.storage.list_databases().join("\n")
        } else if query_upper.contains("TABLES") {
            if let Some(db) = self.storage.get_database(database) {
                db.list_tables().join("\n")
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    fn extract_table_name(&self, query: &str) -> Option<String> {
        let query_upper = query.to_uppercase();
        if let Some(from_pos) = query_upper.find("FROM") {
            let after_from = &query[from_pos + 4..].trim();
            let table_name = after_from.split_whitespace().next()?;
            Some(table_name.to_string())
        } else {
            None
        }
    }

    fn extract_name(&self, query: &str, keyword: &str) -> Option<String> {
        let query_upper = query.to_uppercase();
        if let Some(pos) = query_upper.find(keyword) {
            let after_keyword = &query[pos + keyword.len()..].trim();
            let name = after_keyword.split_whitespace().next()?;
            Some(name.trim_end_matches(';').to_string())
        } else {
            None
        }
    }
}
