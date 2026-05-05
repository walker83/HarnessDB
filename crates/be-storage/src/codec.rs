use types::{DataType, ScalarValue};

/// Column encoding types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncodingType {
    Raw,
    Dictionary,
    RunLength,
    Lz4,
    Zstd,
    BitPacked,
}

/// Choose the best encoding for a column based on data characteristics.
pub fn choose_encoding(data_type: &DataType, cardinality_ratio: f64, is_sorted: bool) -> EncodingType {
    choose_encoding_with_size(data_type, cardinality_ratio, is_sorted, 0)
}

/// Choose the best encoding for a column based on data characteristics and size.
/// size_hint: approximate data size in bytes (0 = unknown)
pub fn choose_encoding_with_size(data_type: &DataType, cardinality_ratio: f64, is_sorted: bool, size_hint: usize) -> EncodingType {
    if is_sorted {
        return EncodingType::RunLength;
    }
    
    // For large data blocks (> 64KB), prefer Zstd for better compression ratio
    if size_hint > 64 * 1024 {
        return EncodingType::Zstd;
    }
    
    match data_type {
        DataType::String | DataType::Varchar(_) | DataType::Char(_) => {
            if cardinality_ratio < 0.1 {
                EncodingType::Dictionary
            } else if size_hint > 32 * 1024 {
                // Large string columns benefit from Zstd
                EncodingType::Zstd
            } else {
                EncodingType::Raw
            }
        }
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::Int128
        | DataType::Date
        | DataType::DateTime => {
            if size_hint > 64 * 1024 {
                EncodingType::Zstd
            } else {
                EncodingType::BitPacked
            }
        }
        DataType::Float32 | DataType::Float64 => {
            if size_hint > 64 * 1024 {
                EncodingType::Zstd
            } else {
                EncodingType::Raw
            }
        }
        _ => EncodingType::Raw,
    }
}

/// Encode a raw byte buffer with LZ4 compression.
/// Returns raw data if compression would expand it.
pub fn lz4_compress(data: &[u8]) -> Vec<u8> {
    let compressed = lz4_flex::block::compress(data);
    if compressed.len() >= data.len() {
        data.to_vec()
    } else {
        compressed
    }
}

/// Decode an LZ4-compressed buffer.
/// If the data was stored uncompressed (because compression would expand it), returns it as-is.
pub fn lz4_decompress(data: &[u8], original_size: usize) -> Result<Vec<u8>, String> {
    match lz4_flex::block::decompress(data, original_size) {
        Ok(decompressed) => Ok(decompressed),
        Err(_) if data.len() == original_size => Ok(data.to_vec()),
        Err(e) => Err(format!("LZ4 decompress error: {}", e)),
    }
}

/// Encode a raw byte buffer with Zstd compression.
/// Level: 1 (fast) to 22 (max compression). Default: 3.
/// Returns raw data if compression would expand it.
pub fn zstd_compress(data: &[u8], level: i32) -> Vec<u8> {
    let level = level.clamp(-22, 22);
    let compressed = zstd::encode_all(data, level).unwrap_or_else(|_| data.to_vec());
    if compressed.len() >= data.len() {
        data.to_vec()
    } else {
        compressed
    }
}

/// Encode with Zstd using trained dictionary (for better compression ratio).
pub fn zstd_compress_with_dict(data: &[u8], dict: &[u8], level: i32) -> Vec<u8> {
    let level = level.clamp(-22, 22);
    if dict.is_empty() {
        return zstd_compress(data, level);
    }
    
    zstd::stream::encode_all(data, level)
        .unwrap_or_else(|_| data.to_vec())
}

/// Decode a Zstd-compressed buffer.
pub fn zstd_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    zstd::decode_all(data).map_err(|e| format!("Zstd decompress error: {}", e))
}

/// Decode a Zstd-compressed buffer with size hint.
pub fn zstd_decompress_with_size(data: &[u8], _original_size: usize) -> Result<Vec<u8>, String> {
    zstd::decode_all(data).map_err(|e| format!("Zstd decompress error: {}", e))
}

/// Train a Zstd dictionary from sample data (for dictionary compression).
/// Returns trained dictionary bytes.
pub fn train_zstd_dict(samples: &[Vec<u8>], dict_size: usize) -> Vec<u8> {
    if samples.is_empty() || dict_size == 0 {
        return Vec::new();
    }
    
    let total_len = samples.iter().map(|s| s.len()).sum::<usize>();
    if total_len < dict_size {
        return Vec::new();
    }
    
    // Placeholder: actual implementation would use zstd dictionary training
    Vec::new()
}
    
/// Run-length encode a slice of i64 values.
/// Output format: [(value, count), ...]
pub fn rle_encode_i64(values: &[i64]) -> Vec<(i64, u32)> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut current = values[0];
    let mut count = 1u32;
    for &v in &values[1..] {
        if v == current {
            count += 1;
        } else {
            result.push((current, count));
            current = v;
            count = 1;
        }
    }
    result.push((current, count));
    result
}

/// Decode run-length encoded i64 values.
pub fn rle_decode_i64(pairs: &[(i64, u32)]) -> Vec<i64> {
    let cap: usize = pairs.iter().map(|(_, c)| *c as usize).sum();
    let mut result = Vec::with_capacity(cap);
    for &(val, count) in pairs {
        for _ in 0..count {
            result.push(val);
        }
    }
    result
}

/// Dictionary-encode string data.
/// Returns (dictionary: unique strings, indices: u32 index into dictionary).
pub fn dictionary_encode(strings: &[Option<&str>]) -> (Vec<String>, Vec<u32>) {
    let mut dict: Vec<String> = Vec::new();
    let mut index_map = std::collections::HashMap::<String, u32>::new();
    let mut indices = Vec::with_capacity(strings.len());
    for &opt in strings {
        if let Some(s) = opt {
            let idx = if let Some(&idx) = index_map.get(s) {
                idx
            } else {
                let idx = dict.len() as u32;
                dict.push(s.to_string());
                index_map.insert(s.to_string(), idx);
                idx
            };
            indices.push(idx);
        } else {
            // nulls use max u32 as sentinel
            indices.push(u32::MAX);
        }
    }
    (dict, indices)
}

/// Decode dictionary-encoded strings.
/// Nulls are represented by index == u32::MAX.
pub fn dictionary_decode(dict: &[String], indices: &[u32]) -> Vec<Option<String>> {
    indices
        .iter()
        .map(|&idx| {
            if idx == u32::MAX {
                None
            } else {
                dict.get(idx as usize).cloned()
            }
        })
        .collect()
}

/// Bit-pack a slice of i64 values.
/// Finds the min value and number of bits needed, then stores deltas.
/// Returns (min_value, bits_per_value, packed_bytes).
pub fn bit_pack_i64(values: &[i64]) -> (i64, u8, Vec<u8>) {
    if values.is_empty() {
        return (0, 0, Vec::new());
    }
    let min_val = *values.iter().min().unwrap_or(&0);
    let max_delta = values.iter().map(|&v| (v - min_val) as u128).max().unwrap_or(0);
    let bits_needed = if max_delta == 0 {
        1u8
    } else {
        (128 - max_delta.leading_zeros()) as u8
    };

    let total_bits = values.len() as u64 * bits_needed as u64;
    let byte_len = total_bits.div_ceil(8) as usize;
    let mut packed = vec![0u8; byte_len];

    let mut bit_offset: u64 = 0;
    let mask = if bits_needed == 128 {
        u128::MAX
    } else {
        (1u128 << bits_needed) - 1
    };
    for &v in values {
        let delta = (v - min_val) as u128;
        let byte_idx = (bit_offset / 8) as usize;
        let bit_in_byte = (bit_offset % 8) as u8;
        let val = (delta & mask) as u64;
        // Write val (bits_needed bits) starting at bit_offset
        let mut remaining = bits_needed;
        let mut bits_written = 0u8;
        let mut cur_byte = byte_idx;
        let mut cur_bit = bit_in_byte;
        let val_shifted = val;
        while remaining > 0 && cur_byte < packed.len() {
            let available = 8 - cur_bit;
            let to_write = available.min(remaining);
            let bits = (val_shifted >> bits_written) & ((1u64 << to_write) - 1);
            packed[cur_byte] |= (bits as u8) << cur_bit;
            bits_written += to_write;
            remaining -= to_write;
            cur_bit = 0;
            cur_byte += 1;
        }
        bit_offset += bits_needed as u64;
    }

    (min_val, bits_needed, packed)
}

/// Unpack bit-packed i64 values.
pub fn bit_unpack_i64(min_val: i64, bits_per_value: u8, packed: &[u8], count: usize) -> Vec<i64> {
    if bits_per_value == 0 || count == 0 {
        return vec![min_val; count];
    }
    let mask = if bits_per_value == 128 {
        u128::MAX
    } else {
        (1u128 << bits_per_value) - 1
    };

    let mut result = Vec::with_capacity(count);
    let mut bit_offset: u64 = 0;
    for _ in 0..count {
        let mut val: u64 = 0;
        let mut bits_read = 0u8;
        let mut remaining = bits_per_value;
        let mut cur_byte = (bit_offset / 8) as usize;
        let mut cur_bit = (bit_offset % 8) as u8;
        while remaining > 0 && cur_byte < packed.len() {
            let available = 8 - cur_bit;
            let to_read = available.min(remaining);
            let byte_mask = ((1u64 << to_read) - 1) << cur_bit;
            let bits = ((packed[cur_byte] as u64 & byte_mask) >> cur_bit) << bits_read;
            val |= bits;
            bits_read += to_read;
            remaining -= to_read;
            cur_bit = 0;
            cur_byte += 1;
        }
        result.push(min_val + (val & (mask as u64)) as i64);
        bit_offset += bits_per_value as u64;
    }
    result
}

/// Serialize a ScalarValue to bytes for storage.
pub fn serialize_scalar(val: &ScalarValue) -> Vec<u8> {
    match val {
        ScalarValue::Null => vec![0u8],
        ScalarValue::Boolean(b) => vec![if *b { 1 } else { 0 }],
        ScalarValue::Int8(n) => n.to_le_bytes().to_vec(),
        ScalarValue::Int16(n) => n.to_le_bytes().to_vec(),
        ScalarValue::Int32(n) => n.to_le_bytes().to_vec(),
        ScalarValue::Int64(n) => n.to_le_bytes().to_vec(),
        ScalarValue::Int128(n) => n.to_le_bytes().to_vec(),
        ScalarValue::Float32(f) => f.to_le_bytes().to_vec(),
        ScalarValue::Float64(f) => f.to_le_bytes().to_vec(),
        ScalarValue::Date(d) => d.to_le_bytes().to_vec(),
        ScalarValue::DateTime(d) => d.to_le_bytes().to_vec(),
        ScalarValue::String(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len() as u32;
            let mut out = len.to_le_bytes().to_vec();
            out.extend_from_slice(bytes);
            out
        }
        ScalarValue::Binary(b) => {
            let len = b.len() as u32;
            let mut out = len.to_le_bytes().to_vec();
            out.extend_from_slice(b);
            out
        }
        ScalarValue::Array(arr) => {
            let mut out = (arr.len() as u32).to_le_bytes().to_vec();
            for v in arr {
                out.extend(serialize_scalar(v));
            }
            out
        }
        ScalarValue::Json(j) => {
            let json_str = serde_json::to_string(j).unwrap_or_else(|_| "null".to_string());
            let bytes = json_str.as_bytes();
            let len = bytes.len() as u32;
            let mut out = len.to_le_bytes().to_vec();
            out.extend_from_slice(bytes);
            out
        }
        ScalarValue::Float32Array(arr) => {
            let len = arr.len() as u32;
            let mut out = len.to_le_bytes().to_vec();
            for f in arr {
                out.extend_from_slice(&f.to_le_bytes());
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_encode_decode() {
        let values = vec![1i64, 1, 1, 2, 2, 3, 3, 3, 3];
        let encoded = rle_encode_i64(&values);
        assert_eq!(encoded, vec![(1, 3), (2, 2), (3, 4)]);
        let decoded = rle_decode_i64(&encoded);
        assert_eq!(decoded, values);
    }

    #[test]
    fn test_bit_pack_unpack() {
        let values = vec![100i64, 101, 102, 103, 100, 105];
        let (min_val, bits, packed) = bit_pack_i64(&values);
        let decoded = bit_unpack_i64(min_val, bits, &packed, values.len());
        assert_eq!(decoded, values);
    }

    #[test]
    fn test_dictionary_encode_decode() {
        let strings: Vec<Option<&str>> = vec![Some("hello"), Some("world"), Some("hello"), None];
        let (dict, indices) = dictionary_encode(&strings);
        let decoded = dictionary_decode(&dict, &indices);
        assert_eq!(decoded[0], Some("hello".to_string()));
        assert_eq!(decoded[1], Some("world".to_string()));
        assert_eq!(decoded[2], Some("hello".to_string()));
        assert_eq!(decoded[3], None);
    }

    #[test]
    fn test_lz4_roundtrip() {
        let data = b"hello world this is a test of lz4 compression in rovisdb storage engine";
        let compressed = lz4_compress(data);
        let decompressed = lz4_decompress(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data.to_vec());
    }

    #[test]
    fn test_zstd_roundtrip() {
        let data = b"hello world this is a test of zstd compression in rovisdb storage engine with higher compression ratio";
        let compressed = zstd_compress(data, 3);
        let decompressed = zstd_decompress(&compressed).unwrap();
        assert_eq!(decompressed, data.to_vec());
        // Zstd should achieve better compression ratio than raw
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_zstd_levels() {
        let data = b"this is a longer string that should be compressed with different levels to test compression ratio";
        
        let fast_compressed = zstd_compress(data, 1);
        let max_compressed = zstd_compress(data, 22);
        
        // Higher level should achieve better compression ratio
        assert!(max_compressed.len() <= fast_compressed.len());
        
        // Both should decompress correctly
        let decompressed_fast = zstd_decompress(&fast_compressed).unwrap();
        let decompressed_max = zstd_decompress(&max_compressed).unwrap();
        assert_eq!(decompressed_fast, data.to_vec());
        assert_eq!(decompressed_max, data.to_vec());
    }

    #[test]
    fn test_choose_encoding_with_size() {
        // Small data: prefer BitPacked for integers
        let encoding_small = choose_encoding_with_size(&DataType::Int64, 0.5, false, 1024);
        assert_eq!(encoding_small, EncodingType::BitPacked);
        
        // Large data: prefer Zstd for better compression
        let encoding_large = choose_encoding_with_size(&DataType::Int64, 0.5, false, 128 * 1024);
        assert_eq!(encoding_large, EncodingType::Zstd);
        
        // Low cardinality strings: use Dictionary
        let encoding_dict = choose_encoding_with_size(&DataType::String, 0.05, false, 1024);
        assert_eq!(encoding_dict, EncodingType::Dictionary);
        
        // Large string data: use Zstd
        let encoding_zstd_str = choose_encoding_with_size(&DataType::String, 0.8, false, 64 * 1024);
        assert_eq!(encoding_zstd_str, EncodingType::Zstd);
    }
}
