pub enum IndexType {
    BitMap,
    BloomFilter,
    Inverted,
    ZoneMap,
}

pub struct SegmentIndex {
    pub index_type: IndexType,
    pub column_name: String,
    pub data: Vec<u8>,
}

impl SegmentIndex {
    pub fn lookup(&self, _key: &[u8]) -> bool {
        // TODO: implement index lookup
        false
    }
}
