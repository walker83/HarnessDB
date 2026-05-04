use be_segment::codec::{CodecType, encode, decode, encode_with_level, compression_ratio};

// ===========================================================================
// P1 Optimization: Compression Algorithm Tests
// ===========================================================================

#[test]
fn test_codec_none() {
    let data = b"hello world";
    let compressed = encode(data, CodecType::None);
    assert_eq!(compressed, data);
    
    let decompressed = decode(&compressed, CodecType::None);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_lz4_basic() {
    let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let compressed = encode(data, CodecType::Lz4);
    
    assert!(compressed.len() < data.len());
    
    let decompressed = decode(&compressed, CodecType::Lz4);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_lz4_repeated_pattern() {
    let pattern = "abcabcabcabcabcabcabcabc";
    let data = pattern.as_bytes();
    let compressed = encode(data, CodecType::Lz4);
    
    assert!(compressed.len() < data.len());
    
    let decompressed = decode(&compressed, CodecType::Lz4);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_zstd_basic() {
    let data = b"repeat repeat repeat repeat repeat repeat";
    let compressed = encode(data, CodecType::Zstd);
    
    assert!(compressed.len() < data.len());
    
    let decompressed = decode(&compressed, CodecType::Zstd);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_zstd_compression_level() {
    let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    
    let compressed_low = encode_with_level(data, CodecType::Zstd, 1);
    let compressed_high = encode_with_level(data, CodecType::Zstd, 19);
    
    assert!(compressed_high.len() <= compressed_low.len());
    
    let decompressed_low = decode(&compressed_low, CodecType::Zstd);
    let decompressed_high = decode(&compressed_high, CodecType::Zstd);
    
    assert_eq!(decompressed_low, data);
    assert_eq!(decompressed_high, data);
}

#[test]
fn test_codec_snappy_basic() {
    let data = b"xyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyzxyz";
    let compressed = encode(data, CodecType::Snappy);
    
    assert!(compressed.len() < data.len());
    
    let decompressed = decode(&compressed, CodecType::Snappy);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_snappy_random_data() {
    let data: Vec<u8> = (0..1000).map(|i| (i * 7 % 256) as u8).collect();
    let compressed = encode(&data, CodecType::Snappy);
    let decompressed = decode(&compressed, CodecType::Snappy);
    assert_eq!(decompressed, data);
}

#[test]
fn test_codec_empty_data() {
    let data = b"";
    
    for codec in [CodecType::None, CodecType::Lz4, CodecType::Zstd, CodecType::Snappy] {
        let compressed = encode(data, codec);
        let decompressed = decode(&compressed, codec);
        assert_eq!(decompressed, data);
    }
}

#[test]
fn test_codec_large_data() {
    let data: Vec<u8> = (0..10000).map(|i| (i % 10) as u8).collect();
    
    for codec in [CodecType::Lz4, CodecType::Zstd, CodecType::Snappy] {
        let compressed = encode(&data, codec);
        assert!(compressed.len() < data.len());
        
        let decompressed = decode(&compressed, codec);
        assert_eq!(decompressed, data);
    }
}

#[test]
fn test_codec_type_from_str() {
    assert_eq!(CodecType::from_str("lz4"), CodecType::Lz4);
    assert_eq!(CodecType::from_str("LZ4"), CodecType::Lz4);
    assert_eq!(CodecType::from_str("zstd"), CodecType::Zstd);
    assert_eq!(CodecType::from_str("ZSTD"), CodecType::Zstd);
    assert_eq!(CodecType::from_str("snappy"), CodecType::Snappy);
    assert_eq!(CodecType::from_str("SNAPPY"), CodecType::Snappy);
    assert_eq!(CodecType::from_str("none"), CodecType::None);
    assert_eq!(CodecType::from_str(""), CodecType::None);
    assert_eq!(CodecType::from_str("invalid"), CodecType::None);
}

#[test]
fn test_codec_type_to_str() {
    assert_eq!(CodecType::None.to_str(), "none");
    assert_eq!(CodecType::Lz4.to_str(), "lz4");
    assert_eq!(CodecType::Zstd.to_str(), "zstd");
    assert_eq!(CodecType::Snappy.to_str(), "snappy");
}

#[test]
fn test_compression_ratio() {
    let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    
    let ratio_lz4 = compression_ratio(data, CodecType::Lz4);
    let ratio_zstd = compression_ratio(data, CodecType::Zstd);
    let ratio_snappy = compression_ratio(data, CodecType::Snappy);
    
    assert!(ratio_lz4 > 0.0);
    assert!(ratio_zstd > 0.0);
    assert!(ratio_snappy > 0.0);
    
    let ratio_none = compression_ratio(data, CodecType::None);
    assert_eq!(ratio_none, 0.0);
}

#[test]
fn test_compression_ratio_empty() {
    let data = b"";
    let ratio = compression_ratio(data, CodecType::Zstd);
    assert_eq!(ratio, 0.0);
}

#[test]
fn test_codec_incompressible_data() {
    let random_data: Vec<u8> = (0..1000).map(|i| ((i * 997) % 256) as u8).collect();
    
    for codec in [CodecType::Lz4, CodecType::Zstd, CodecType::Snappy] {
        let compressed = encode(&random_data, codec);
        let decompressed = decode(&compressed, codec);
        assert_eq!(decompressed, random_data);
    }
}

#[test]
fn test_decode_with_size() {
    use be_segment::codec::decode_with_size;
    
    let data = b"test data for decompression";
    let compressed = encode(data, CodecType::Zstd);
    let decompressed = decode_with_size(&compressed, CodecType::Zstd, data.len());
    assert_eq!(decompressed, data);
}

#[test]
fn test_lz4_high_compression_mode() {
    let data: Vec<u8> = vec![0xAB; 5000];
    
    let compressed_normal = encode(data.as_slice(), CodecType::Lz4);
    let compressed_high = encode_with_level(data.as_slice(), CodecType::Lz4, 12);
    
    assert!(compressed_high.len() <= compressed_normal.len());
    
    let decompressed = decode(&compressed_high, CodecType::Lz4);
    assert_eq!(decompressed, data);
}

#[test]
fn test_zstd_negative_level() {
    let data = b"some test data some test data some test data";
    
    let compressed = encode_with_level(data, CodecType::Zstd, -5);
    let decompressed = decode(&compressed, CodecType::Zstd);
    assert_eq!(decompressed, data);
}