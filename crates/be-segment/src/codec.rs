pub enum CodecType {
    None,
    Lz4,
    Zstd,
    Snappy,
}

impl CodecType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lz4" => CodecType::Lz4,
            "zstd" => CodecType::Zstd,
            "snappy" => CodecType::Snappy,
            "none" | "" => CodecType::None,
            _ => CodecType::None,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            CodecType::None => "none",
            CodecType::Lz4 => "lz4",
            CodecType::Zstd => "zstd",
            CodecType::Snappy => "snappy",
        }
    }
}

pub fn encode(data: &[u8], codec: CodecType) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        CodecType::Lz4 => {
            lz4::block::compress(data, Some(lz4::block::CompressionMode::DEFAULT))
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Zstd => {
            zstd::encode_all(data, 3)
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Snappy => {
            snap::raw::Encoder::new()
                .compress_vec(data)
                .unwrap_or_else(|_| data.to_vec())
        }
    }
}

pub fn encode_with_level(data: &[u8], codec: CodecType, level: i32) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        CodecType::Lz4 => {
            let mode = if level > 0 {
                lz4::block::CompressionMode::HIGHCOMPRESSION(level.min(16))
            } else {
                lz4::block::CompressionMode::FAST(level.abs().min(16))
            };
            lz4::block::compress(data, Some(mode))
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Zstd => {
            zstd::encode_all(data, level.min(22).max(-22))
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Snappy => {
            snap::raw::Encoder::new()
                .compress_vec(data)
                .unwrap_or_else(|_| data.to_vec())
        }
    }
}

pub fn decode(data: &[u8], codec: CodecType) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        CodecType::Lz4 => {
            lz4::block::decompress(data, Some(data.len() * 4))
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Zstd => {
            zstd::decode_all(data)
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Snappy => {
            snap::raw::Decoder::new()
                .decompress_vec(data)
                .unwrap_or_else(|_| data.to_vec())
        }
    }
}

pub fn decode_with_size(data: &[u8], codec: CodecType, uncompressed_size: usize) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        CodecType::Lz4 => {
            lz4::block::decompress(data, Some(uncompressed_size))
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Zstd => {
            zstd::decode_all(data)
                .unwrap_or_else(|_| data.to_vec())
        }
        CodecType::Snappy => {
            snap::raw::Decoder::new()
                .decompress_vec(data)
                .unwrap_or_else(|_| data.to_vec())
        }
    }
}

pub fn compression_ratio(data: &[u8], codec: CodecType) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let compressed = encode(data, codec);
    1.0 - (compressed.len() as f64 / data.len() as f64)
}
