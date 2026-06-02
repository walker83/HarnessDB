//! MaxCompute Tunnel Protocol — bulk data transfer.
//!
//! Implements the Tunnel session-based upload/download protocol compatible with
//! pyodps and aliyun-odps-java-sdk clients.
//!
//! # Endpoints
//! - `GET  /api/projects/{project}/tunnel` — Tunnel endpoint discovery
//! - `POST /api/projects/{project}/tables/{table}?uploads` — Create upload session
//! - `PUT  /api/projects/{project}/tables/{table}?uploadid={id}&blockid={n}` — Upload block
//! - `POST /api/projects/{project}/tables/{table}?uploadid={id}` — Commit upload
//! - `POST /api/projects/{project}/tables/{table}?downloads` — Create download session
//! - `GET  /api/projects/{project}/tables/{table}?downloadid={id}&rowrange=(start,count)` — Download data
//! - `GET  /api/projects/{project}/tables/{table}?{uploadid|downloadid}={id}` — Reload session
//!
//! # Data Format
//! Records are encoded in a protobuf-like binary wire format:
//! - Tags: varint `(field_number << 3) | wire_type`
//! - Wire types: VARINT(0), FIXED64(1), LENGTH_DELIMITED(2), FIXED32(5)
//! - Per-record CRC32C checksum, terminated by `TUNNEL_END_RECORD`
//! - Stream footer: `TUNNEL_META_COUNT` + `TUNNEL_META_CHECKSUM`
//!
//! # Compression
//! ZLIB/deflate is supported via `Content-Encoding: deflate` (upload) and
//! `Accept-Encoding: deflate` (download).

pub mod handlers;
pub mod io;
pub mod schema;
pub mod compression;
pub mod json;
pub mod session;

/// Tunnel protocol version constant.
pub const TUNNEL_VERSION: u32 = 6;
