//! TableStore REST API command handler

use crate::storage::{AttributeValue, ColumnDef, Row, TableSchema, TableStoreStorage};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Trait for handling TableStore commands
pub trait TableStoreCommandHandler: Send + Sync {
    fn handle_request(&self, method: &str, path: &str, query: &str, body: Option<&str>) -> (u16, String);
}

/// Default TableStore command handler
pub struct DefaultTableStoreHandler {
    storage: Arc<TableStoreStorage>,
}

// ── Request types matching the test JSON format ──

#[derive(Deserialize)]
struct ColumnDefRequest {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
}

#[derive(Deserialize)]
struct CreateTableRequest {
    primary_key: Vec<ColumnDefRequest>,
    #[serde(default)]
    defined_columns: Vec<ColumnDefRequest>,
}

#[derive(Deserialize)]
struct NameValue {
    name: String,
    value: serde_json::Value,
}

#[derive(Deserialize)]
struct PutRowRequest {
    primary_key: Vec<NameValue>,
    attributes: Vec<NameValue>,
}

#[derive(Deserialize)]
struct UpdateRowRequest {
    primary_key: Vec<NameValue>,
    attributes: Vec<NameValue>,
}

#[derive(Deserialize)]
struct GetRangeRequest {
    start: HashMap<String, serde_json::Value>,
    end: HashMap<String, serde_json::Value>,
    limit: Option<usize>,
}

/// Default instance name used since the test has no instance prefix in paths.
const DEFAULT_INSTANCE: &str = "default";

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

    /// Convert array-of-{name,value} to BTreeMap (used for primary keys).
    fn pairs_to_primary_key(pairs: &[NameValue]) -> BTreeMap<String, AttributeValue> {
        pairs
            .iter()
            .map(|nv| (nv.name.clone(), Self::json_to_attribute_value(&nv.value)))
            .collect()
    }

    /// Convert array-of-{name,value} to HashMap (used for attributes).
    fn pairs_to_attributes(pairs: &[NameValue]) -> HashMap<String, AttributeValue> {
        pairs
            .iter()
            .map(|nv| (nv.name.clone(), Self::json_to_attribute_value(&nv.value)))
            .collect()
    }

    /// Convert HashMap to BTreeMap (used for range queries where body already uses map form).
    fn map_to_primary_key(map: &HashMap<String, serde_json::Value>) -> BTreeMap<String, AttributeValue> {
        map.iter()
            .map(|(k, v)| (k.clone(), Self::json_to_attribute_value(v)))
            .collect()
    }

    /// Parse a primary key from a URL query string like `id=1&name=foo`.
    fn parse_pk_from_query(query: &str) -> Option<BTreeMap<String, AttributeValue>> {
        if query.is_empty() {
            return None;
        }
        let mut pk = BTreeMap::new();
        for part in query.split('&') {
            if let Some((k, v)) = part.split_once('=') {
                if !k.is_empty() {
                    let attr_val = if let Ok(i) = v.parse::<i64>() {
                        AttributeValue::Integer(i)
                    } else if let Ok(f) = v.parse::<f64>() {
                        AttributeValue::Double(f)
                    } else {
                        AttributeValue::String(v.to_string())
                    };
                    pk.insert(k.to_string(), attr_val);
                }
            }
        }
        if pk.is_empty() { None } else { Some(pk) }
    }
}

impl TableStoreCommandHandler for DefaultTableStoreHandler {
    fn handle_request(&self, method: &str, path: &str, query: &str, body: Option<&str>) -> (u16, String) {
        let path_parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        match (method, path_parts.as_slice()) {
            // ── GET / ── server info ──
            ("GET", [""]) => {
                let instances = self.storage.list_instances();
                let response = serde_json::json!({
                    "status": "ok",
                    "protocol": "tablestore",
                    "instances": instances
                });
                (200, serde_json::to_string(&response).unwrap())
            }

            // ── GET /tables ── list tables ──
            ("GET", ["tables"]) => {
                let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                let tables = inst.list_tables();
                let response = serde_json::json!({ "tables": tables });
                (200, serde_json::to_string(&response).unwrap())
            }

            // ── GET /tables/{name} ── describe table ──
            ("GET", ["tables", name]) => {
                let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                if let Some(tbl) = inst.get_table(name) {
                    let schema = tbl.schema();
                    let response = serde_json::json!({
                        "table_name": name,
                        "status": "ACTIVE",
                        "primary_key": schema.primary_key,
                        "defined_columns": schema.defined_columns,
                        "row_count": tbl.count()
                    });
                    (200, serde_json::to_string(&response).unwrap())
                } else {
                    (404, r#"{"error": "Table not found"}"#.to_string())
                }
            }

            // ── PUT /tables/{name} ── create table ──
            ("PUT", ["tables", name]) => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<CreateTableRequest>(body_str) {
                        let pk_cols: Vec<ColumnDef> = req.primary_key.into_iter()
                            .map(|c| ColumnDef { name: c.name, type_name: c.type_name })
                            .collect();
                        let def_cols: Vec<ColumnDef> = req.defined_columns.into_iter()
                            .map(|c| ColumnDef { name: c.name, type_name: c.type_name })
                            .collect();
                        let schema = TableSchema::new(pk_cols, def_cols);
                        let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                        inst.create_table(name, schema);
                        let response = serde_json::json!({
                            "status": "success",
                            "table_name": name
                        });
                        return (200, serde_json::to_string(&response).unwrap());
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // ── DELETE /tables/{name} ── delete table ──
            ("DELETE", ["tables", name]) => {
                let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                if inst.delete_table(name) {
                    (200, r#"{"status": "success"}"#.to_string())
                } else {
                    (404, r#"{"error": "Table not found"}"#.to_string())
                }
            }

            // ── POST /tables/{name}/rows ── put row ──
            ("POST", ["tables", name, "rows"]) => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<PutRowRequest>(body_str) {
                        let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                        if let Some(tbl) = inst.get_table(name) {
                            let primary_key = Self::pairs_to_primary_key(&req.primary_key);
                            let attributes = Self::pairs_to_attributes(&req.attributes);
                            tbl.put_row(Row { primary_key, attributes });
                            return (200, r#"{"status": "success"}"#.to_string());
                        } else {
                            return (404, r#"{"error": "Table not found"}"#.to_string());
                        }
                    }
                }
                (400, r#"{"error": "Invalid request"}"#.to_string())
            }

            // ── GET /tables/{name}/rows?id=… ── get row by PK from query string ──
            ("GET", ["tables", name, "rows"]) => {
                if let Some(pk) = Self::parse_pk_from_query(query) {
                    let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                    if let Some(tbl) = inst.get_table(name) {
                        if let Some(row) = tbl.get_row(&pk) {
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
                (400, r#"{"error": "Missing primary key in query string"}"#.to_string())
            }

            // ── DELETE /tables/{name}/rows?id=… ── delete row by PK from query string ──
            ("DELETE", ["tables", name, "rows"]) => {
                if let Some(pk) = Self::parse_pk_from_query(query) {
                    let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                    if let Some(tbl) = inst.get_table(name) {
                        if tbl.delete_row(&pk) {
                            return (200, r#"{"status": "success"}"#.to_string());
                        } else {
                            return (404, r#"{"error": "Row not found"}"#.to_string());
                        }
                    } else {
                        return (404, r#"{"error": "Table not found"}"#.to_string());
                    }
                }
                (400, r#"{"error": "Missing primary key in query string"}"#.to_string())
            }

            // ── POST /tables/{name}/rows/{id} ── update row ──
            ("POST", ["tables", name, "rows", _id]) => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<UpdateRowRequest>(body_str) {
                        let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                        if let Some(tbl) = inst.get_table(name) {
                            let primary_key = Self::pairs_to_primary_key(&req.primary_key);
                            let new_attrs = Self::pairs_to_attributes(&req.attributes);
                            if let Some(existing) = tbl.get_row(&primary_key) {
                                let mut merged = existing.attributes;
                                for (k, v) in new_attrs {
                                    merged.insert(k, v);
                                }
                                tbl.put_row(Row { primary_key, attributes: merged });
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

            // ── POST /tables/{name}/range ── get range ──
            ("POST", ["tables", name, "range"]) => {
                if let Some(body_str) = body {
                    if let Ok(req) = serde_json::from_str::<GetRangeRequest>(body_str) {
                        let inst = self.storage.get_instance(DEFAULT_INSTANCE);
                        if let Some(tbl) = inst.get_table(name) {
                            let start = Self::map_to_primary_key(&req.start);
                            let end = Self::map_to_primary_key(&req.end);
                            let limit = req.limit.unwrap_or(100);
                            let rows = tbl.get_range(&start, &end, limit);
                            let response = serde_json::json!({ "rows": rows });
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
