pub enum CodecType {
    None,
    Lz4,
    Zstd,
    Snappy,
}

pub fn encode(data: &[u8], codec: CodecType) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        CodecType::Lz4 => {
            // TODO: implement LZ4 compression
            data.to_vec()
        }
        CodecType::Zstd => {
            // TODO: implement Zstd compression
            data.to_vec()
        }
        CodecType::Snappy => {
            // TODO: implement Snappy compression
            data.to_vec()
        }
    }
}

pub fn decode(data: &[u8], codec: CodecType) -> Vec<u8> {
    match codec {
        CodecType::None => data.to_vec(),
        _ => {
            // TODO: implement decompression
            data.to_vec()
        }
    }
}
