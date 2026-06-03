//! Elasticsearch REST API command handler

use crate::storage::{Document, ElasticsearchStorage};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for handling Elasticsearch commands
pub trait ElasticsearchCommandHandler: Send + Sync {
    fn handle_request(&self, method: &str, path: &str, body: Option<&str>) -> serde_json::Value;
}

/// Default Elasticsearch command handler
pub struct DefaultElasticsearchHandler {
    storage: Arc<ElasticsearchStorage>,
}

impl DefaultElasticsearchHandler {
    pub fn new(storage: Arc<ElasticsearchStorage>) -> Self {
        Self { storage }
    }
}

impl ElasticsearchCommandHandler for DefaultElasticsearchHandler {
    fn handle_request(&self, method: &str, path: &str, body: Option<&str>) -> serde_json::Value {
        let path_parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        match (method, path_parts.as_slice()) {
            // GET / - Cluster health
            ("GET", [""]) | ("GET", []) => {
                json!({
                    "name": "harness",
                    "cluster_name": "harness-cluster",
                    "cluster_uuid": "harness-uuid",
                    "version": {
                        "number": "7.10.2",
                        "build_flavor": "default",
                        "build_type": "docker",
                        "build_hash": "harness",
                        "build_date": "2024-01-01",
                        "build_snapshot": false,
                        "lucene_version": "8.7.0",
                        "minimum_wire_compatibility_version": "6.8.0",
                        "minimum_index_compatibility_version": "6.0.0-beta1"
                    },
                    "tagline": "You Know, for Search"
                })
            }

            // GET /_cluster/health
            ("GET", ["_cluster", "health"]) => {
                json!({
                    "cluster_name": "harness-cluster",
                    "status": "green",
                    "timed_out": false,
                    "number_of_nodes": 1,
                    "number_of_data_nodes": 1,
                    "active_primary_shards": 0,
                    "active_shards": 0,
                    "relocating_shards": 0,
                    "initializing_shards": 0,
                    "unassigned_shards": 0
                })
            }

            // GET /_cat/indices
            ("GET", ["_cat", "indices"]) => {
                let indices = self.storage.list_indices();
                let result: Vec<String> = indices
                    .iter()
                    .map(|name| {
                        let index = self.storage.get_index(name).unwrap();
                        format!("green open {} harness-uuid 1 0 {} 0 0b 0b", name, index.count())
                    })
                    .collect();
                json!(result.join("\n").to_string())
            }

            // PUT /{index} - Create index
            ("PUT", [index_name]) if path_parts.len() == 1 => {
                self.storage.create_index(index_name);
                json!({
                    "acknowledged": true,
                    "shards_acknowledged": true,
                    "index": index_name
                })
            }

            // DELETE /{index} - Delete index
            ("DELETE", [index_name]) if path_parts.len() == 1 => {
                let deleted = self.storage.delete_index(index_name);
                if deleted {
                    json!({"acknowledged": true})
                } else {
                    json!({
                        "error": {
                            "root_cause": [{
                                "type": "index_not_found_exception",
                                "reason": "no such index"
                            }],
                            "type": "index_not_found_exception",
                            "reason": "no such index"
                        },
                        "status": 404
                    })
                }
            }

            // GET /{index} - Get index info
            ("GET", [index_name]) if path_parts.len() == 1 => {
                if self.storage.index_exists(index_name) {
                    let mut result = serde_json::Map::new();
                    result.insert(
                        index_name.to_string(),
                        json!({
                            "aliases": {},
                            "mappings": {},
                            "settings": {
                                "index": {
                                    "number_of_shards": "1",
                                    "number_of_replicas": "0"
                                }
                            }
                        }),
                    );
                    json!(result)
                } else {
                    json!({
                        "error": {
                            "root_cause": [{
                                "type": "index_not_found_exception",
                                "reason": "no such index"
                            }],
                            "type": "index_not_found_exception",
                            "reason": "no such index"
                        },
                        "status": 404
                    })
                }
            }

            // PUT /{index}/_doc/{id} - Index document
            ("PUT", [index_name, "_doc", id]) if path_parts.len() == 3 => {
                let index = self.storage.get_index(index_name);
                if let Some(idx) = index {
                    if let Some(body_str) = body {
                        if let Ok(doc) = serde_json::from_str::<HashMap<String, serde_json::Value>>(body_str) {
                            let document = Document { fields: doc };
                            idx.index_document(id.to_string(), document);
                            return json!({
                                "_index": index_name,
                                "_type": "_doc",
                                "_id": id,
                                "_version": 1,
                                "result": "created",
                                "_shards": {
                                    "total": 2,
                                    "successful": 1,
                                    "failed": 0
                                },
                                "_seq_no": 0,
                                "_primary_term": 1
                            });
                        }
                    }
                }
                json!({"error": "Invalid request"})
            }

            // POST /{index}/_doc - Index document with auto-generated ID
            ("POST", [index_name, "_doc"]) if path_parts.len() == 2 => {
                let index = self.storage.get_index(index_name);
                if let Some(idx) = index {
                    if let Some(body_str) = body {
                        if let Ok(doc) = serde_json::from_str::<HashMap<String, serde_json::Value>>(body_str) {
                            let document = Document { fields: doc };
                            let id = uuid::Uuid::new_v4().to_string();
                            idx.index_document(id.clone(), document);
                            return json!({
                                "_index": index_name,
                                "_type": "_doc",
                                "_id": id,
                                "_version": 1,
                                "result": "created",
                                "_shards": {
                                    "total": 2,
                                    "successful": 1,
                                    "failed": 0
                                },
                                "_seq_no": 0,
                                "_primary_term": 1
                            });
                        }
                    }
                }
                json!({"error": "Invalid request"})
            }

            // GET /{index}/_doc/{id} - Get document
            ("GET", [index_name, "_doc", id]) if path_parts.len() == 3 => {
                if let Some(index) = self.storage.get_index(index_name) {
                    if let Some(doc) = index.get_document(id) {
                        return json!({
                            "_index": index_name,
                            "_type": "_doc",
                            "_id": id,
                            "_version": 1,
                            "_seq_no": 0,
                            "_primary_term": 1,
                            "found": true,
                            "_source": doc.fields
                        });
                    }
                }
                json!({
                    "_index": index_name,
                    "_type": "_doc",
                    "_id": id,
                    "found": false
                })
            }

            // DELETE /{index}/_doc/{id} - Delete document
            ("DELETE", [index_name, "_doc", id]) if path_parts.len() == 3 => {
                if let Some(index) = self.storage.get_index(index_name) {
                    let deleted = index.delete_document(id);
                    return json!({
                        "_index": index_name,
                        "_type": "_doc",
                        "_id": id,
                        "_version": 2,
                        "result": if deleted { "deleted" } else { "not_found" },
                        "_shards": {
                            "total": 2,
                            "successful": 1,
                            "failed": 0
                        },
                        "_seq_no": 1,
                        "_primary_term": 1
                    });
                }
                json!({"error": "Index not found"})
            }

            // POST /{index}/_search - Search
            ("POST" | "GET", [index_name, "_search"]) if path_parts.len() == 2 => {
                if let Some(index) = self.storage.get_index(index_name) {
                    let query = body
                        .and_then(|b| serde_json::from_str(b).ok())
                        .unwrap_or(json!({"query": {"match_all": {}}}));

                    let results = index.search(&query);
                    let hits: Vec<serde_json::Value> = results
                        .iter()
                        .map(|(id, doc)| {
                            json!({
                                "_index": index_name,
                                "_type": "_doc",
                                "_id": id,
                                "_score": 1.0,
                                "_source": doc.fields
                            })
                        })
                        .collect();

                    return json!({
                        "took": 1,
                        "timed_out": false,
                        "_shards": {
                            "total": 1,
                            "successful": 1,
                            "skipped": 0,
                            "failed": 0
                        },
                        "hits": {
                            "total": {
                                "value": hits.len(),
                                "relation": "eq"
                            },
                            "max_score": 1.0,
                            "hits": hits
                        }
                    });
                }
                json!({"error": "Index not found"})
            }

            _ => {
                json!({
                    "error": {
                        "root_cause": [{
                            "type": "invalid_index_name_exception",
                            "reason": "Invalid index name"
                        }],
                        "type": "invalid_index_name_exception",
                        "reason": "Invalid index name"
                    },
                    "status": 400
                })
            }
        }
    }
}
