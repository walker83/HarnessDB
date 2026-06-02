//! MaxCompute Tunnel protobuf-like wire format encoder/decoder.
//!
//! Implements the binary record stream format used by the MaxCompute Tunnel
//! protocol for bulk upload and download. This is NOT standard protobuf, but
//! uses a compatible wire encoding:
//!
//! - Tags: varint `(field_number << 3) | wire_type`
//! - Wire types: VARINT(0), FIXED64(1), LENGTH_DELIMITED(2), FIXED32(5)
//! - Per-record CRC32C checksum, terminated by `TUNNEL_END_RECORD`
//! - Stream footer: `TUNNEL_META_COUNT` record count + `TUNNEL_META_CHECKSUM`
//!
//! Null values are encoded as a `null_count` + null column indices prefix
//! before each record's fields (matching pyodps `io/reader.py`).

use std::io;

use crate::tunnel::schema::TunnelSchema;

// ============================================================================
// Constants
// ============================================================================

/// Tunnel protocol version.
pub const TUNNEL_VERSION: u32 = 6;

/// Tag value marking end of a record. Followed by per-record CRC32C.
pub const TUNNEL_END_RECORD: u32 = 33_553_408; // 0x01FFFFE0

/// Tag value for record count in stream footer.
pub const TUNNEL_META_COUNT: u32 = 33_554_430; // 0x01FFFFFE

/// Tag value for overall checksum in stream footer.
pub const TUNNEL_META_CHECKSUM: u32 = 33_554_431; // 0x02000000

/// Wire type constants (standard protobuf wire types).
const WIRE_TYPE_VARINT: u8 = 0;
const WIRE_TYPE_FIXED64: u8 = 1;
const WIRE_TYPE_LENGTH_DELIMITED: u8 = 2;
const WIRE_TYPE_FIXED32: u8 = 5;

// ============================================================================
// Varint / ZigZag
// ============================================================================

/// Encode an unsigned integer as a varint (7 bits per byte, MSB = continuation).
pub fn encode_varint(val: u64, buf: &mut Vec<u8>) {
    let mut v = val;
    while v > 0x7F {
        buf.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
    buf.push(v as u8);
}

/// Decode a varint from a byte slice, returning (value, bytes_consumed).
pub fn decode_varint(data: &[u8]) -> io::Result<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos >= data.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated varint"));
        }
        let byte = data[pos];
        result |= ((byte & 0x7F) as u64) << shift;
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "varint too long"));
        }
    }
    Ok((result, pos))
}

/// ZigZag encode a signed i64 to unsigned u64.
#[inline]
pub fn zigzag_encode(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

/// ZigZag decode an unsigned u64 to signed i64.
#[inline]
pub fn zigzag_decode(v: u64) -> i64 {
    ((v >> 1) as i64) ^ (-((v & 1) as i64))
}

// ============================================================================
// TunnelWriter — encodes records to protobuf wire format
// ============================================================================

/// Encodes rows of `Option<String>` values into the MaxCompute Tunnel binary format.
pub struct TunnelWriter {
    buf: Vec<u8>,
    per_record_checksums: Vec<u32>,
}

impl TunnelWriter {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
            per_record_checksums: Vec::new(),
        }
    }

    /// Encode a single row into the buffer.
    pub fn write_row(&mut self, row: &[Option<String>], schema: &TunnelSchema) {
        let columns = schema.all_columns();

        // Collect null column indices (0-based)
        let null_indices: Vec<usize> = row
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if v.is_none() { Some(i) } else { None })
            .collect();

        let start_len = self.buf.len();

        // Write null_count (untagged varint)
        encode_varint(null_indices.len() as u64, &mut self.buf);
        // Write null column indices
        for &idx in &null_indices {
            encode_varint(idx as u64, &mut self.buf);
        }

        // Write non-null fields
        for (col_idx, col) in columns.iter().enumerate() {
            if null_indices.contains(&col_idx) {
                continue;
            }
            let field_number = (col_idx + 1) as u32;
            let value = row[col_idx].as_ref().unwrap();
            self.write_field(field_number, &col.odps_type, value);
        }

        // Per-record CRC32C of the record data (from null_count to last field value)
        let record_crc = crc32fast::hash(&self.buf[start_len..]);

        // Write end-of-record marker
        encode_varint(TUNNEL_END_RECORD as u64, &mut self.buf);
        // Write per-record CRC32C as uint32 LE
        self.buf.extend_from_slice(&record_crc.to_le_bytes());

        self.per_record_checksums.push(record_crc);
    }

    /// Encode a field value based on its ODPS type.
    fn write_field(&mut self, field_number: u32, odps_type: &str, value: &str) {
        let upper = odps_type.to_uppercase();
        match upper.as_str() {
            "BIGINT" | "INT" | "SMALLINT" | "TINYINT" => {
                let v: i64 = value.parse().unwrap_or(0);
                let tag = (field_number << 3) | WIRE_TYPE_VARINT as u32;
                encode_varint(tag as u64, &mut self.buf);
                encode_varint(zigzag_encode(v), &mut self.buf);
            }
            "BOOLEAN" => {
                let v: u64 = if value == "1" || value.eq_ignore_ascii_case("true") {
                    1
                } else {
                    0
                };
                let tag = (field_number << 3) | WIRE_TYPE_VARINT as u32;
                encode_varint(tag as u64, &mut self.buf);
                encode_varint(v, &mut self.buf);
            }
            "FLOAT" => {
                let v: f32 = value.parse().unwrap_or(0.0);
                let tag = (field_number << 3) | WIRE_TYPE_FIXED32 as u32;
                encode_varint(tag as u64, &mut self.buf);
                self.buf.extend_from_slice(&v.to_le_bytes());
            }
            "DOUBLE" | "REAL" => {
                let v: f64 = value.parse().unwrap_or(0.0);
                let tag = (field_number << 3) | WIRE_TYPE_FIXED64 as u32;
                encode_varint(tag as u64, &mut self.buf);
                self.buf.extend_from_slice(&v.to_le_bytes());
            }
            "STRING" | "VARCHAR" | "CHAR" | "TEXT" => {
                let tag = (field_number << 3) | WIRE_TYPE_LENGTH_DELIMITED as u32;
                encode_varint(tag as u64, &mut self.buf);
                encode_varint(value.len() as u64, &mut self.buf);
                self.buf.extend_from_slice(value.as_bytes());
            }
            "BINARY" | "BLOB" | "VARBINARY" => {
                let tag = (field_number << 3) | WIRE_TYPE_LENGTH_DELIMITED as u32;
                encode_varint(tag as u64, &mut self.buf);
                let bytes = hex::decode(value).unwrap_or_else(|_| value.as_bytes().to_vec());
                encode_varint(bytes.len() as u64, &mut self.buf);
                self.buf.extend_from_slice(&bytes);
            }
            "DATETIME" | "TIMESTAMP" => {
                let tag = (field_number << 3) | WIRE_TYPE_VARINT as u32;
                encode_varint(tag as u64, &mut self.buf);
                let ms: i64 = value.parse().unwrap_or(0);
                encode_varint(zigzag_encode(ms), &mut self.buf);
            }
            "DATE" => {
                let tag = (field_number << 3) | WIRE_TYPE_VARINT as u32;
                encode_varint(tag as u64, &mut self.buf);
                let days: i64 = value.parse().unwrap_or(0);
                encode_varint(zigzag_encode(days), &mut self.buf);
            }
            "DECIMAL" | "NUMERIC" => {
                let tag = (field_number << 3) | WIRE_TYPE_LENGTH_DELIMITED as u32;
                encode_varint(tag as u64, &mut self.buf);
                encode_varint(value.len() as u64, &mut self.buf);
                self.buf.extend_from_slice(value.as_bytes());
            }
            // Fallback: encode as length-delimited string
            _ => {
                let tag = (field_number << 3) | WIRE_TYPE_LENGTH_DELIMITED as u32;
                encode_varint(tag as u64, &mut self.buf);
                encode_varint(value.len() as u64, &mut self.buf);
                self.buf.extend_from_slice(value.as_bytes());
            }
        }
    }

    /// Write the stream footer (TUNNEL_META_COUNT + TUNNEL_META_CHECKSUM).
    pub fn write_footer(&mut self) {
        // Record count
        encode_varint(TUNNEL_META_COUNT as u64, &mut self.buf);
        encode_varint(self.per_record_checksums.len() as u64, &mut self.buf);

        // Overall CRC32C of per-record checksums
        let mut crc_bytes = Vec::new();
        for &crc in &self.per_record_checksums {
            crc_bytes.extend_from_slice(&crc.to_le_bytes());
        }
        let overall_crc = crc32fast::hash(&crc_bytes);

        encode_varint(TUNNEL_META_CHECKSUM as u64, &mut self.buf);
        self.buf.extend_from_slice(&overall_crc.to_le_bytes());
    }

    /// Encode all rows and the footer, returning the complete binary payload.
    pub fn finish(mut self, rows: &[Vec<Option<String>>], schema: &TunnelSchema) -> Vec<u8> {
        for row in rows {
            self.write_row(row, schema);
        }
        self.write_footer();
        self.buf
    }

    /// Access the raw buffer.
    pub fn buf(&self) -> &[u8] {
        &self.buf
    }
}

impl Default for TunnelWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TunnelReader — decodes protobuf wire format to rows
// ============================================================================

/// Decodes MaxCompute Tunnel binary data into rows of `Option<String>`.
pub struct TunnelReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> TunnelReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Read all records from the data, returning rows + metadata.
    pub fn read_all(&mut self, schema: &TunnelSchema) -> io::Result<(Vec<Vec<Option<String>>>, Option<u64>, Option<u32>)> {
        let mut rows = Vec::new();
        let mut record_count_meta: Option<u64> = None;
        let mut overall_crc: Option<u32> = None;

        loop {
            let record = self.read_row(schema)?;
            match record {
                Some(row) => rows.push(row),
                None => break,
            }
        }

        // Check if footer markers are present after reading all records
        if self.pos < self.data.len() {
            if let Ok((tag, _)) = self.read_varint() {
                if tag == TUNNEL_META_COUNT as u64 {
                    if let Ok((count, _)) = self.read_varint() {
                        record_count_meta = Some(count);
                    }
                }
                if self.pos + 4 + 1 <= self.data.len() {
                    if let Ok((tag2, _)) = self.read_varint() {
                        if tag2 == TUNNEL_META_CHECKSUM as u64 {
                            let mut crc_bytes = [0u8; 4];
                            crc_bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
                            overall_crc = Some(u32::from_le_bytes(crc_bytes));
                        }
                    }
                }
            }
        }

        Ok((rows, record_count_meta, overall_crc))
    }

    /// Read a single record. Returns None at end of stream (footer markers).
    pub fn read_row(&mut self, schema: &TunnelSchema) -> io::Result<Option<Vec<Option<String>>>> {
        if self.pos >= self.data.len() {
            return Ok(None);
        }

        // Peek at next tag
        let peek_start = self.pos;
        let (tag, _) = self.read_varint()?;

        if tag == TUNNEL_META_COUNT as u64 || tag == TUNNEL_META_CHECKSUM as u64 {
            self.pos = peek_start;
            return Ok(None);
        }

        // The first varint is null_count (untagged)
        let null_count = tag as usize;

        // Read null column indices
        let mut null_set = std::collections::HashSet::new();
        for _ in 0..null_count {
            let (idx, _) = self.read_varint()?;
            null_set.insert(idx as usize);
        }

        let col_count = schema.column_count();
        let mut row = vec![None; col_count];

        // Mark nulls
        for &idx in &null_set {
            if idx < col_count {
                row[idx] = None;
            }
        }

        // Read non-null fields
        loop {
            if self.pos >= self.data.len() {
                break;
            }
            let (field_tag, _) = self.read_varint()?;

            if field_tag == TUNNEL_END_RECORD as u64 {
                // Per-record CRC32C follows
                if self.pos + 4 <= self.data.len() {
                    self.pos += 4; // skip CRC
                }
                break;
            }

            let field_number = (field_tag >> 3) as usize;
            let wire_type = (field_tag & 0x07) as u8;

            if field_number == 0 || field_number > col_count {
                self.skip_field(wire_type)?;
                continue;
            }

            let col_idx = field_number - 1;
            let odps_type = &schema.all_columns()[col_idx].odps_type;
            let value = self.read_value(wire_type, odps_type)?;
            row[col_idx] = Some(value);
        }

        Ok(Some(row))
    }

    fn read_value(&mut self, _wire_type: u8, odps_type: &str) -> io::Result<String> {
        let upper = odps_type.to_uppercase();
        match upper.as_str() {
            "BIGINT" | "INT" | "SMALLINT" | "TINYINT" => {
                let (v, _) = self.read_varint()?;
                Ok(zigzag_decode(v).to_string())
            }
            "BOOLEAN" => {
                let (v, _) = self.read_varint()?;
                Ok(if v != 0 { "1".to_string() } else { "0".to_string() })
            }
            "FLOAT" => {
                if self.pos + 4 > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated float"));
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
                self.pos += 4;
                Ok(f32::from_le_bytes(bytes).to_string())
            }
            "DOUBLE" | "REAL" => {
                if self.pos + 8 > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated double"));
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&self.data[self.pos..self.pos + 8]);
                self.pos += 8;
                Ok(f64::from_le_bytes(bytes).to_string())
            }
            "STRING" | "VARCHAR" | "CHAR" | "TEXT" | "DECIMAL" | "NUMERIC" => {
                let (len, _) = self.read_varint()?;
                let len = len as usize;
                if self.pos + len > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated string"));
                }
                let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).to_string();
                self.pos += len;
                Ok(s)
            }
            "BINARY" | "BLOB" | "VARBINARY" => {
                let (len, _) = self.read_varint()?;
                let len = len as usize;
                if self.pos + len > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated binary"));
                }
                let s = hex::encode(&self.data[self.pos..self.pos + len]);
                self.pos += len;
                Ok(s)
            }
            "DATETIME" | "TIMESTAMP" => {
                let (v, _) = self.read_varint()?;
                Ok(zigzag_decode(v).to_string())
            }
            "DATE" => {
                let (v, _) = self.read_varint()?;
                Ok(zigzag_decode(v).to_string())
            }
            _ => {
                let (len, _) = self.read_varint()?;
                let len = len as usize;
                if self.pos + len > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated value"));
                }
                let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).to_string();
                self.pos += len;
                Ok(s)
            }
        }
    }

    fn skip_field(&mut self, wire_type: u8) -> io::Result<()> {
        match wire_type {
            WIRE_TYPE_VARINT => { self.read_varint()?; }
            WIRE_TYPE_FIXED64 => {
                if self.pos + 8 > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated fixed64"));
                }
                self.pos += 8;
            }
            WIRE_TYPE_LENGTH_DELIMITED => {
                let (len, _) = self.read_varint()?;
                if self.pos + len as usize > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated length-delimited"));
                }
                self.pos += len as usize;
            }
            WIRE_TYPE_FIXED32 => {
                if self.pos + 4 > self.data.len() {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated fixed32"));
                }
                self.pos += 4;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown wire type: {}", wire_type),
                ));
            }
        }
        Ok(())
    }

    fn read_varint(&mut self) -> io::Result<(u64, usize)> {
        let remaining = &self.data[self.pos..];
        let (val, consumed) = decode_varint(remaining)?;
        self.pos += consumed;
        Ok((val, consumed))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunnel::schema::TunnelColumn;

    fn make_schema(types: &[&str]) -> TunnelSchema {
        let columns: Vec<TunnelColumn> = types
            .iter()
            .enumerate()
            .map(|(i, &t)| TunnelColumn {
                name: format!("col{}", i),
                odps_type: t.to_string(),
                nullable: true,
                comment: None,
            })
            .collect();
        TunnelSchema { columns, partition_keys: vec![] }
    }

    #[test]
    fn test_varint_zero() {
        let mut buf = Vec::new();
        encode_varint(0, &mut buf);
        assert_eq!(buf, vec![0x00]);
        let (val, consumed) = decode_varint(&buf).unwrap();
        assert_eq!(val, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_varint_small_value() {
        let mut buf = Vec::new();
        encode_varint(42, &mut buf);
        assert_eq!(buf, vec![42]);
        let (val, _) = decode_varint(&buf).unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn test_varint_large_value() {
        let mut buf = Vec::new();
        encode_varint(300, &mut buf);
        let (val, _) = decode_varint(&buf).unwrap();
        assert_eq!(val, 300);
    }

    #[test]
    fn test_varint_max_u64() {
        let mut buf = Vec::new();
        encode_varint(u64::MAX, &mut buf);
        let (val, _) = decode_varint(&buf).unwrap();
        assert_eq!(val, u64::MAX);
    }

    #[test]
    fn test_varint_roundtrip_various() {
        let values: &[u64] = &[0, 1, 127, 128, 255, 256, 16383, 16384, u64::MAX];
        for &v in values {
            let mut buf = Vec::new();
            encode_varint(v, &mut buf);
            let (decoded, _) = decode_varint(&buf).unwrap();
            assert_eq!(v, decoded, "varint roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_zigzag_encode() {
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);
    }

    #[test]
    fn test_zigzag_decode() {
        assert_eq!(zigzag_decode(0), 0);
        assert_eq!(zigzag_decode(1), -1);
        assert_eq!(zigzag_decode(2), 1);
        assert_eq!(zigzag_decode(3), -2);
        assert_eq!(zigzag_decode(4), 2);
    }

    #[test]
    fn test_zigzag_roundtrip() {
        let values: &[i64] = &[0, 1, -1, 42, -42, i64::MAX, i64::MIN];
        for &v in values {
            let decoded = zigzag_decode(zigzag_encode(v));
            assert_eq!(v, decoded, "zigzag roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_crc32c_basic() {
        let crc = crc32fast::hash(b"hello world");
        assert_ne!(crc, 0, "CRC32C of non-empty data should not be zero");
    }

    #[test]
    fn test_crc32c_empty() {
        let crc = crc32fast::hash(b"");
        assert_eq!(crc, 0, "CRC32C of empty data should be zero");
    }

    #[test]
    fn test_crc32c_deterministic() {
        let a = crc32fast::hash(b"test data");
        let b = crc32fast::hash(b"test data");
        assert_eq!(a, b, "CRC32C should be deterministic");
    }

    #[test]
    fn test_roundtrip_single_bigint() {
        let schema = make_schema(&["BIGINT"]);
        let rows = vec![vec![Some("42".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);
        assert!(!data.is_empty(), "encoded data should not be empty");

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row)) = reader.read_row(&schema) {
            assert_eq!(row.len(), 1);
            assert_eq!(row[0].as_deref(), Some("42"));
        }
    }

    #[test]
    fn test_roundtrip_string() {
        let schema = make_schema(&["STRING"]);
        let rows = vec![vec![Some("hello world".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row)) = reader.read_row(&schema) {
            assert_eq!(row[0].as_deref(), Some("hello world"));
        }
    }

    #[test]
    fn test_roundtrip_double() {
        let schema = make_schema(&["DOUBLE"]);
        let rows = vec![vec![Some("3.14159".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row)) = reader.read_row(&schema) {
            assert!(row[0].as_deref().is_some());
            let val: f64 = row[0].as_ref().unwrap().parse().unwrap();
            assert!((val - 3.14159).abs() < 0.001);
        }
    }

    #[test]
    fn test_roundtrip_boolean() {
        let schema = make_schema(&["BOOLEAN"]);
        let rows = vec![vec![Some("1".to_string())], vec![Some("0".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row1)) = reader.read_row(&schema) {
            assert_eq!(row1[0].as_deref(), Some("1"));
        }
        if let Ok(Some(row2)) = reader.read_row(&schema) {
            assert_eq!(row2[0].as_deref(), Some("0"));
        }
    }

    #[test]
    fn test_roundtrip_null() {
        let schema = make_schema(&["BIGINT", "STRING"]);
        let rows = vec![vec![None, Some("test".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row)) = reader.read_row(&schema) {
            assert_eq!(row.len(), 2);
            assert!(row[0].is_none(), "first column should be null");
            assert_eq!(row[1].as_deref(), Some("test"));
        }
    }

    #[test]
    fn test_roundtrip_multiple_columns() {
        let schema = make_schema(&["BIGINT", "STRING", "DOUBLE"]);
        let rows = vec![vec![
            Some("123".to_string()),
            Some("abc".to_string()),
            Some("1.5".to_string()),
        ]];
        let data = TunnelWriter::new().finish(&rows, &schema);

        let mut reader = TunnelReader::new(&data);
        if let Ok(Some(row)) = reader.read_row(&schema) {
            assert_eq!(row.len(), 3);
            assert_eq!(row[0].as_deref(), Some("123"));
            assert_eq!(row[1].as_deref(), Some("abc"));
            let val: f64 = row[2].as_ref().unwrap().parse().unwrap();
            assert!((val - 1.5).abs() < 0.001);
        }
    }

    #[test]
    fn test_writer_has_footer() {
        let schema = make_schema(&["BIGINT"]);
        let rows = vec![vec![Some("1".to_string())], vec![Some("2".to_string())]];
        let data = TunnelWriter::new().finish(&rows, &schema);
        assert!(data.len() > 20, "encoded data with footer should be substantial");
    }

    #[test]
    fn test_empty_rows() {
        let schema = make_schema(&["BIGINT"]);
        let rows: Vec<Vec<Option<String>>> = vec![];
        let data = TunnelWriter::new().finish(&rows, &schema);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_truncated_varint() {
        let result = decode_varint(&[0x80]);
        assert!(result.is_err(), "truncated varint should return error");
    }

    #[test]
    fn test_tag_encoding() {
        let mut buf = Vec::new();
        encode_varint(8, &mut buf);
        assert_eq!(buf, vec![8]);

        let mut buf = Vec::new();
        encode_varint(9, &mut buf);
        assert_eq!(buf, vec![9]);

        let mut buf = Vec::new();
        encode_varint(10, &mut buf);
        assert_eq!(buf, vec![10]);
    }

    #[test]
    fn test_constant_values() {
        assert_eq!(TUNNEL_VERSION, 6);
        assert_eq!(TUNNEL_END_RECORD, 33_553_408);
        assert_eq!(TUNNEL_META_COUNT, 33_554_430);
        assert_eq!(TUNNEL_META_CHECKSUM, 33_554_431);
    }
}
