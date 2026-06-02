# MaxCompute Tunnel Protocol - Complete Research Report

## Table of Contents
1. [Overview](#overview)
2. [Tunnel Endpoint Discovery](#tunnel-endpoint-discovery)
3. [Authentication](#authentication)
4. [REST API Endpoints](#rest-api-endpoints)
5. [Upload Workflow](#upload-workflow)
6. [Download Workflow](#download-workflow)
7. [Data Format - Protobuf Record Encoding](#data-format---protobuf-record-encoding)
8. [Data Format - Arrow Encoding](#data-format---arrow-encoding)
9. [Compression](#compression)
10. [Checksums](#checksums)
11. [Error Response Format](#error-response-format)
12. [HTTP Headers Reference](#http-headers-reference)
13. [Query Parameters Reference](#query-parameters-reference)
14. [JSON Response Schemas](#json-response-schemas)
15. [Implementation Strategy for RorisDB](#implementation-strategy-for-rorisdb)

---

## Overview

The MaxCompute Tunnel is a RESTful HTTP-based data transfer service for bulk upload/download of data to/from MaxCompute tables. It uses a **session-based** workflow:

1. Client creates a session (upload or download)
2. Client transfers data in blocks (upload) or row ranges (download)
3. Client commits the session (upload only)

**Key protocol facts:**
- Protocol version: `6` (constant `TUNNEL_VERSION`)
- Data transform version: `v1`
- Base URL pattern: `{tunnel_endpoint}/projects/{project}/tables/{table}`
- With schema: `{tunnel_endpoint}/projects/{project}/schemas/{schema}/tables/{table}`
- Content format: Custom protobuf-like binary encoding (NOT standard protobuf, but wire-compatible)
- Arrow format: Apache Arrow IPC RecordBatch format (for Arrow tunnels)
- Session responses: JSON format
- Error responses: JSON format (with XML fallback for older APIs)

---

## Tunnel Endpoint Discovery

Before making tunnel API calls, the client discovers the tunnel endpoint:

```
GET {odps_endpoint}/projects/{project}/tunnel
```

Response: plain text body containing the tunnel server address (e.g., `dt.cn-shanghai.maxcompute.aliyun.com`)

The client prepends the protocol scheme from the ODPS endpoint to form the full tunnel URL.

**For RorisDB mock**: You can skip this by having the client use a direct tunnel endpoint URL, or implement this simple endpoint on your ODPS-compatible service.

---

## Authentication

MaxCompute Tunnel uses the **same authentication** as the ODPS REST API. The signature is placed in the `Authorization` header.

### V2 Signature (Legacy)

```
Authorization: ODPS {AccessKeyId}:{Signature}
```

Where `Signature = Base64(HMAC-SHA1(SecretAccessKey, CanonicalString))`

**CanonicalString** is built as:
```
{HTTP_METHOD}\n
{Content-MD5}\n
{Content-Type}\n
{Date}\n
{x-odps-header1:value1}
{x-odps-header2:value2}
...
{CanonicalResource}
```

- All `x-odps-*` headers are sorted alphabetically, formatted as `key:value`
- `Content-MD5` and `Content-Type` default to empty string if not present
- `Date` is RFC 1123 format
- `CanonicalResource` = URL path + sorted query string (params with empty values get just the key)

### V4 Signature (Current)

```
Authorization: ODPS {AccessKeyId}/{Date}/{Region}/odps/aliyun_v4_request:{Signature}
```

- Uses HMAC-SHA256 with key derivation (similar to AWS SigV4)
- Key derivation: `kSecret → kDate → kRegion → kService → kSigning`
- The canonical string is the same format as V2 but signed with the derived key

**For RorisDB mock**: You can implement a simple signature verification or skip authentication entirely for testing. The signature uses standard HMAC-SHA1 (V2) or HMAC-SHA256 (V4).

---

## REST API Endpoints

### Base Resource URL
```
/projects/{project}/tables/{table}
```
With schema:
```
/projects/{project}/schemas/{schema}/tables/{table}
```

### Upload Session APIs

| Operation | Method | Query Params | Body | Description |
|-----------|--------|-------------|------|-------------|
| Create Upload Session | POST | `uploads` | empty | Create new upload session |
| Reload Upload Session | GET | `uploadid={id}` | empty | Get session status and block list |
| Upload Block | PUT | `uploadid={id}&blockid={n}` | binary data | Upload a block of data |
| Commit Upload | POST | `uploadid={id}` | empty | Finalize the upload session |

### Download Session APIs

| Operation | Method | Query Params | Body | Description |
|-----------|--------|-------------|------|-------------|
| Create Download Session | POST | `downloads` | empty | Create new download session |
| Reload Download Session | GET | `downloadid={id}` | empty | Get session status and record count |
| Download Data | GET | `downloadid={id}&rowrange=(start,count)` | empty | Download row range (action: `data`) |

### Stream Upload APIs

| Operation | Method | Query Params | Body | Description |
|-----------|--------|-------------|------|-------------|
| Create Stream Session | POST | (to `/streams` suffix) | empty | Create stream upload session |
| Upload Stream Block | PUT | `uploadid={id}&slotid={n}` | binary data | Upload data to stream |
| Abort Stream | POST | `uploadid={id}` (to `/streams`) | empty | Abort stream session |

### Upsert APIs

| Operation | Method | Query Params | Body | Description |
|-----------|--------|-------------|------|-------------|
| Create Upsert Session | POST | `slotnum={n}` (to `/upserts`) | empty | Create upsert session |
| Upsert Data | PUT | `upsertid={id}&bucketid={b}&slotid={s}&record_count={n}` | binary data | Write upsert records |
| Commit Upsert | POST | `upsertid={id}` (to `/upserts`) | empty | Commit upsert session |
| Abort Upsert | DELETE | `upsertid={id}` (to `/upserts`) | empty | Abort upsert session |

### Preview API

| Operation | Method | Query Params | Body | Description |
|-----------|--------|-------------|------|-------------|
| Preview Data | GET | `limit={n}` (to `/preview` suffix) | empty | Read initial rows |

---

## Upload Workflow

### Step-by-Step

```
1. POST /projects/{proj}/tables/{table}?uploads
   → Response JSON: { "UploadID": "xxx", "Status": "NORMAL", "Schema": {...}, ... }

2. PUT /projects/{proj}/tables/{table}?uploadid={id}&blockid=0
   Content-Type: application/octet-stream
   Transfer-Encoding: chunked
   Content-Encoding: deflate (optional, for compression)
   Body: [protobuf-encoded binary data]
   → Response: 200 OK

3. PUT /projects/{proj}/tables/{table}?uploadid={id}&blockid=1
   → Response: 200 OK

4. POST /projects/{proj}/tables/{table}?uploadid={id}
   → Response JSON: { "Status": "NORMAL", "UploadedBlockList": [...], ... }
   (This commits the session)
```

### Upload Data Format (Protobuf Record Stream)

Each block's body is a stream of protobuf-encoded records:

**Per Record:**
```
For each non-null column (1-indexed field_number = column_index + 1):
  Tag: varint (field_number << 3 | wire_type)
  Value: depends on type (see wire types below)

End-of-record marker:
  Tag: varint (33553408 << 3 | 0)  // TUNNEL_END_RECORD = 33553408, WIRETYPE_VARINT = 0
  Value: uint32 checksum (CRC32C of this record's data)
```

**End-of-stream markers (written when block is closed):**
```
Tag: varint (33554430 << 3 | 0)  // TUNNEL_META_COUNT = 33554430
Value: sint64 record_count

Tag: varint (33554431 << 3 | 0)  // TUNNEL_META_CHECKSUM = 33554431
Value: uint32 overall_crc32c (CRC32C of all per-record checksums)
```

### Wire Types by ODPS Data Type

| ODPS Type | Wire Type | Encoding |
|-----------|-----------|----------|
| BIGINT, INT, SMALLINT, TINYINT | VARINT (0) | zigzag + varint (sint64) |
| BOOLEAN | VARINT (0) | varint (0 or 1) |
| DATETIME | VARINT (0) | zigzag + varint (milliseconds since epoch) |
| DATE | VARINT (0) | zigzag + varint (days since epoch) |
| INTERVAL_YEAR_MONTH | VARINT (0) | zigzag + varint (total months) |
| FLOAT | FIXED32 (5) | 4 bytes little-endian IEEE 754 |
| DOUBLE | FIXED64 (1) | 8 bytes little-endian IEEE 754 |
| STRING, VARCHAR, CHAR | LENGTH_DELIMITED (2) | varint length + UTF-8 bytes |
| BINARY | LENGTH_DELIMITED (2) | varint length + raw bytes |
| TIMESTAMP | LENGTH_DELIMITED (2) | sint64 seconds + sint32 nanoseconds |
| TIMESTAMP_NTZ | LENGTH_DELIMITED (2) | sint64 seconds + sint32 nanoseconds (UTC) |
| DECIMAL | LENGTH_DELIMITED (2) | varint length + string representation |
| JSON | LENGTH_DELIMITED (2) | varint length + JSON string bytes |
| ARRAY | LENGTH_DELIMITED (2) | uint32 count + [bool is_null + value...] |
| MAP | LENGTH_DELIMITED (2) | uint32 key_count + keys + uint32 val_count + vals |
| STRUCT | LENGTH_DELIMITED (2) | [bool is_null + value...] for each field |
| INTERVAL_DAY_TIME | LENGTH_DELIMITED (2) | sint64 seconds + sint32 nanoseconds |
| VECTOR | LENGTH_DELIMITED (2) | uint32 dimension + values |

### CRC32C Checksum Details

For each record:
1. Before each column's value is encoded, update CRC32C with `(column_index as int32 little-endian)`
2. Update CRC32C with the value's binary representation:
   - bool: `\x01` for true, `\x00` for false
   - int64/long: 8 bytes little-endian
   - int32: 4 bytes little-endian
   - float: 4 bytes little-endian IEEE 754
   - double: 8 bytes little-endian IEEE 754
   - string/binary: raw bytes
3. After all columns, compute `checksum = CRC32C.getvalue()` → convert to int
4. Write TUNNEL_END_RECORD tag + `uint32(checksum)` (unsigned interpretation)
5. Feed this checksum int into the overall CRC32C accumulator (`crccrc`)

At stream end:
- Write TUNNEL_META_COUNT + record_count (sint64)
- Write TUNNEL_META_CHECKSUM + `uint32(crccrc.getvalue())`

---

## Data Format - Arrow Encoding

For Arrow tunnels (used when `arrow` param is present), the data format is:

**Upload (write):**
```
[4 bytes: chunk_size (big-endian uint32)]
[chunk_size bytes of Arrow IPC RecordBatch data]
[4 bytes: CRC32C checksum of the chunk data]
[... more chunks ...]
[4 bytes: overall CRC32C of all chunk data]  // final checksum
```

**Download (read):**
Same format as upload. The reader reads:
1. 4-byte chunk_size (big-endian)
2. chunk_size + 4 bytes of data (data + checksum)
3. Verify CRC32C
4. Repeat until last chunk (shorter than chunk_size + 4)
5. Final 4 bytes = overall CRC32C

The Arrow IPC data is standard `pa.RecordBatch.serialize()` output.

---

## Compression

Supported compression algorithms (Content-Encoding header values):

| Algorithm | Content-Encoding (legacy) | Content-Encoding (new) |
|-----------|--------------------------|----------------------|
| RAW (none) | (none) | (none) |
| ZLIB | `deflate` | (not supported in new) |
| SNAPPY | `x-snappy-framed` | (not supported in new) |
| ZSTD | `zstd` | `ZSTD` |
| LZ4 | `x-lz4-frame` | `LZ4_FRAME` |
| ARROW_LZ4 | `x-odps-lz4-frame` | `LZ4_FRAME` |

- Upload: Client sets `Content-Encoding` header
- Download: Client sets `Accept-Encoding` header; server responds with `Content-Encoding`
- The compressed data wraps the protobuf/arrow binary stream

---

## Checksums

**Algorithm**: CRC32C (Castagnoli) is the default. CRC32 is also supported.

**Checksum update operations:**
- `update_bool(val)`: CRC of `\x01` or `\x00`
- `update_int(val)`: CRC of `struct.pack("<i", val)` (4 bytes LE)
- `update_long(val)`: CRC of `struct.pack("<q", val)` (8 bytes LE)
- `update_float(val)`: CRC of `struct.pack("<f", val)` (4 bytes LE)
- `update_double(val)`: CRC of `struct.pack("<d", val)` (8 bytes LE)
- `update(bytes)`: CRC of raw bytes

---

## Error Response Format

### JSON Error Response (primary)
```json
{
  "Code": "ErrorCodeString",
  "Message": "Human-readable error message",
  "RequestId": "request-id-string"
}
```

Additionally may include:
```json
{
  "HoldClientMillis": "5000"
}
```

### XML Error Response (legacy fallback)
```xml
<Error>
  <Code>ErrorCodeString</Code>
  <Message>Human-readable error message</Message>
  <RequestId>request-id-string</RequestId>
</Error>
```

### Common Tunnel Error Codes
- `TunnelError` - Generic tunnel error
- `InvalidArgument` - Invalid parameter
- `ObjectNotFound` - Session/table not found
- `InvalidParameter` - Invalid parameter value
- `MetaTransactionFailed` - Internal transaction failure
- `StreamSessionNotFound` - Stream session not found
- `UpsertSessionNotFound` - Upsert session not found

HTTP status codes:
- 200: Success
- 400: Bad request / invalid parameter
- 401: Authentication required
- 403: Forbidden
- 404: Not found
- 500: Internal server error

---

## HTTP Headers Reference

### Required Headers (all requests)
```
Content-Length: 0                      # (for non-data requests)
odps-tunnel-date-transform: v1         # Data transform version
x-odps-tunnel-version: 6              # Tunnel protocol version
odps-tunnel-sdk-support-schema-evolution: true
```

### Upload Block Headers
```
Transfer-Encoding: chunked
Content-Type: application/octet-stream
Content-Encoding: deflate              # (optional, if compression enabled)
```

### Download Data Headers
```
Accept-Encoding: deflate               # (optional, request compression)
```

### Response Headers
```
x-odps-request-id: {uuid}             # Request ID for debugging
odps-tunnel-metrics: {json}           # Performance metrics (optional)
Content-Encoding: deflate              # (if response is compressed)
```

### Stream/Upsert Headers
```
odps-tunnel-routed-server: {ip}:{port}  # Server routing info
odps-tunnel-slot-num: {n}              # Number of slots
odps-slot-num: {n}                     # Requested slot count
```

### Tags Header (optional)
```
odps-tunnel-tags: tag1,tag2
```

---

## Query Parameters Reference

### Upload Session
| Param | Value | Description |
|-------|-------|-------------|
| `uploads` | (empty/null) | Create upload session (POST) |
| `uploadid` | session_id | Upload session ID |
| `blockid` | integer | Block ID (for PUT) |
| `overwrite` | `true` | Overwrite table data |
| `create_partition` | `true` | Auto-create partition |
| `arrow` | (empty) | Use Arrow format |

### Download Session
| Param | Value | Description |
|-------|-------|-------------|
| `downloads` | (empty/null) | Create download session (POST) |
| `downloadid` | session_id | Download session ID |
| `rowrange` | `(start,count)` | Row range to download |
| `columns` | `col1,col2` | Columns to download |
| `asyncmode` | `true` | Async session creation |
| `arrow` | (empty) | Use Arrow format |
| `raw_size` | integer | Raw size limit |

### Common
| Param | Value | Description |
|-------|-------|-------------|
| `partition` | spec | Partition specification |
| `quotaName` | name | Quota name |
| `curr_project` | name | Current project name |

### Stream Upload
| Param | Value | Description |
|-------|-------|-------------|
| `slotid` | integer | Slot ID |
| `schema_version` | version | Schema version |
| `zorder_columns` | cols | Z-order columns |
| `dynamic_partition` | `true` | Dynamic partition |
| `check_latest_schema` | bool | Check latest schema |

### Upsert
| Param | Value | Description |
|-------|-------|-------------|
| `slotnum` | integer | Number of slots |
| `upsertid` | session_id | Upsert session ID |
| `bucketid` | integer | Bucket ID |
| `record_count` | integer | Record count |
| `lifecycle` | hours | Session lifecycle (1-24) |

---

## JSON Response Schemas

### Create Upload Session Response
```json
{
  "UploadID": "string",
  "Status": "NORMAL",
  "Schema": {
    "columns": [
      { "name": "col1", "type": "string", "nullable": true },
      { "name": "col2", "type": "bigint", "nullable": true }
    ],
    "partitionKeys": [
      { "name": "pt", "type": "string" }
    ]
  },
  "MaxFieldSize": 0,
  "QuotaName": "string"
}
```

### Reload Upload Session Response (same as create + block list)
```json
{
  "UploadID": "string",
  "Status": "NORMAL",
  "UploadedBlockList": [
    { "BlockID": 0 },
    { "BlockID": 1 }
  ],
  "Schema": { ... },
  "QuotaName": "string"
}
```

### Create Download Session Response
```json
{
  "DownloadID": "string",
  "Status": "NORMAL",
  "RecordCount": 1000,
  "Schema": { ... },
  "QuotaName": "string",
  "SupportReadByRawSize": false
}
```

### Stream Upload Session Response
```json
{
  "session_name": "string",
  "status": "NORMAL",
  "schema": { ... },
  "slots": [
    [0, "10.0.0.1:8080"],
    [1, "10.0.0.2:8080"]
  ],
  "schema_version": "1",
  "QuotaName": "string"
}
```

### Upsert Session Response
```json
{
  "id": "string",
  "status": "NORMAL",
  "schema": { ... },
  "slots": [
    {
      "slot_id": 0,
      "worker_addr": "10.0.0.1:8080",
      "buckets": [0, 1, 2]
    }
  ],
  "hash_key": ["key_col"],
  "hasher": "default",
  "quota_name": "string",
  "enable_partial_update": false
}
```

---

## Implementation Strategy for RorisDB

### Minimal Viable Implementation

To be compatible with pyodps and Java SDK, implement these endpoints:

#### 1. Tunnel Endpoint Discovery (on ODPS endpoint)
```
GET /projects/{project}/tunnel
→ 200, body: "127.0.0.1:{port}"  (or your tunnel server address)
```

#### 2. Create Upload Session
```
POST /projects/{project}/tables/{table}?uploads
→ 200, JSON: { "UploadID": "uuid", "Status": "NORMAL", "Schema": {...} }
```

#### 3. Upload Block
```
PUT /projects/{project}/tables/{table}?uploadid={id}&blockid={n}
Body: protobuf binary (or compressed)
→ 200 OK
```

Parse the protobuf stream:
- Read tag (field_number, wire_type) pairs
- Each field_number - 1 = column index
- TUNNEL_END_RECORD (33553408) = end of record
- TUNNEL_META_COUNT (33554430) = record count
- TUNNEL_META_CHECKSUM (33554431) = overall checksum

#### 4. Commit Upload
```
POST /projects/{project}/tables/{table}?uploadid={id}
→ 200, JSON: { "Status": "NORMAL", "UploadedBlockList": [...] }
```
→ Actually ingest the uploaded data into your Parquet storage.

#### 5. Create Download Session
```
POST /projects/{project}/tables/{table}?downloads
→ 200, JSON: { "DownloadID": "uuid", "Status": "NORMAL", "RecordCount": N, "Schema": {...} }
```

#### 6. Download Data
```
GET /projects/{project}/tables/{table}?downloadid={id}&rowrange=(start,count)&data
→ 200, body: protobuf binary stream (or Arrow IPC)
```
Encode the data using the same protobuf format described above.

#### 7. Reload Session (GET with session ID)
```
GET /projects/{project}/tables/{table}?uploadid={id}
GET /projects/{project}/tables/{table}?downloadid={id}
→ 200, JSON: same as create response with current status
```

### Authentication

For a mock/compatibility server, you have options:
1. **Accept any `Authorization` header** (simplest for testing)
2. **Validate V2 signature** (HMAC-SHA1, straightforward to implement)
3. **Skip auth entirely** (if pyodps can be configured without credentials)

### Data Handling

For **upload**: 
- Receive the protobuf binary stream per block
- Decode records using the protobuf wire format
- Store records temporarily
- On commit, write to Parquet files

For **download**:
- Read from Parquet files
- Encode records into protobuf binary stream
- Return as HTTP response body

### Key Wire Format Constants
```
TUNNEL_END_RECORD   = 33553408  (0x01FFFFE0)
TUNNEL_META_COUNT   = 33554430  (0x01FFFFFF)
TUNNEL_META_CHECKSUM = 33554431 (0x02000000)
TUNNEL_END_METRICS  = 33554176  (0x01FFFF80)
```

### Tag Encoding
```
tag = (field_number << 3) | wire_type
```
Wire types:
- 0 = VARINT
- 1 = FIXED64
- 2 = LENGTH_DELIMITED
- 5 = FIXED32

### Varint Encoding
Standard protobuf varint: 7 bits per byte, MSB = continuation bit.

### ZigZag Encoding (for signed integers)
```
encode: (n << 1) ^ (n >> 31)  # for 32-bit
encode: (n << 1) ^ (n >> 63)  # for 64-bit
decode: (n >> 1) ^ -(n & 1)
```

---

## Sources

All information derived from reading the source code of:
- **pyodps** (Python SDK): `github.com/aliyun/aliyun-odps-python-sdk`
  - `odps/tunnel/base.py` - Tunnel base class, endpoint discovery
  - `odps/tunnel/tabletunnel.py` - All session types and API calls
  - `odps/tunnel/io/writer.py` - Record/Arrow writer (protobuf encoding)
  - `odps/tunnel/io/reader.py` - Record/Arrow reader (protobuf decoding)
  - `odps/tunnel/io/stream.py` - Compression options and streams
  - `odps/tunnel/pb/encoder.py` - Protobuf encoder
  - `odps/tunnel/pb/decoder.py` - Protobuf decoder
  - `odps/tunnel/pb/wire_format.py` - Wire format constants and helpers
  - `odps/tunnel/pb/output_stream.py` - Varint/LittleEndian encoding
  - `odps/tunnel/wireconstants.py` - TUNNEL_END_RECORD etc.
  - `odps/tunnel/checksum.py` - CRC32C checksum implementation
  - `odps/tunnel/errors.py` - Error parsing (XML and JSON)
  - `odps/rest.py` - REST client, request building
  - `odps/accounts.py` - Authentication (V2/V4 signatures)

- **aliyun-odps-java-sdk** (Java SDK): `github.com/aliyun/aliyun-odps-java-sdk`
  - `TableTunnel.java` - All session types, inner UploadSession/DownloadSession classes
  - `TunnelConstants.java` - All parameter names and constants
  - `HttpHeaders.java` - All HTTP header names
  - `TunnelException.java` - Error response parsing (JSON)
  - `SessionBase.java` - Base session HTTP request logic
  - `Configuration.java` - Tunnel configuration
  - `GeneralConfiguration.java` - URL building
  - `ResourceBuilder.java` - REST resource URL patterns
  - `Util.java` - Common headers
