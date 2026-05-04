use bytes::{BufMut, BytesMut};

/// Encode a length-encoded integer into the buffer.
/// Format: if < 251 => 1 byte; if < 65536 => 0xFC + 2 bytes; if < 16777216 => 0xFD + 3 bytes; else 0xFE + 8 bytes.
pub fn encode_lenenc_int(buf: &mut BytesMut, n: u64) {
    if n < 251 {
        buf.put_u8(n as u8);
    } else if n < 65536 {
        buf.put_u8(0xFC);
        buf.put_u16_le(n as u16);
    } else if n < 16777216 {
        buf.put_u8(0xFD);
        buf.put_u8((n & 0xFF) as u8);
        buf.put_u8(((n >> 8) & 0xFF) as u8);
        buf.put_u8(((n >> 16) & 0xFF) as u8);
    } else {
        buf.put_u8(0xFE);
        buf.put_u64_le(n);
    }
}

/// Encode a length-encoded string into the buffer.
pub fn encode_lenenc_string(buf: &mut BytesMut, s: &[u8]) {
    encode_lenenc_int(buf, s.len() as u64);
    buf.put_slice(s);
}

/// Encode a length-encoded string from a &str.
pub fn encode_lenenc_str(buf: &mut BytesMut, s: &str) {
    encode_lenenc_string(buf, s.as_bytes());
}

/// Encode a NULL value (0xFB sentinel).
pub fn encode_null(buf: &mut BytesMut) {
    buf.put_u8(0xFB);
}

/// Encode a date value in binary protocol format.
/// Format: length + year(2) + month(1) + day(1)
pub fn encode_date(buf: &mut BytesMut, year: u16, month: u8, day: u8) {
    buf.put_u8(4); // length
    buf.put_u16_le(year);
    buf.put_u8(month);
    buf.put_u8(day);
}

/// Encode a datetime value in binary protocol format.
/// Format: length + year(2) + month(1) + day(1) [+ hour(1) + minute(1) + second(1)]
#[allow(clippy::too_many_arguments)]
pub fn encode_datetime(
    buf: &mut BytesMut,
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    microsecond: u32,
) {
    if microsecond > 0 {
        buf.put_u8(11); // length
        buf.put_u16_le(year);
        buf.put_u8(month);
        buf.put_u8(day);
        buf.put_u8(hour);
        buf.put_u8(minute);
        buf.put_u8(second);
        buf.put_u32_le(microsecond);
    } else if hour > 0 || minute > 0 || second > 0 {
        buf.put_u8(7); // length
        buf.put_u16_le(year);
        buf.put_u8(month);
        buf.put_u8(day);
        buf.put_u8(hour);
        buf.put_u8(minute);
        buf.put_u8(second);
    } else {
        buf.put_u8(4); // length
        buf.put_u16_le(year);
        buf.put_u8(month);
        buf.put_u8(day);
    }
}

/// Encode a time value in binary protocol format.
pub fn encode_time(
    buf: &mut BytesMut,
    is_negative: bool,
    days: u32,
    hours: u8,
    minutes: u8,
    seconds: u8,
    microsecond: u32,
) {
    let neg_flag = if is_negative { 1u8 } else { 0u8 };
    if microsecond > 0 {
        buf.put_u8(12); // length
        buf.put_u8(neg_flag);
        buf.put_u32_le(days);
        buf.put_u8(hours);
        buf.put_u8(minutes);
        buf.put_u8(seconds);
        buf.put_u32_le(microsecond);
    } else {
        buf.put_u8(8); // length
        buf.put_u8(neg_flag);
        buf.put_u32_le(days);
        buf.put_u8(hours);
        buf.put_u8(minutes);
        buf.put_u8(seconds);
    }
}
