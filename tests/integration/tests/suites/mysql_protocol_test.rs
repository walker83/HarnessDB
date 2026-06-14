use bytes::BytesMut;
use mysql_protocol::charset;
use mysql_protocol::packet::{
    Column, HandshakeV10, PacketBuilder, column_type, data_type_to_column_type, encode_text_row,
    make_eof_packet, make_err_packet, make_general_err, make_ok_packet, make_stmt_prepare_ok,
    read_packet_header, scalar_to_column_type, scalar_to_text_bytes, write_packet_header,
};
use mysql_protocol::server::{ColumnDef, ColumnType, QueryHandler, QueryResult, ServerConfig};
use types::{DataType, ScalarValue};

// ===========================================================================
// Packet header tests
// ===========================================================================

#[test]
fn test_packet_header_write_and_read() {
    let mut buf = BytesMut::new();
    write_packet_header(&mut buf, 100, 1);

    let result = read_packet_header(&buf).unwrap();
    assert_eq!(result.0, 100); // length
    assert_eq!(result.1, 1); // sequence id
}

#[test]
fn test_packet_header_max_size() {
    let mut buf = BytesMut::new();
    write_packet_header(&mut buf, 0xFFFFFF, 255);

    let result = read_packet_header(&buf).unwrap();
    assert_eq!(result.0, 0xFFFFFF);
    assert_eq!(result.1, 255);
}

#[test]
fn test_packet_header_too_short() {
    let buf = BytesMut::from(&b"\x01\x02"[..]);
    assert!(read_packet_header(&buf).is_none());
}

// ===========================================================================
// PacketBuilder tests
// ===========================================================================

#[test]
fn test_packet_builder_basic() {
    let mut pb = PacketBuilder::new(0);
    pb.put_u8(0x00);
    pb.put_u16_le(0x0002);
    pb.put_u32_le(0x00000000);

    let (packet, next_seq) = pb.finish();
    assert_eq!(next_seq, 1);
    assert_eq!(packet.len(), 4 + 7); // 4 header + 7 payload
}

#[test]
fn test_packet_builder_lenenc_str() {
    let mut pb = PacketBuilder::new(0);
    pb.lenenc_str("hello");

    let (packet, _) = pb.finish();
    // header (4) + lenenc_int(1 byte for len=5) + "hello" (5 bytes)
    assert_eq!(packet.len(), 4 + 1 + 5);
}

// ===========================================================================
// Handshake packet tests
// ===========================================================================

#[test]
fn test_handshake_v10_encode() {
    let salt: [u8; 20] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11, 0x12, 0x13, 0x14,
    ];
    let handshake = HandshakeV10::new(42, salt);
    let packet = handshake.encode(0);

    // Verify the packet is well-formed
    assert!(packet.len() > 50);

    // Read the header
    let (payload_len, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 0);
    assert!(payload_len > 0);

    // Verify protocol version is 10
    assert_eq!(packet[4], 10);
}

// ===========================================================================
// OK packet tests
// ===========================================================================

#[test]
fn test_ok_packet() {
    let packet = make_ok_packet(1, 5, 100, 0x0002, 0);
    let (_payload_len, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 1);
    assert_eq!(packet[4], 0x00); // OK header byte
}

// ===========================================================================
// ERR packet tests
// ===========================================================================

#[test]
fn test_err_packet() {
    let packet = make_general_err(1, 1064, "syntax error");
    let (_payload_len, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 1);
    assert_eq!(packet[4], 0xFF); // ERR header byte
}

#[test]
fn test_err_packet_with_sql_state() {
    let sql_state = b"HY000";
    let packet = make_err_packet(1, 1064, sql_state, "general error");
    assert_eq!(packet[4], 0xFF); // ERR header
    // Check SQL state marker '#'
    assert_eq!(packet[7], b'#');
}

// ===========================================================================
// EOF packet tests
// ===========================================================================

#[test]
fn test_eof_packet() {
    let packet = make_eof_packet(3, 0, 0x0002);
    let (_payload_len, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 3);
    assert_eq!(packet[4], 0xFE); // EOF header byte
}

// ===========================================================================
// Column definition tests
// ===========================================================================

#[test]
fn test_column_encode() {
    let col = Column::new("test_col", column_type::LONGLONG);
    let packet = col.encode(0);

    let (payload_len, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 0);
    assert!(payload_len > 0);
}

#[test]
fn test_column_with_options() {
    let col = Column::new("name", column_type::VAR_STRING)
        .with_schema("mydb")
        .with_table("mytable")
        .with_length(255);

    assert_eq!(col.schema, "mydb");
    assert_eq!(col.table, "mytable");
    assert_eq!(col.column_length, 255);
    assert_eq!(col.column_type, column_type::VAR_STRING);
}

// ===========================================================================
// Prepared statement response tests
// ===========================================================================

#[test]
fn test_stmt_prepare_ok() {
    let packet = make_stmt_prepare_ok(1, 42, 3, 2, 0);
    let (_, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 1);
    assert_eq!(packet[4], 0x00); // OK header
}

// ===========================================================================
// Row encoding tests
// ===========================================================================

#[test]
fn test_encode_text_row() {
    let row = vec![Some(b"hello".to_vec()), Some(b"123".to_vec()), None];

    let packet = encode_text_row(0, &row);
    let (_, seq_id) = read_packet_header(&packet).unwrap();
    assert_eq!(seq_id, 0);
}

// ===========================================================================
// Scalar to column type mapping tests
// ===========================================================================

#[test]
fn test_scalar_to_column_type_mapping() {
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Int64(0)),
        column_type::LONGLONG
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Int32(0)),
        column_type::LONG
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Float64(0.0)),
        column_type::DOUBLE
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Float32(0.0)),
        column_type::FLOAT
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Boolean(true)),
        column_type::TINY
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::String("".into())),
        column_type::VAR_STRING
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::Date(0)),
        column_type::DATE
    );
    assert_eq!(
        scalar_to_column_type(&ScalarValue::DateTime(0)),
        column_type::DATETIME
    );
    assert_eq!(scalar_to_column_type(&ScalarValue::Null), column_type::NULL);
}

#[test]
fn test_data_type_to_column_type_mapping() {
    assert_eq!(
        data_type_to_column_type(&DataType::Int64),
        column_type::LONGLONG
    );
    assert_eq!(
        data_type_to_column_type(&DataType::Int32),
        column_type::LONG
    );
    assert_eq!(
        data_type_to_column_type(&DataType::Float64),
        column_type::DOUBLE
    );
    assert_eq!(
        data_type_to_column_type(&DataType::String),
        column_type::VAR_STRING
    );
    assert_eq!(
        data_type_to_column_type(&DataType::Boolean),
        column_type::TINY
    );
    assert_eq!(data_type_to_column_type(&DataType::Date), column_type::DATE);
}

// ===========================================================================
// Scalar to text bytes tests
// ===========================================================================

#[test]
fn test_scalar_to_text_bytes() {
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Int64(42)),
        Some(b"42".to_vec())
    );
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Boolean(true)),
        Some(b"1".to_vec())
    );
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Boolean(false)),
        Some(b"0".to_vec())
    );
    assert_eq!(scalar_to_text_bytes(&ScalarValue::Null), None);
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::String("hello".into())),
        Some(b"hello".to_vec())
    );
}

// Regression test for GitHub issue #1: DATE and DATETIME types must produce
// correct text representations instead of the hardcoded "1970-01-01".
#[test]
fn test_scalar_to_text_bytes_date() {
    // 2026-05-29: days since epoch = 20603
    // Verify round-trip: epoch day 0 → "1970-01-01"
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Date(0)),
        Some(b"1970-01-01".to_vec())
    );
    // 2024-01-15: 19738 days since epoch
    let days_2024_01_15 = chrono::NaiveDate::from_ymd_opt(2024, 1, 15)
        .unwrap()
        .signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days() as i32;
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Date(days_2024_01_15)),
        Some(b"2024-01-15".to_vec())
    );
    // 2026-05-29
    let days_2026_05_29 = chrono::NaiveDate::from_ymd_opt(2026, 5, 29)
        .unwrap()
        .signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days() as i32;
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Date(days_2026_05_29)),
        Some(b"2026-05-29".to_vec())
    );
    // Leap year date: 2024-02-29
    let days_leap = chrono::NaiveDate::from_ymd_opt(2024, 2, 29)
        .unwrap()
        .signed_duration_since(chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .num_days() as i32;
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::Date(days_leap)),
        Some(b"2024-02-29".to_vec())
    );
}

#[test]
fn test_scalar_to_text_bytes_datetime() {
    // Epoch → "1970-01-01 00:00:00"
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::DateTime(0)),
        Some(b"1970-01-01 00:00:00".to_vec())
    );
    // 2026-05-31 10:30:00 UTC in microseconds since epoch
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 5, 31)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap()
        .and_utc();
    let micros = dt.timestamp() * 1_000_000;
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::DateTime(micros)),
        Some(b"2026-05-31 10:30:00".to_vec())
    );
    // 2024-02-29 23:59:59 UTC
    let dt2 = chrono::NaiveDate::from_ymd_opt(2024, 2, 29)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap()
        .and_utc();
    let micros2 = dt2.timestamp() * 1_000_000;
    assert_eq!(
        scalar_to_text_bytes(&ScalarValue::DateTime(micros2)),
        Some(b"2024-02-29 23:59:59".to_vec())
    );
}

// ===========================================================================
// Charset tests
// ===========================================================================

#[test]
fn test_charset_constants() {
    assert_eq!(charset::CHARSET_BINARY, 63);
    assert_eq!(charset::CHARSET_UTF8MB4, 45);
    assert_eq!(charset::DEFAULT_CHARSET, charset::CHARSET_UTF8MB4);
}

#[test]
fn test_charset_name_mapping() {
    assert_eq!(charset::charset_name(charset::CHARSET_UTF8MB4), "utf8mb4");
    assert_eq!(charset::charset_name(charset::CHARSET_UTF8), "utf8");
    assert_eq!(charset::charset_name(charset::CHARSET_BINARY), "binary");
    assert_eq!(charset::charset_name(0), "unknown");
}

#[test]
fn test_collation_name_mapping() {
    assert_eq!(
        charset::collation_name(charset::CHARSET_UTF8MB4),
        "utf8mb4_general_ci"
    );
    assert_eq!(
        charset::collation_name(charset::CHARSET_UTF8),
        "utf8_general_ci"
    );
}

#[test]
fn test_max_bytes_per_char() {
    assert_eq!(charset::max_bytes_per_char(charset::CHARSET_UTF8MB4), 4);
    assert_eq!(charset::max_bytes_per_char(charset::CHARSET_UTF8), 3);
    assert_eq!(charset::max_bytes_per_char(charset::CHARSET_BINARY), 1);
}

// ===========================================================================
// QueryResult tests
// ===========================================================================

#[test]
fn test_query_result_new() {
    let columns = vec![ColumnDef {
        name: "id".into(),
        col_type: ColumnType::Int,
    }];
    let result = QueryResult::new(columns);
    assert!(result.rows.is_empty());
    assert_eq!(result.columns.len(), 1);
}

#[test]
fn test_query_result_with_rows() {
    let columns = vec![ColumnDef {
        name: "name".into(),
        col_type: ColumnType::String,
    }];
    let rows = vec![vec![Some("Alice".into())], vec![Some("Bob".into())]];
    let result = QueryResult::with_rows(columns, rows);
    assert_eq!(result.rows.len(), 2);
}

#[test]
fn test_query_result_ok() {
    let result = QueryResult::ok();
    assert!(result.columns.is_empty());
    assert!(result.rows.is_empty());
}

// ===========================================================================
// ColumnDef to Column conversion
// ===========================================================================

#[test]
fn test_column_def_to_column_conversion() {
    let col_def = ColumnDef {
        name: "my_col".into(),
        col_type: ColumnType::Int,
    };
    let col: Column = (&col_def).into();
    assert_eq!(col.name, "my_col");
    assert_eq!(col.column_type, column_type::LONGLONG);

    let col_def2 = ColumnDef {
        name: "str_col".into(),
        col_type: ColumnType::String,
    };
    let col2: Column = (&col_def2).into();
    assert_eq!(col2.column_type, column_type::VAR_STRING);
}

// ===========================================================================
// ServerConfig tests
// ===========================================================================

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();
    assert_eq!(config.bind_addr, "127.0.0.1");
    assert_eq!(config.port, 9030);
}

// ===========================================================================
// QueryHandler trait test (mock implementation)
// ===========================================================================

struct MockQueryHandler;

impl QueryHandler for MockQueryHandler {
    fn handle_query(&self, _conn_id: u32, sql: &str) -> QueryResult {
        if sql.starts_with("SELECT") {
            QueryResult::with_rows(
                vec![ColumnDef {
                    name: "result".into(),
                    col_type: ColumnType::Int,
                }],
                vec![vec![Some("42".into())]],
            )
        } else {
            QueryResult::ok()
        }
    }
}

#[test]
fn test_mock_query_handler() {
    let handler = MockQueryHandler;

    let result = handler.handle_query(0, "SELECT 1");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.columns.len(), 1);

    let result = handler.handle_query(0, "CREATE TABLE t (id INT)");
    assert!(result.rows.is_empty());
}
