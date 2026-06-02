//! MongoDB command handler

use crate::storage::MongoDBStorage;
use crate::wire::{Message, MessageHeader, MessagePayload, OpMsg, OpReply, Section};
use bson::{doc, Document};
use bytes::BytesMut;
use std::sync::Arc;

/// Trait for handling MongoDB commands
pub trait MongoDBCommandHandler: Send + Sync {
    fn handle_message(&self, message: &Message) -> Message;
}

/// Default MongoDB command handler
pub struct DefaultMongoDBHandler {
    storage: Arc<MongoDBStorage>,
}

impl DefaultMongoDBHandler {
    pub fn new(storage: Arc<MongoDBStorage>) -> Self {
        Self { storage }
    }

    fn handle_command(&self, db_name: &str, command: &Document) -> Document {
        // Extract command name (first field)
        let cmd_name = command.keys().next().map(|s| s.as_str()).unwrap_or("");

        match cmd_name {
            "ping" => doc! { "ok": 1 },
            "ismaster" | "hello" => self.cmd_ismaster(),
            "buildInfo" => self.cmd_build_info(),
            "serverStatus" => self.cmd_server_status(),
            "listDatabases" => self.cmd_list_databases(),
            "listCollections" => self.cmd_list_collections(db_name),
            "insert" => self.cmd_insert(db_name, command),
            "find" => self.cmd_find(db_name, command),
            "update" => self.cmd_update(db_name, command),
            "delete" => self.cmd_delete(db_name, command),
            "count" => self.cmd_count(db_name, command),
            "create" => self.cmd_create(db_name, command),
            "drop" => self.cmd_drop(db_name, command),
            "getLog" => self.cmd_get_log(command),
            "getMore" => doc! { "ok": 1, "cursor": { "id": 0, "ns": "", "nextBatch": [] } },
            _ => doc! { "ok": 0, "errmsg": format!("Unknown command: {}", cmd_name) },
        }
    }

    fn cmd_ismaster(&self) -> Document {
        doc! {
            "ismaster": true,
            "maxBsonObjectSize": 16777216,
            "maxMessageSizeBytes": 48000000,
            "maxWriteBatchSize": 100000,
            "localTime": bson::DateTime::now(),
            "maxWireVersion": 17,
            "minWireVersion": 0,
            "ok": 1
        }
    }

    fn cmd_build_info(&self) -> Document {
        doc! {
            "version": "7.0.0",
            "gitVersion": "roris",
            "modules": [],
            "allocator": "system",
            "bits": 64,
            "debug": false,
            "maxBsonObjectSize": 16777216,
            "ok": 1
        }
    }

    fn cmd_server_status(&self) -> Document {
        doc! {
            "host": "roris",
            "version": "7.0.0",
            "process": "roris",
            "pid": std::process::id() as i64,
            "uptime": 1i64,
            "ok": 1
        }
    }

    fn cmd_list_databases(&self) -> Document {
        let db_names = self.storage.list_databases();
        let databases: Vec<Document> = db_names
            .into_iter()
            .map(|name| {
                doc! {
                    "name": name,
                    "sizeOnDisk": 0i64,
                    "empty": true
                }
            })
            .collect();

        doc! {
            "databases": databases,
            "totalSize": 0i64,
            "ok": 1
        }
    }

    fn cmd_list_collections(&self, db_name: &str) -> Document {
        let db = self.storage.get_database(db_name);
        let collections = db.list_collections();

        let cursor_docs: Vec<Document> = collections
            .into_iter()
            .map(|name| doc! { "name": name, "type": "collection" })
            .collect();

        doc! {
            "cursor": {
                "id": 0i64,
                "ns": format!("{}.$cmd.listCollections", db_name),
                "firstBatch": cursor_docs
            },
            "ok": 1
        }
    }

    fn cmd_insert(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("insert") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let documents = match command.get_array("documents") {
            Ok(docs) => docs,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing documents array" },
        };

        let db = self.storage.get_database(db_name);
        let collection = db.get_collection(collection_name);

        let mut inserted = 0;
        for doc_value in documents {
            if let Ok(doc) = doc_value.as_document().ok_or(()) {
                let id = doc
                    .get_object_id("_id")
                    .map(|oid| oid.to_hex())
                    .unwrap_or_else(|_| bson::oid::ObjectId::new().to_hex());
                collection.insert(id, doc.clone());
                inserted += 1;
            }
        }

        doc! { "n": inserted, "ok": 1 }
    }

    fn cmd_find(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("find") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let filter = command.get_document("filter").ok();

        let db = self.storage.get_database(db_name);
        let collection = db.get_collection(collection_name);
        let documents = collection.find(filter);

        doc! {
            "cursor": {
                "id": 0i64,
                "ns": format!("{}.{}", db_name, collection_name),
                "firstBatch": documents
            },
            "ok": 1
        }
    }

    fn cmd_update(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("update") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let updates = match command.get_array("updates") {
            Ok(u) => u,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing updates array" },
        };

        let db = self.storage.get_database(db_name);
        let collection = db.get_collection(collection_name);

        let mut modified = 0;
        for update_value in updates {
            if let Some(update_doc) = update_value.as_document() {
                if let (Ok(filter), Ok(update)) = (
                    update_doc.get_document("q"),
                    update_doc.get_document("u"),
                ) {
                    // Simple update - find matching documents and update
                    let docs = collection.find(Some(filter));
                    for doc in docs {
                        if let Some(id) = doc.get_object_id("_id").ok() {
                            if collection.update(&id.to_hex(), update) {
                                modified += 1;
                            }
                        }
                    }
                }
            }
        }

        doc! { "n": modified, "nModified": modified, "ok": 1 }
    }

    fn cmd_delete(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("delete") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let deletes = match command.get_array("deletes") {
            Ok(d) => d,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing deletes array" },
        };

        let db = self.storage.get_database(db_name);
        let collection = db.get_collection(collection_name);

        let mut deleted = 0;
        for delete_value in deletes {
            if let Some(delete_doc) = delete_value.as_document() {
                if let Ok(filter) = delete_doc.get_document("q") {
                    let docs = collection.find(Some(filter));
                    for doc in docs {
                        if let Some(id) = doc.get_object_id("_id").ok() {
                            if collection.delete(&id.to_hex()) {
                                deleted += 1;
                            }
                        }
                    }
                }
            }
        }

        doc! { "n": deleted, "ok": 1 }
    }

    fn cmd_count(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("count") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let db = self.storage.get_database(db_name);
        let collection = db.get_collection(collection_name);
        let count = collection.count() as i64;

        doc! { "n": count, "ok": 1 }
    }

    fn cmd_create(&self, db_name: &str, command: &Document) -> Document {
        let collection_name = match command.get_str("create") {
            Ok(name) => name,
            Err(_) => return doc! { "ok": 0, "errmsg": "Missing collection name" },
        };

        let db = self.storage.get_database(db_name);
        db.get_collection(collection_name); // Creates if doesn't exist

        doc! { "ok": 1 }
    }

    fn cmd_drop(&self, db_name: &str, command: &Document) -> Document {
        // Simplified - just return OK
        doc! { "ok": 1 }
    }

    fn cmd_get_log(&self, command: &Document) -> Document {
        let log_name = command.get_str("getLog").unwrap_or("global");

        let log_lines = match log_name {
            "*" => vec!["global", "startupWarnings"],
            _ => vec![],
        };

        doc! {
            "totalLinesWritten": log_lines.len() as i64,
            "log": log_lines,
            "ok": 1
        }
    }
}

impl MongoDBCommandHandler for DefaultMongoDBHandler {
    fn handle_message(&self, message: &Message) -> Message {
        let response_payload = match &message.payload {
            MessagePayload::OpMsg(op_msg) => {
                // Extract command from OP_MSG
                if let Some(Section::Body(command)) = op_msg.sections.first() {
                    // Get database name from $db field
                    let db_name = command
                        .get_str("$db")
                        .unwrap_or("test");

                    let response_doc = self.handle_command(db_name, command);
                    MessagePayload::OpMsg(OpMsg::new_body(response_doc))
                } else {
                    MessagePayload::OpMsg(OpMsg::new_body(doc! { "ok": 0, "errmsg": "Invalid message" }))
                }
            }
            MessagePayload::OpQuery(query) => {
                // Legacy OP_QUERY - extract command from query document
                let db_and_collection = &query.full_collection_name;
                let parts: Vec<&str> = db_and_collection.splitn(2, '.').collect();
                let db_name = parts.first().copied().unwrap_or("test");

                // Check if it's a command (collection is $cmd)
                if parts.get(1) == Some(&"$cmd") {
                    let response_doc = self.handle_command(db_name, &query.query);
                    MessagePayload::OpReply(OpReply::new_ok(0, vec![response_doc]))
                } else {
                    // Regular query
                    let collection_name = parts.get(1).copied().unwrap_or("");
                    let db = self.storage.get_database(db_name);
                    let collection = db.get_collection(collection_name);
                    let filter = query.query.get_document("query").ok().or_else(|| query.query.get_document("$query").ok());
                    let documents = collection.find(filter);
                    MessagePayload::OpReply(OpReply::new_ok(0, documents))
                }
            }
            _ => {
                MessagePayload::OpMsg(OpMsg::new_body(doc! { "ok": 0, "errmsg": "Unsupported operation" }))
            }
        };

        Message::encode_reply(
            message.header.request_id.wrapping_add(1),
            message.header.request_id,
            response_payload,
        )
    }
}
