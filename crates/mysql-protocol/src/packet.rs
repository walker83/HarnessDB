use bytes::{Buf, BufMut, BytesMut};
use std::io::Cursor;
use types::ScalarValue;

use crate::charset::{self, DEFAULT_CHARSET};
use crate::value::{encode_lenenc_int, encode_lenenc_str};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum size of a single MySQL packet payload.
pub const MAX_PACKET_SIZE: usize = 0x00FF_FFFF;

/// Default server capability flags.
const DEFAULT_CAPABILITIES: u32 =
    CapabilityFlags::PROTOCOL_41
    | CapabilityFlags::PLUGIN_AUTH
    | CapabilityFlags::SECURE_CONNECTION
    | CapabilityFlags::CONNECT_WITH_DB
    | CapabilityFlags::LONG_FLAG
    | CapabilityFlags::TRANSACTIONS
    | CapabilityFlags::MULTI_STATEMENTS
    | CapabilityFlags::MULTI_RESULTS
    | CapabilityFlags::PS_MULTI_RESULTS
    | CapabilityFlags::PLUGIN_AUTH_LENENC_CLIENT_DATA
    | CapabilityFlags::DEPRECATE_EOF;

/// Server status flags sent in OK packets.
pub const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;

/// MySQL native password auth plugin name.
pub const AUTH_PLUGIN_NAME: &[u8] = b"mysql_native_password";

/// Column type constants (MYSQL_TYPE_*).
pub mod column_type {
    pub const DECIMAL: u8 = 0x00;
    pub const TINY: u8 = 0x01;
    pub const SHORT: u8 = 0x02;
    pub const LONG: u8 = 0x03;
    pub const FLOAT: u8 = 0x04;
    pub const DOUBLE: u8 = 0x05;
    pub const NULL: u8 = 0x06;
    pub const TIMESTAMP: u8 = 0x07;
    pub const LONGLONG: u8 = 0x08;
    pub const INT24: u8 = 0x09;
    pub const DATE: u8 = 0x0A;
    pub const TIME: u8 = 0x0B;
    pub const DATETIME: u8 = 0x0C;
    pub const YEAR: u8 = 0x0D;
    pub const VARCHAR: u8 = 0x0F;
    pub const BIT: u8 = 0x10;
    pub const NEWDECIMAL: u8 = 0xF6;
    pub const ENUM: u8 = 0xF7;
    pub const SET: u8 = 0xF8;
    pub const TINY_BLOB: u8 = 0xF9;
    pub const MEDIUM_BLOB: u8 = 0xFA;
    pub const LONG_BLOB: u8 = 0xFB;
    pub const BLOB: u8 = 0xFC;
    pub const VAR_STRING: u8 = 0xFD;
    pub const STRING: u8 = 0xFE;
    pub const GEOMETRY: u8 = 0xFF;
}

// ---------------------------------------------------------------------------
// Capability flags
// ---------------------------------------------------------------------------

pub mod CapabilityFlags {
    pub const LONG_PASSWORD: u32 = 0x00000001;
    pub const FOUND_ROWS: u32 = 0x00000002;
    pub const LONG_FLAG: u32 = 0x00000004;
    pub const CONNECT_WITH_DB: u32 = 0x00000008;
    pub const NO_SCHEMA: u32 = 0x00000010;
    pub const COMPRESS: u32 = 0x00000020;
    pub const ODBC: u32 = 0x00000040;
    pub const LOCAL_FILES: u32 = 0x00000080;
    pub const IGNORE_SPACE: u32 = 0x00000100;
    pub const PROTOCOL_41: u32 = 0x00000200;
    pub const INTERACTIVE: u32 = 0x00000400;
    pub const SSL: u32 = 0x00000800;
    pub const IGNORE_SIGPIPE: u32 = 0x00001000;
    pub const TRANSACTIONS: u32 = 0x00002000;
    pub const RESERVED: u32 = 0x00004000;
    pub const SECURE_CONNECTION: u32 = 0x00008000;
    pub const MULTI_STATEMENTS: u32 = 0x00010000;
    pub const MULTI_RESULTS: u32 = 0x00020000;
    pub const PS_MULTI_RESULTS: u32 = 0x00040000;
    pub const PLUGIN_AUTH: u32 = 0x00080000;
    pub const CONNECT_ATTRS: u32 = 0x00100000;
    pub const PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x00200000;
    pub const CAN_HANDLE_EXPIRED_PASSWORDS: u32 = 0x00400000;
    pub const SESSION_TRACK: u32 = 0x00800000;
    pub const DEPRECATE_EOF: u32 = 0x01000000;
}

// ---------------------------------------------------------------------------
// Command types
// ---------------------------------------------------------------------------

pub mod command {
    pub const COM_SLEEP: u8 = 0x00;
    pub const COM_QUIT: u8 = 0x01;
    pub const COM_INIT_DB: u8 = 0x02;
    pub const COM_QUERY: u8 = 0x03;
    pub const COM_FIELD_LIST: u8 = 0x04;
    pub const COM_CREATE_DB: u8 = 0x05;
    pub const COM_DROP_DB: u8 = 0x06;
    pub const COM_REFRESH: u8 = 0x07;
    pub const COM_SHUTDOWN: u8 = 0x08;
    pub const COM_STATISTICS: u8 = 0x09;
    pub const COM_PROCESS_INFO: u8 = 0x0A;
    pub const COM_CONNECT: u8 = 0x0B;
    pub const COM_PROCESS_KILL: u8 = 0x0C;
    pub const COM_DEBUG: u8 = 0x0D;
    pub const COM_PING: u8 = 0x0E;
    pub const COM_TIME: u8 = 0x0F;
    pub const COM_DELAYED_INSERT: u8 = 0x10;
    pub const COM_CHANGE_USER: u8 = 0x11;
    pub const COM_BINLOG_DUMP: u8 = 0x12;
    pub const COM_TABLE_DUMP: u8 = 0x13;
    pub const COM_CONNECT_OUT: u8 = 0x14;
    pub const COM_REGISTER_SLAVE: u8 = 0x15;
    pub const COM_STMT_PREPARE: u8 = 0x16;
    pub const COM_STMT_EXECUTE: u8 = 0x17;
    pub const COM_STMT_SEND_LONG_DATA: u8 = 0x18;
    pub const COM_STMT_CLOSE: u8 = 0x19;
    pub const COM_STMT_RESET: u8 = 0x1A;
    pub const COM_SET_OPTION: u8 = 0x1B;
    pub const COM_STMT_FETCH: u8 = 0x1C;
    pub const COM_DAEMON: u8 = 0x1D;
    pub const COM_BINLOG_DUMP_GTID: u8 = 0x1E;
    pub const COM_RESET_CONNECTION: u8 = 0x1F;
}

// ---------------------------------------------------------------------------
// Packet header helpers
// ---------------------------------------------------------------------------

/// Write a MySQL packet header: 3-byte length + 1-byte sequence id.
pub fn write_packet_header(buf: &mut BytesMut, length: usize, seq_id: u8) {
    buf.put_u8((length & 0xFF) as u8);
    buf.put_u8(((length >> 8) & 0xFF) as u8);
    buf.put_u8(((length >> 16) & 0xFF) as u8);
    buf.put_u8(seq_id);
}

/// Read a MySQL packet header. Returns (payload_length, sequence_id).
pub fn read_packet_header(buf: &[u8]) -> Option<(usize, u8)> {
    if buf.len() < 4 {
        return None;
    }
    let length = (buf[0] as usize) | ((buf[1] as usize) << 8) | ((buf[2] as usize) << 16);
    let seq_id = buf[3];
    Some((length, seq_id))
}

// ---------------------------------------------------------------------------
// Packet builder (accumulates payload, wraps with header on finish)
// ---------------------------------------------------------------------------

pub struct PacketBuilder {
    payload: BytesMut,
    seq_id: u8,
}

impl PacketBuilder {
    pub fn new(seq_id: u8) -> Self {
        Self {
            payload: BytesMut::with_capacity(256),
            seq_id,
        }
    }

    pub fn put_u8(&mut self, v: u8) -> &mut Self {
        self.payload.put_u8(v);
        self
    }

    pub fn put_u16_le(&mut self, v: u16) -> &mut Self {
        self.payload.put_u16_le(v);
        self
    }

    pub fn put_u32_le(&mut self, v: u32) -> &mut Self {
        self.payload.put_u32_le(v);
        self
    }

    pub fn put_u64_le(&mut self, v: u64) -> &mut Self {
        self.payload.put_u64_le(v);
        self
    }

    pub fn put_slice(&mut self, s: &[u8]) -> &mut Self {
        self.payload.put_slice(s);
        self
    }

    pub fn lenenc_int(&mut self, n: u64) -> &mut Self {
        encode_lenenc_int(&mut self.payload, n);
        self
    }

    pub fn lenenc_str(&mut self, s: &str) -> &mut Self {
        encode_lenenc_str(&mut self.payload, s);
        self
    }

    pub fn lenenc_bytes(&mut self, b: &[u8]) -> &mut Self {
        encode_lenenc_int(&mut self.payload, b.len() as u64);
        self.payload.put_slice(b);
        self
    }

    /// Build the final packet with header prepended.
    /// Returns the encoded packet and the next sequence id.
    pub fn finish(self) -> (BytesMut, u8) {
        let len = self.payload.len();
        let mut buf = BytesMut::with_capacity(4 + len);
        write_packet_header(&mut buf, len, self.seq_id);
        buf.put(self.payload);
        (buf, self.seq_id.wrapping_add(1))
    }
}

// ---------------------------------------------------------------------------
// Server handshake v10 packet
// ---------------------------------------------------------------------------

pub struct HandshakeV10 {
    pub server_version: String,
    pub connection_id: u32,
    pub auth_salt: [u8; 20],
    pub capability_flags: u32,
    pub charset: u8,
    pub status_flags: u16,
    pub auth_plugin_name: &'static [u8],
}

impl HandshakeV10 {
    pub fn new(connection_id: u32, auth_salt: [u8; 20]) -> Self {
        Self {
            server_version: "5.7.44-RovisDB".to_string(),
            connection_id,
            auth_salt,
            capability_flags: DEFAULT_CAPABILITIES,
            charset: DEFAULT_CHARSET,
            status_flags: SERVER_STATUS_AUTOCOMMIT,
            auth_plugin_name: AUTH_PLUGIN_NAME,
        }
    }

    /// Encode into a single packet with the given sequence id.
    pub fn encode(&self, seq_id: u8) -> BytesMut {
        let mut pb = PacketBuilder::new(seq_id);

        // Protocol version
        pb.put_u8(10); // protocol v10

        // Server version (null-terminated)
        pb.put_slice(self.server_version.as_bytes());
        pb.put_u8(0);

        // Connection ID (thread id)
        pb.put_u32_le(self.connection_id);

        // Auth salt part 1 (first 8 bytes)
        pb.put_slice(&self.auth_salt[..8]);

        // Filler
        pb.put_u8(0);

        // Capability flags lower 16 bits
        pb.put_u16_le((self.capability_flags & 0xFFFF) as u16);

        // Character set
        pb.put_u8(self.charset);

        // Status flags
        pb.put_u16_le(self.status_flags);

        // Capability flags upper 16 bits
        pb.put_u16_le(((self.capability_flags >> 16) & 0xFFFF) as u16);

        // Length of auth plugin data (always 21 for mysql_native_password: 20 bytes + null)
        if (self.capability_flags & CapabilityFlags::SECURE_CONNECTION) != 0 {
            pb.put_u8(21);
        } else {
            pb.put_u8(0);
        }

        // Reserved 10 bytes of zeroes
        pb.put_slice(&[0u8; 10]);

        // Auth salt part 2 (remaining 12 bytes)
        if (self.capability_flags & CapabilityFlags::SECURE_CONNECTION) != 0 {
            pb.put_slice(&self.auth_salt[8..]);
            pb.put_u8(0);
        }

        // Auth plugin name (null-terminated)
        if (self.capability_flags & CapabilityFlags::PLUGIN_AUTH) != 0 {
            pb.put_slice(self.auth_plugin_name);
            pb.put_u8(0);
        }

        let (packet, _) = pb.finish();
        packet
    }
}

// ---------------------------------------------------------------------------
// Handshake response (parsed from client)
// ---------------------------------------------------------------------------

pub struct HandshakeResponse {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub charset: u8,
    pub username: String,
    pub auth_response: Vec<u8>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}

impl HandshakeResponse {
    /// Parse a handshake response from raw packet payload (after header).
    pub fn parse(payload: &[u8]) -> Result<Self, String> {
        let mut cur = Cursor::new(payload);

        let capability_flags_lower = cur.get_u16_le() as u32;
        let max_packet_size = cur.get_u32_le();
        let charset = cur.get_u8();

        // Skip 23 bytes of reserved data
        let reserved_start = cur.position() as usize;
        if payload.len() < reserved_start + 23 {
            return Err("handshake response too short".to_string());
        }
        cur.set_position((reserved_start + 23) as u64);

        // Username (null-terminated)
        let username_start = cur.position() as usize;
        let username_end = payload[username_start..]
            .iter()
            .position(|&b| b == 0)
            .ok_or("username not null-terminated")?
            + username_start;
        let username = String::from_utf8_lossy(&payload[username_start..username_end]).to_string();
        cur.set_position((username_end + 1) as u64);

        // Combine capability flags: the client sends 4 bytes at the start.
        // We read 2 bytes already; the next 2 are at offset 2-3 (overlapping with max_packet_size
        // start in our cursor position). Actually, the wire format is:
        //   capability_flags(4 bytes LE) max_packet_size(4) charset(1) reserved(23) username...
        // We read 2 bytes with get_u16_le, but need all 4. Re-read from raw payload.
        let capability_flags =
            capability_flags_lower | ((payload[2] as u32) << 8) | ((payload[3] as u32) << 16);

        // Auth response
        let pos = cur.position() as usize;
        let auth_response = if (capability_flags & CapabilityFlags::PLUGIN_AUTH_LENENC_CLIENT_DATA) != 0 {
            // Length-encoded auth response
            let (len, n) = read_lenenc_int(&payload[pos..])?;
            cur.set_position((pos + n + len) as u64);
            payload[pos + n..pos + n + len].to_vec()
        } else if (capability_flags & CapabilityFlags::SECURE_CONNECTION) != 0 {
            let len = payload[pos] as usize;
            cur.set_position((pos + 1 + len) as u64);
            payload[pos + 1..pos + 1 + len].to_vec()
        } else {
            // Null-terminated
            let end = payload[pos..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(payload.len() - pos);
            cur.set_position((pos + end + 1) as u64);
            payload[pos..pos + end].to_vec()
        };

        // Database
        let pos = cur.position() as usize;
        let database = if (capability_flags & CapabilityFlags::CONNECT_WITH_DB) != 0
            && pos < payload.len()
        {
            let end = payload[pos..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(payload.len() - pos);
            let db = String::from_utf8_lossy(&payload[pos..pos + end]).to_string();
            cur.set_position((pos + end + 1) as u64);
            Some(db)
        } else {
            None
        };

        // Auth plugin name
        let pos = cur.position() as usize;
        let auth_plugin_name = if (capability_flags & CapabilityFlags::PLUGIN_AUTH) != 0
            && pos < payload.len()
        {
            let end = payload[pos..]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(payload.len() - pos);
            let name = String::from_utf8_lossy(&payload[pos..pos + end]).to_string();
            Some(name)
        } else {
            None
        };

        Ok(Self {
            capability_flags,
            max_packet_size,
            charset,
            username,
            auth_response,
            database,
            auth_plugin_name,
        })
    }
}

/// Read a length-encoded integer from a byte slice.
/// Returns (value, bytes_consumed).
fn read_lenenc_int(buf: &[u8]) -> Result<(usize, usize), String> {
    if buf.is_empty() {
        return Err("buffer empty".to_string());
    }
    match buf[0] {
        0..=0xFA => Ok((buf[0] as usize, 1)),
        0xFC => {
            if buf.len() < 3 {
                return Err("buffer too short for lenenc int FC".to_string());
            }
            let v = u16::from_le_bytes([buf[1], buf[2]]) as usize;
            Ok((v, 3))
        }
        0xFD => {
            if buf.len() < 4 {
                return Err("buffer too short for lenenc int FD".to_string());
            }
            let v = (buf[1] as usize) | ((buf[2] as usize) << 8) | ((buf[3] as usize) << 16);
            Ok((v, 4))
        }
        0xFE => {
            if buf.len() < 9 {
                return Err("buffer too short for lenenc int FE".to_string());
            }
            let v = u64::from_le_bytes([
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
            ]);
            Ok((v as usize, 9))
        }
        _ => Err(format!("invalid lenenc int prefix: {:#x}", buf[0])),
    }
}

// ---------------------------------------------------------------------------
// OK packet
// ---------------------------------------------------------------------------

/// Build an OK response packet.
pub fn make_ok_packet(
    seq_id: u8,
    affected_rows: u64,
    last_insert_id: u64,
    status_flags: u16,
    warning_count: u16,
) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0x00); // OK header byte
    pb.lenenc_int(affected_rows);
    pb.lenenc_int(last_insert_id);
    pb.put_u16_le(status_flags);
    pb.put_u16_le(warning_count);
    let (packet, _) = pb.finish();
    packet
}

// ---------------------------------------------------------------------------
// ERR packet
// ---------------------------------------------------------------------------

/// Build an ERR response packet.
pub fn make_err_packet(seq_id: u8, error_code: u16, sql_state: &[u8; 5], message: &str) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0xFF); // ERR header byte
    pb.put_u16_le(error_code);
    pb.put_u8(b'#'); // SQL state marker
    pb.put_slice(sql_state);
    pb.put_slice(message.as_bytes());
    let (packet, _) = pb.finish();
    packet
}

/// Convenience: build an ERR packet with a HY000 general error.
pub fn make_general_err(seq_id: u8, error_code: u16, message: &str) -> BytesMut {
    make_err_packet(seq_id, error_code, b"HY000", message)
}

// ---------------------------------------------------------------------------
// EOF packet
// ---------------------------------------------------------------------------

/// Build an EOF packet (used when CLIENT_DEPRECATE_EOF is NOT set).
pub fn make_eof_packet(seq_id: u8, warning_count: u16, status_flags: u16) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0xFE); // EOF header byte
    pb.put_u16_le(warning_count);
    pb.put_u16_le(status_flags);
    let (packet, _) = pb.finish();
    packet
}

// ---------------------------------------------------------------------------
// Column definition packet (COM_QUERY response column)
// ---------------------------------------------------------------------------

/// A column descriptor used in result set responses.
#[derive(Debug, Clone)]
pub struct Column {
    pub schema: String,
    pub table: String,
    pub org_table: String,
    pub name: String,
    pub org_name: String,
    pub charset: u8,
    pub column_length: u32,
    pub column_type: u8,
    pub flags: u16,
    pub decimals: u8,
}

impl Column {
    pub fn new(name: &str, column_type: u8) -> Self {
        Self {
            schema: "def".to_string(),
            table: String::new(),
            org_table: String::new(),
            name: name.to_string(),
            org_name: name.to_string(),
            charset: charset::DEFAULT_CHARSET,
            column_length: 0,
            column_type,
            flags: 0,
            decimals: 0,
        }
    }

    pub fn with_schema(mut self, schema: &str) -> Self {
        self.schema = schema.to_string();
        self
    }

    pub fn with_table(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self.org_table = table.to_string();
        self
    }

    pub fn with_charset(mut self, charset: u8) -> Self {
        self.charset = charset;
        self
    }

    pub fn with_length(mut self, len: u32) -> Self {
        self.column_length = len;
        self
    }

    pub fn with_flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// Encode this column definition as a MySQL packet.
    pub fn encode(&self, seq_id: u8) -> BytesMut {
        let mut pb = PacketBuilder::new(seq_id);

        // For Protocol::ColumnDefinition41, we use lenenc encoding
        pb.lenenc_str("def");        // catalog (always "def")
        pb.lenenc_str(&self.schema);
        pb.lenenc_str(&self.table);
        pb.lenenc_str(&self.org_table);
        pb.lenenc_str(&self.name);
        pb.lenenc_str(&self.org_name);
        pb.lenenc_int(0x0C);         // length of fixed-length fields (always 0x0C)
        pb.put_u16_le(self.charset as u16);
        pb.put_u32_le(self.column_length);
        pb.put_u8(self.column_type);
        pb.put_u16_le(self.flags);
        pb.put_u8(self.decimals);
        pb.put_u16_le(0x0000);       // filler

        let (packet, _) = pb.finish();
        packet
    }
}

// ---------------------------------------------------------------------------
// Column type mapping from ScalarValue
// ---------------------------------------------------------------------------

/// Map a ScalarValue to a MySQL column type.
pub fn scalar_to_column_type(val: &ScalarValue) -> u8 {
    match val {
        ScalarValue::Null => column_type::NULL,
        ScalarValue::Boolean(_) => column_type::TINY,
        ScalarValue::Int8(_) => column_type::TINY,
        ScalarValue::Int16(_) => column_type::SHORT,
        ScalarValue::Int32(_) => column_type::LONG,
        ScalarValue::Int64(_) => column_type::LONGLONG,
        ScalarValue::Int128(_) => column_type::NEWDECIMAL,
        ScalarValue::Float32(_) => column_type::FLOAT,
        ScalarValue::Float64(_) => column_type::DOUBLE,
        ScalarValue::Date(_) => column_type::DATE,
        ScalarValue::DateTime(_) => column_type::DATETIME,
        ScalarValue::String(_) => column_type::VAR_STRING,
        ScalarValue::Binary(_) => column_type::BLOB,
        ScalarValue::Array(_) => column_type::BLOB,
    }
}

/// Map a DataType to a MySQL column type.
pub fn data_type_to_column_type(dt: &types::DataType) -> u8 {
    match dt {
        types::DataType::Null => column_type::NULL,
        types::DataType::Boolean => column_type::TINY,
        types::DataType::Int8 => column_type::TINY,
        types::DataType::Int16 => column_type::SHORT,
        types::DataType::Int32 => column_type::LONG,
        types::DataType::Int64 => column_type::LONGLONG,
        types::DataType::Int128 => column_type::NEWDECIMAL,
        types::DataType::Float32 => column_type::FLOAT,
        types::DataType::Float64 => column_type::DOUBLE,
        types::DataType::Decimal(_) => column_type::NEWDECIMAL,
        types::DataType::Date => column_type::DATE,
        types::DataType::DateTime => column_type::DATETIME,
        types::DataType::Varchar(_) | types::DataType::Char(_) | types::DataType::String => {
            column_type::VAR_STRING
        }
        types::DataType::Binary => column_type::BLOB,
        _ => column_type::BLOB,
    }
}

// ---------------------------------------------------------------------------
// Row encoding (text protocol for COM_QUERY)
// ---------------------------------------------------------------------------

/// Encode a single text-protocol row. Each value is a length-encoded string (or 0xFB for NULL).
pub fn encode_text_row(seq_id: u8, values: &[Option<Vec<u8>>]) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    for val in values {
        match val {
            Some(data) => {
                pb.lenenc_bytes(data);
            }
            None => {
                pb.put_u8(0xFB); // NULL
            }
        }
    }
    let (packet, _) = pb.finish();
    packet
}

/// Encode a ScalarValue to text protocol bytes.
pub fn scalar_to_text_bytes(val: &ScalarValue) -> Option<Vec<u8>> {
    match val {
        ScalarValue::Null => None,
        ScalarValue::Boolean(b) => Some(if *b { b"1".to_vec() } else { b"0".to_vec() }),
        ScalarValue::Int8(n) => Some(n.to_string().into_bytes()),
        ScalarValue::Int16(n) => Some(n.to_string().into_bytes()),
        ScalarValue::Int32(n) => Some(n.to_string().into_bytes()),
        ScalarValue::Int64(n) => Some(n.to_string().into_bytes()),
        ScalarValue::Int128(n) => Some(n.to_string().into_bytes()),
        ScalarValue::Float32(f) => Some(format!("{f:.prec$}", f = f, prec = 6).into_bytes()),
        ScalarValue::Float64(f) => Some({
            let s = format!("{f}");
            s.into_bytes()
        }),
        ScalarValue::Date(_days) => {
            Some(b"1970-01-01".to_vec())
        }
        ScalarValue::DateTime(_micros) => {
            Some(b"1970-01-01 00:00:00".to_vec())
        }
        ScalarValue::String(s) => Some(s.clone().into_bytes()),
        ScalarValue::Binary(b) => Some(b.clone()),
        ScalarValue::Array(_) => Some(b"[]".to_vec()),
    }
}

// ---------------------------------------------------------------------------
// Statement (prepared statement) helpers
// ---------------------------------------------------------------------------

/// Build a COM_STMT_PREPARE response.
pub fn make_stmt_prepare_ok(
    seq_id: u8,
    stmt_id: u32,
    num_columns: u16,
    num_params: u16,
    warning_count: u16,
) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0x00); // OK header for prepared statement
    pb.put_u32_le(stmt_id);
    pb.put_u16_le(num_columns);
    pb.put_u16_le(num_params);
    pb.put_u8(0x00); // filler
    pb.put_u16_le(warning_count);
    let (packet, _) = pb.finish();
    packet
}

// ---------------------------------------------------------------------------
// Auth switch request
// ---------------------------------------------------------------------------

/// Build an auth switch request packet (if we need to switch auth plugins).
pub fn make_auth_switch_request(seq_id: u8, plugin_name: &[u8], plugin_data: &[u8]) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0xFE); // status byte for auth switch
    pb.put_slice(plugin_name);
    pb.put_u8(0); // null terminator
    pb.put_slice(plugin_data);
    let (packet, _) = pb.finish();
    packet
}
