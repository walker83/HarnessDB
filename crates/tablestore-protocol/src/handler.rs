//! TableStore REST API command handler

use crate::storage::{AttributeValue, Row, TableSchema, TableStoreStorage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Trait for handling TableStore commands
pub trait TableStoreCommandHandler: Send + Sync {
    fn handle_request(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String);
}

/// Default TableStore command handler
pub struct DefaultTableStoreHandler {
    storage: Arc<TableStoreStorage>,
}

#[derive(Serialize, Deserialize)]
struct CreateTableRequest {
    table_name: String,
    primary_key: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct PutRowRequest {
    primary_key: HashMap<String, serde_json::Value>,
    attributes: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct GetRowRequest {
    primary_key: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct GetRangeRequest {
    start: HashMap<String, serde_json::Value>,
    end: HashMap<String, serde_json::Value>,
    limit: Option<usize>,
}

impl DefaultTableStoreHandler {
    pub fn new(storage: Arc<TableStoreStorage>) -> Self {
        Self { storage }
    }

    fn json_to_attribute_value(v: &serde_json::Value) -> AttributeValue {
        match v {
            serde_json::Value::String(s) => AttributeValue::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    AttributeValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    AttributeValue::Double(f)
                } else {
                    AttributeValue::String(n.to_string())
                }
            }
            serde_json::Value::Bool(b) => AttributeValue::Boolean(*b),
            _ => AttributeValue::String(v.to_string()),
        }
    }

    fn convert_primary_key(pk: &HashMap<String, serde_json::Value>) -> BTreeMap<String, AttributeValue> {
        pk.iter()
            .map(|(k, v)| (k.clone(), Self::json_to_attribute_value(v)))
            .collect()
    }

    fn convert_attributes(attrs: &HashMap<String, serde_json::Value>) -> HashMap<String, AttributeValue> {
        attrs
            .iter()
            .map(|(k, v)| (k.clone(), Self::json_to_attribute_value(v)))
            .collect()
    }
}

impl TableStoreCommandHandler for DefaultTableStoreHandler {
    fn handle_request(&self, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
        let path_parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        match (method, path_parts.as_slice()) {
            // GET / - List instances
            ("GET", [""]) => {
                let instances = self.storage.list_instances();
                let response = serde_json::json!({
                    "instances": instances
                });
                (200, serde_json::to_string(&response).unwrap())
            }

            // POST /{instance}/tables - Create table
            ("POST", [instance, "tables"]) if path_parts.len() == 2 => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<CreateTableRequest>(body_str) {
                        let inst = self.storage.get_instance(instance);
                        let schema = TableSchema::new(req.primary_key);
                        inst.create_table(&req.table_name, schema);
                        let response = serde_json::json!({
                            "status": "success",
                            "table_name": req.table_name
                        });
                        return (200, serde_json::to_string(&response).unwrap());
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // GET /{instance}/tables - List tables
            ("GET", [instance, "tables"]) if path_parts.len() == 2 => {
                let inst = self.storage.get_instance(instance);
                let tables = inst.list_tables();
                let response = serde_json::json!({
                    "tables": tables
                });
                (200, serde_json::to_string(&response).unwrap())
            }

            // DELETE /{instance}/tables/{table} - Delete table
            ("DELETE", [instance, "tables", table]) if path_parts.len() == 3 => {
                let inst = self.storage.get_instance(instance);
                if inst.delete_table(table) {
                    (200, r#"{"status": "success"}"#.to_string())
                } else {
                    (404, r#"{"error": "Table not found"}"#.to_string())
                }
            }

            // PUT /{instance}/{table}/row - Put row
            ("PUT", [instance, table, "row"]) if path_parts.len() == 3 => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<PutRowRequest>(body_str) {
                        let inst = self.storage.get_instance(instance);
                        if let Some(tbl) = inst.get_table(table) {
                            let primary_key = Self::convert_primary_key(&req.primary_key);
                            let attributes = Self::convert_attributes(&req.attributes);
                            let row = Row {
                                primary_key,
                                attributes,
                            };
                            tbl.put_row(row);
                            return (200, r#"{"status": "success"}"#.to_string());
                        } else {
                            return (404, r#"{"error": "Table not found"}"#.to_string());
                        }
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // POST /{instance}/{table}/row - Get row
            ("POST", [instance, table, "row"]) if path_parts.len() == 3 => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<GetRowRequest>(body_str) {
                        let inst = self.storage.get_instance(instance);
                        if let Some(tbl) = inst.get_table(table) {
                            let primary_key = Self::convert_primary_key(&req.primary_key);
                            if let Some(row) = tbl.get_row(&primary_key) {
                                let response = serde_json::json!({
                                    "primary_key": row.primary_key,
                                    "attributes": row.attributes
                                });
                                return (200, serde_json::to_string(&response).unwrap());
                            } else {
                                return (404, r#"{"error": "Row not found"}"#.to_string());
                            }
                        } else {
                            return (404, r#"{"error": "Table not found"}"#.to_string());
                        }
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // DELETE /{instance}/{table}/row - Delete row
            ("DELETE", [instance, table, "row"]) if path_parts.len() == 3 => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<GetRowRequest>(body_str) {
                        let inst = self.storage.get_instance(instance);
                        if let Some(tbl) = inst.get_table(table) {
                            let primary_key = Self::convert_primary_key(&req.primary_key);
                            if tbl.delete_row(&primary_key) {
                                return (200, r#"{"status": "success"}"#.to_string());
                            } else {
                                return (404, r#"{"error": "Row not found"}"#.to_string());
                            }
                        } else {
                            return (404, r#"{"error": "Table not found"}"#.to_string());
                        }
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // POST /{instance}/{table}/range - Get range
            ("POST", [instance, table, "range"]) if path_parts.len() == 3 => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<GetRangeRequest>(body_str) {
                        let inst = self.storage.get_instance(instance);
                        if let Some(tbl) = inst.get_table(table) {
                            let start = Self::convert_primary_key(&req.start);
                            let end = Self::convert_primary_key(&req.end);
                            let limit = req.limit.unwrap_or(100);
                            let rows = tbl.get_range(&start, &end, limit);
                            let response = serde_json::json!({
                                "rows": rows
                            });
                            return (200, serde_json::to_string(&response).unwrap());
                        } else {
                            return (404, r#"{"error": "Table not found"}"#.to_string());
                        }
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            _ => (404, r#"{"error": "Not found"}"#.to_string()),
        }
    }
}
