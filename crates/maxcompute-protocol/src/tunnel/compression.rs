//! MaxCompute Tunnel compression support.
//!
//! Supports ZLIB (deflate) compression for upload and download.
//! Content-Encoding: deflate on upload, Accept-Encoding: deflate on download.

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use std::io::{self, Read, Write};

/// Decompress ZLIB/deflate data.
pub fn decompress_deflate(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

/// Compress data using ZLIB/deflate.
pub fn compress_deflate(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let data = b"Hello, MaxCompute Tunnel!";
        let compressed = compress_deflate(data).unwrap();
        assert!(compressed.len() < data.len() + 20, "compression overhead should be reasonable");
        let decompressed = decompress_deflate(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_decompress_invalid_data() {
        let result = decompress_deflate(b"not valid zlib data");
        assert!(result.is_err(), "decompressing invalid data should fail");
    }

    #[test]
    fn test_compress_empty() {
        let data = b"";
        let compressed = compress_deflate(data).unwrap();
        let decompressed = decompress_deflate(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
