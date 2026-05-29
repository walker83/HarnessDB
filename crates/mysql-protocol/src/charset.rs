/// MySQL charset ID constants.
/// See: https://dev.mysql.com/doc/refman/8.0/en/charset-charsets.html
pub const CHARSET_BINARY: u8 = 63;
pub const CHARSET_LATIN1: u8 = 8;
pub const CHARSET_GBK: u8 = 28;
pub const CHARSET_UTF8: u8 = 33; // utf8 (MySQL's 3-byte utf8)
pub const CHARSET_UTF8MB4: u8 = 45; // utf8mb4 (true UTF-8)

/// Default charset used in server greeting.
pub const DEFAULT_CHARSET: u8 = CHARSET_UTF8MB4;

/// Mapping from MySQL charset ID to the character set name.
pub fn charset_name(id: u8) -> &'static str {
    match id {
        CHARSET_BINARY => "binary",
        CHARSET_LATIN1 => "latin1",
        CHARSET_GBK => "gbk",
        CHARSET_UTF8 => "utf8",
        CHARSET_UTF8MB4 => "utf8mb4",
        _ => "unknown",
    }
}

/// Mapping from MySQL charset ID to the default collation name.
pub fn collation_name(id: u8) -> &'static str {
    match id {
        CHARSET_BINARY => "binary",
        CHARSET_LATIN1 => "latin1_swedish_ci",
        CHARSET_GBK => "gbk_chinese_ci",
        CHARSET_UTF8 => "utf8_general_ci",
        CHARSET_UTF8MB4 => "utf8mb4_general_ci",
        _ => "unknown",
    }
}

/// Get the max bytes per character for a charset.
pub fn max_bytes_per_char(id: u8) -> u8 {
    match id {
        CHARSET_BINARY => 1,
        CHARSET_LATIN1 => 1,
        CHARSET_GBK => 2,
        CHARSET_UTF8 => 3,
        CHARSET_UTF8MB4 => 4,
        _ => 1,
    }
}
