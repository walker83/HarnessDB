//! InfluxDB HTTP API command handler

use crate::line_protocol::LineProtocolParser;
use crate::storage::InfluxDBStorage;
use std::sync::Arc;

/// Trait for handling InfluxDB commands
pub trait InfluxDBCommandHandler: Send + Sync {
    fn handle_write(&self, database: &str, body: &str) -> Result<(), String>;
    fn handle_query(&self, database: &str, query: &str) -> String;
}

/// Default InfluxDB command handler
pub struct DefaultInfluxDBHandler {
    storage: Arc<InfluxDBStorage>,
}

impl DefaultInfluxDBHandler {
    pub fn new(storage: Arc<InfluxDBStorage>) -> Self {
        Self { storage }
    }
}

impl InfluxDBCommandHandler for DefaultInfluxDBHandler {
    fn handle_write(&self, database: &str, body: &str) -> Result<(), String> {
        let db = self.storage.get_database(database);

        // Parse each line
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Some(point) = LineProtocolParser::parse(line) {
                let measurement = db.get_measurement(&point.measurement);
                measurement.write(point);
            }
        }

        Ok(())
    }

    fn handle_query(&self, database: &str, query: &str) -> String {
        let query = query.trim().trim_end_matches(';');
        let upper = query.to_uppercase();

        // Simple query parser
        if upper.starts_with("SHOW DATABASES") {
            let dbs = self.storage.list_databases();
            let results: Vec<String> = dbs.into_iter().map(|db| format!("name: {}", db)).collect();
            return format!("results:\n{}", results.join("\n"));
        }

        if upper.starts_with("SHOW MEASUREMENTS") {
            let db = self.storage.get_database(database);
            let measurements = db.list_measurements();
            let results: Vec<String> = measurements.into_iter().map(|m| format!("name: {}", m)).collect();
            return format!("results:\n{}", results.join("\n"));
        }

        if upper.starts_with("CREATE DATABASE") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            if parts.len() >= 3 {
                let db_name = parts[2];
                self.storage.create_database(db_name);
                return "OK".to_string();
            }
        }

        if upper.starts_with("DROP DATABASE") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            if parts.len() >= 3 {
                let db_name = parts[2];
                self.storage.drop_database(db_name);
                return "OK".to_string();
            }
        }

        if upper.starts_with("SELECT") {
            // Simple SELECT parser - extract measurement name
            if let Some(from_pos) = upper.find("FROM") {
                let after_from = &query[from_pos + 4..].trim();
                let measurement_name = after_from.split_whitespace().next().unwrap_or("");

                let db = self.storage.get_database(database);
                let measurement = db.get_measurement(measurement_name);

                // Parse time range if present
                let mut start = None;
                let mut end = None;

                if let Some(where_pos) = upper.find("WHERE") {
                    let where_clause = &query[where_pos + 5..];
                    // Simple time range parsing (very basic)
                    if where_clause.to_uppercase().contains("TIME") {
                        // In a real implementation, parse time conditions
                    }
                }

                let points = measurement.query(start, end);

                // Format as InfluxDB line protocol
                let mut results = Vec::new();
                for point in points {
                    let fields: Vec<String> = point.fields
                        .iter()
                        .map(|(k, v)| {
                            let value = match v {
                                crate::line_protocol::FieldValue::Float(f) => f.to_string(),
                                crate::line_protocol::FieldValue::Integer(i) => format!("{}i", i),
                                crate::line_protocol::FieldValue::String(s) => format!("\"{}\"", s),
                                crate::line_protocol::FieldValue::Boolean(b) => if *b { "t" } else { "f" }.to_string(),
                            };
                            format!("{}={}", k, value)
                        })
                        .collect();

                    let tags: Vec<String> = point.tags
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();

                    let mut line = measurement_name.to_string();
                    if !tags.is_empty() {
                        line.push(',');
                        line.push_str(&tags.join(","));
                    }
                    line.push(' ');
                    line.push_str(&fields.join(","));
                    line.push(' ');
                    line.push_str(&point.timestamp.to_string());
                    results.push(line);
                }

                return results.join("\n");
            }
        }

        "Error: Unsupported query".to_string()
    }
}
