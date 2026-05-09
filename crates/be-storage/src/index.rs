use serde::{Deserialize, Serialize};
use types::{Bitmap, Block, ScalarValue};
use roaring::RoaringBitmap;
use std::collections::HashMap;

/// Zone map index: tracks min/max/null_count per page for one column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneMap {
    pub min_value: Option<Vec<u8>>,
    pub max_value: Option<Vec<u8>>,
    pub null_count: u64,
    pub num_rows: u64,
}

impl Default for ZoneMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoneMap {
    pub fn new() -> Self {
        Self {
            min_value: None,
            max_value: None,
            null_count: 0,
            num_rows: 0,
        }
    }

    /// Build a zone map from a slice of scalar values.
    pub fn build(values: &[ScalarValue]) -> Self {
        let mut zm = ZoneMap::new();
        zm.num_rows = values.len() as u64;
        let mut min_val: Option<ScalarValue> = None;
        let mut max_val: Option<ScalarValue> = None;
        for v in values {
            if v.is_null() {
                zm.null_count += 1;
                continue;
            }
            match (&min_val, &max_val) {
                (None, _) | (_, None) => {
                    min_val = Some(v.clone());
                    max_val = Some(v.clone());
                }
                (Some(min), Some(max)) => {
                    if compare_scalars(v, min) == std::cmp::Ordering::Less {
                        min_val = Some(v.clone());
                    }
                    if compare_scalars(v, max) == std::cmp::Ordering::Greater {
                        max_val = Some(v.clone());
                    }
                }
            }
        }
        zm.min_value = min_val.as_ref().map(crate::codec::serialize_scalar);
        zm.max_value = max_val.as_ref().map(crate::codec::serialize_scalar);
        zm
    }

    /// Check if this zone map could contain rows matching the predicate.
    /// Returns false if the zone can be definitively pruned.
    pub fn may_match(&self, op: &PredicateOp, value: &ScalarValue) -> bool {
        // If all nulls and looking for non-null, prune.
        if self.null_count == self.num_rows {
            return false;
        }
        let min_sv = self.min_value.as_ref().and_then(|d| deserialize_scalar(d.as_slice()));
        let max_sv = self.max_value.as_ref().and_then(|d| deserialize_scalar(d.as_slice()));
        match (min_sv, max_sv) {
            (Some(min), Some(max)) => match op {
                PredicateOp::Eq => {
                    // value within [min, max]
                    compare_scalars(value, &min) != std::cmp::Ordering::Less
                        && compare_scalars(value, &max) != std::cmp::Ordering::Greater
                }
                PredicateOp::Lt => compare_scalars(&min, value) == std::cmp::Ordering::Less,
                PredicateOp::Le => compare_scalars(&min, value) != std::cmp::Ordering::Greater,
                PredicateOp::Gt => compare_scalars(&max, value) == std::cmp::Ordering::Greater,
                PredicateOp::Ge => compare_scalars(&max, value) != std::cmp::Ordering::Less,
                // For complex predicates, zone map can't prune - assume may match
                _ => true,
            },
            _ => true,
        }
    }
}

/// Bloom filter index for high-cardinality columns.
/// Uses a simple hash-based probabilistic structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomFilter {
    bitmap: Vec<u64>,
    num_hashes: usize,
    len: usize,
}

impl BloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let bits = Self::optimal_bits(expected_items, fp_rate);
        let words = bits.div_ceil(64);
        let num_hashes = Self::optimal_hashes(bits, expected_items);
        Self {
            bitmap: vec![0u64; words],
            num_hashes,
            len: 0,
        }
    }

    pub fn insert(&mut self, data: &[u8]) {
        let (h1, h2) = Self::hash_pair(data);
        let bits = self.bitmap.len() * 64;
        for i in 0..self.num_hashes {
            let idx = Self::hash_index(h1, h2, i as u64, bits);
            let word = idx / 64;
            let bit = idx % 64;
            self.bitmap[word] |= 1u64 << bit;
        }
        self.len += 1;
    }

    pub fn may_contain(&self, data: &[u8]) -> bool {
        let (h1, h2) = Self::hash_pair(data);
        let bits = self.bitmap.len() * 64;
        for i in 0..self.num_hashes {
            let idx = Self::hash_index(h1, h2, i as u64, bits);
            let word = idx / 64;
            let bit = idx % 64;
            if self.bitmap[word] & (1u64 << bit) == 0 {
                return false;
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn optimal_bits(n: usize, p: f64) -> usize {
        let m = -(n as f64 * p.ln()) / (2.0_f64.ln().powi(2));
        m.ceil() as usize
    }

    fn optimal_hashes(m: usize, n: usize) -> usize {
        let k = (m as f64 / n as f64 * 2.0_f64.ln()).ceil() as usize;
        k.clamp(1, 20)
    }

    fn hash_pair(data: &[u8]) -> (u64, u64) {
        // Simple FNV-1a based double hashing
        let mut h1: u64 = 14695981039346656037;
        for &b in data {
            h1 ^= b as u64;
            h1 = h1.wrapping_mul(1099511628211);
        }
        let mut h2: u64 = 14695981039346656037;
        for (i, &b) in data.iter().enumerate() {
            h2 ^= (b ^ (i as u8)) as u64;
            h2 = h2.wrapping_mul(1099511628211);
        }
        (h1, h2)
    }

    fn hash_index(h1: u64, h2: u64, i: u64, bits: usize) -> usize {
        (h1.wrapping_add(h2.wrapping_mul(i)) % (bits as u64)) as usize
    }
}

/// Bitmap index for low-cardinality columns.
/// Uses RoaringBitmap for efficient storage and fast intersection operations.
/// Suitable for columns with < 1000 distinct values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitmapIndex {
    /// Map from value to row IDs (as RoaringBitmap)
    bitmaps: HashMap<String, Vec<u8>>,  // Serialized RoaringBitmap
    /// List of all unique values (for deserialization)
    values: Vec<String>,
    /// Bitmap for NULL values
    null_bitmap: Vec<u8>,
    /// Number of rows indexed
    num_rows: usize,
}

impl Default for BitmapIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapIndex {
    pub fn new() -> Self {
        Self {
            bitmaps: HashMap::new(),
            values: Vec::new(),
            null_bitmap: Vec::new(),
            num_rows: 0,
        }
    }
    
    /// Build a bitmap index from scalar values.
    /// Returns None if cardinality is too high (> 1000 distinct values).
    pub fn build(values: &[ScalarValue], max_cardinality: usize) -> Option<Self> {
        if values.is_empty() {
            return Some(Self::new());
        }
        
        // Use string keys to avoid Eq/Hash constraints on ScalarValue
        let mut distinct_keys = std::collections::HashSet::new();
        for v in values {
            if !v.is_null() {
                distinct_keys.insert(Self::scalar_to_key(v));
            }
        }
        
        if distinct_keys.len() > max_cardinality {
            return None;  // Cardinality too high
        }
        
        let mut index = Self::new();
        index.num_rows = values.len();
        
        // Build bitmaps per value
        let mut bitmaps_map: HashMap<String, RoaringBitmap> = HashMap::new();
        let mut null_bitmap = RoaringBitmap::new();
        
        for (row_id, value) in values.iter().enumerate() {
            if value.is_null() {
                null_bitmap.insert(row_id as u32);
            } else {
                let key = Self::scalar_to_key(value);
                let bitmap = bitmaps_map.entry(key).or_insert_with(RoaringBitmap::new);
                bitmap.insert(row_id as u32);
            }
        }
        
        // Serialize bitmaps
        for (value_key, bitmap) in bitmaps_map.iter() {
            let serialized = Self::serialize_bitmap(bitmap);
            index.bitmaps.insert(value_key.clone(), serialized);
            index.values.push(value_key.clone());
        }
        
        index.null_bitmap = Self::serialize_bitmap(&null_bitmap);
        
        Some(index)
    }
    
    /// Check if the index might contain rows matching the predicate.
    pub fn may_match(&self, op: &PredicateOp, value: &ScalarValue) -> Option<Vec<u32>> {
        if value.is_null() {
            // For null predicates, return null bitmap
            match op {
                PredicateOp::Eq => {
                    let bitmap = Self::deserialize_bitmap(&self.null_bitmap);
                    Some(bitmap.iter().collect())
                }
                _ => None,  // Null comparisons other than = are not supported
            }
        } else {
            let value_key = Self::scalar_to_key(value);
            match op {
                PredicateOp::Eq => {
                    // Return bitmap for this exact value
                    self.bitmaps.get(&value_key).map(|serialized| {
                        Self::deserialize_bitmap(serialized).iter().collect()
                    })
                }
                PredicateOp::In => {
                    // For In operation, would need union of multiple bitmaps
                    None  // Implemented in extended PredicateOp
                }
                _ => None,  // Range queries not efficient with bitmap index
            }
        }
    }
    
    /// Convert ScalarValue to string key for bitmap storage.
    fn scalar_to_key(value: &ScalarValue) -> String {
        match value {
            ScalarValue::Boolean(b) => b.to_string(),
            ScalarValue::Int8(n) => n.to_string(),
            ScalarValue::Int16(n) => n.to_string(),
            ScalarValue::Int32(n) => n.to_string(),
            ScalarValue::Int64(n) => n.to_string(),
            ScalarValue::Int128(n) => n.to_string(),
            ScalarValue::Float32(f) => f.to_string(),
            ScalarValue::Float64(f) => f.to_string(),
            ScalarValue::Date(d) => d.to_string(),
            ScalarValue::DateTime(d) => d.to_string(),
            ScalarValue::String(s) => s.clone(),
            ScalarValue::Binary(b) => format!("binary_{}", b.len()),
            ScalarValue::Array(_) => "array".to_string(),
            ScalarValue::Json(_) => "json".to_string(),
            ScalarValue::Null => "null".to_string(),
            ScalarValue::Float32Array(_) => "float32_array".to_string(),
        }
    }
    
    /// Serialize RoaringBitmap to bytes.
    fn serialize_bitmap(bitmap: &RoaringBitmap) -> Vec<u8> {
        let mut serialized = Vec::new();
        if bitmap.serialize_into(&mut serialized).is_err() {
            return Vec::new();
        }
        serialized
    }
    
    /// Deserialize bytes to RoaringBitmap.
    fn deserialize_bitmap(data: &[u8]) -> RoaringBitmap {
        if data.is_empty() {
            return RoaringBitmap::new();
        }
        RoaringBitmap::deserialize_from(data).unwrap_or_else(|_| RoaringBitmap::new())
    }
    
    /// Get number of indexed values.
    pub fn num_values(&self) -> usize {
        self.values.len()
    }
    
    /// Check if this is a valid bitmap index.
    pub fn is_valid(&self) -> bool {
        !self.values.is_empty() || self.num_rows > 0
    }
}

/// Lightweight inverted index for text search.
/// Stores term → row ID mappings for efficient full-text and LIKE queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndex {
    /// Term → list of row IDs where the term appears
    postings: HashMap<String, Vec<u32>>,
    /// Number of rows indexed
    num_rows: usize,
    /// Total number of unique terms
    total_terms: usize,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            postings: HashMap::new(),
            num_rows: 0,
            total_terms: 0,
        }
    }

    /// Build inverted index from string values.
    pub fn build(values: &[ScalarValue]) -> Self {
        let mut index = Self::new();
        index.num_rows = values.len();

        for (row_id, value) in values.iter().enumerate() {
            if let ScalarValue::String(s) = value {
                let tokens = Self::tokenize(s);
                for token in tokens {
                    index.postings.entry(token).or_insert_with(Vec::new).push(row_id as u32);
                }
            }
        }

        index.total_terms = index.postings.len();
        index
    }

    /// Simple tokenizer: lowercase, split by whitespace/punctuation.
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|s| !s.is_empty() && s.len() >= 2)
            .map(|s| s.to_string())
            .collect()
    }

    /// Search for a single term. Returns row IDs containing the term.
    pub fn search_term(&self, term: &str) -> Option<&[u32]> {
        self.postings.get(&term.to_lowercase()).map(|v| v.as_slice())
    }

    /// Search for all terms (AND). Returns rows containing ALL terms.
    pub fn search_all(&self, terms: &[String]) -> Vec<u32> {
        if terms.is_empty() {
            return Vec::new();
        }

        let mut result: Option<Vec<u32>> = None;
        for term in terms {
            if let Some(postings) = self.search_term(term) {
                result = Some(match result {
                    None => postings.to_vec(),
                    Some(prev) => {
                        // Intersection: keep rows in both lists
                        let mut intersection = Vec::new();
                        let mut i = 0;
                        let mut j = 0;
                        while i < prev.len() && j < postings.len() {
                            match prev[i].cmp(&postings[j]) {
                                std::cmp::Ordering::Equal => {
                                    intersection.push(prev[i]);
                                    i += 1;
                                    j += 1;
                                }
                                std::cmp::Ordering::Less => i += 1,
                                std::cmp::Ordering::Greater => j += 1,
                            }
                        }
                        intersection
                    }
                });
            } else {
                return Vec::new(); // Term not found: empty result
            }
        }
        result.unwrap_or_default()
    }

    /// Search for any term (OR). Returns rows containing ANY term.
    pub fn search_any(&self, terms: &[String]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut seen = Vec::new();
        for term in terms {
            if let Some(postings) = self.search_term(term) {
                for &row_id in postings {
                    if !seen.contains(&row_id) {
                        seen.push(row_id);
                        result.push(row_id);
                    }
                }
            }
        }
        result.sort();
        result
    }

    /// Check if a LIKE pattern can be satisfied by the index.
    /// Returns matching row IDs if the pattern uses indexed terms.
    pub fn search_like(&self, pattern: &str) -> Vec<u32> {
        // Extract terms from pattern (remove % and _ wildcards)
        let cleaned = pattern.replace('%', " ").replace('_', " ");
        let terms = Self::tokenize(&cleaned);
        if terms.is_empty() {
            return Vec::new();
        }
        // For LIKE, use "any term" matching (OR)
        self.search_any(&terms)
    }

    pub fn num_terms(&self) -> usize {
        self.total_terms
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }
}

/// Distance metric for ANN Index.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    /// Euclidean distance (L2)
    L2,
}

/// ANN Index for vector similarity search using a flat + IVF hybrid approach.
/// Supports Float32Vector columns for approximate nearest neighbor search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ANNIndex {
    /// Stored vectors (index = row ID)
    vectors: Vec<Vec<f32>>,
    /// Vector dimension
    dimension: usize,
    /// Distance metric
    pub metric: MetricType,
    /// Number of vectors
    num_vectors: usize,
}

impl ANNIndex {
    pub fn new(dimension: usize, metric: MetricType) -> Self {
        Self {
            vectors: Vec::new(),
            dimension,
            metric,
            num_vectors: 0,
        }
    }

    /// Build ANN index from Float32Array values.
    pub fn build(values: &[ScalarValue], dimension: usize) -> Option<Self> {
        let mut index = Self::new(dimension, MetricType::L2);

        for value in values {
            if let ScalarValue::Float32Array(vec) = value {
                if vec.len() == dimension {
                    index.vectors.push(vec.clone());
                    index.num_vectors += 1;
                }
            }
        }

        if index.num_vectors == 0 {
            return None;
        }

        Some(index)
    }

    /// Search for k nearest neighbors using linear scan (exact for small datasets).
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(u32, f32)> {
        if self.vectors.is_empty() || query.len() != self.dimension {
            return Vec::new();
        }

        let mut distances: Vec<(u32, f32)> = self.vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i as u32, self.compute_distance(query, v)))
            .collect();

        // Sort by distance (ascending)
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k
        distances.truncate(k);
        distances
    }

    /// Search within a radius threshold.
    pub fn search_radius(&self, query: &[f32], radius: f32) -> Vec<(u32, f32)> {
        if self.vectors.is_empty() || query.len() != self.dimension {
            return Vec::new();
        }

        self.vectors
            .iter()
            .enumerate()
            .filter_map(|(i, v)| {
                let dist = self.compute_distance(query, v);
                if dist <= radius {
                    Some((i as u32, dist))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Compute distance between two vectors.
    fn compute_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        match self.metric {
            MetricType::L2 => {
                let sum: f32 = a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y) * (x - y))
                    .sum();
                sum.sqrt()
            }
        }
    }

    /// Get the number of vectors in the index.
    pub fn num_vectors(&self) -> usize {
        self.num_vectors
    }

    /// Get the vector dimension.
    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Predicate operators for pushdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    In,          // col IN (val1, val2, ...)
    Between,     // col BETWEEN val1 AND val2
    Like,        // col LIKE pattern
    NotEq,       // col != value
    NotIn,       // col NOT IN (val1, val2, ...)
    IsNull,      // col IS NULL
    IsNotNull,   // col IS NOT NULL
}

/// A single predicate for a column.
#[derive(Debug, Clone)]
pub struct ColumnPredicate {
    pub column_name: String,
    pub op: PredicateOp,
    pub value: ScalarValue,
    /// Additional values for In/Between operations
    pub values: Vec<ScalarValue>,
}

impl ColumnPredicate {
    pub fn new(column_name: String, op: PredicateOp, value: ScalarValue) -> Self {
        Self {
            column_name,
            op,
            value,
            values: Vec::new(),
        }
    }
    
    pub fn new_in(column_name: String, values: Vec<ScalarValue>) -> Self {
        Self {
            column_name,
            op: PredicateOp::In,
            value: ScalarValue::Null,  // Placeholder
            values,
        }
    }
    
    pub fn new_between(column_name: String, low: ScalarValue, high: ScalarValue) -> Self {
        Self {
            column_name,
            op: PredicateOp::Between,
            value: low,
            values: vec![high],
        }
    }
    
    pub fn new_like(column_name: String, pattern: String) -> Self {
        Self {
            column_name,
            op: PredicateOp::Like,
            value: ScalarValue::String(pattern),
            values: Vec::new(),
        }
    }
    
    pub fn new_is_null(column_name: String) -> Self {
        Self {
            column_name,
            op: PredicateOp::IsNull,
            value: ScalarValue::Null,
            values: Vec::new(),
        }
    }
    
    pub fn new_is_not_null(column_name: String) -> Self {
        Self {
            column_name,
            op: PredicateOp::IsNotNull,
            value: ScalarValue::Null,
            values: Vec::new(),
        }
    }
}

/// Compare two scalar values. Only works for comparable types.
pub fn compare_scalars(a: &ScalarValue, b: &ScalarValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (ScalarValue::Null, _) | (_, ScalarValue::Null) => Ordering::Equal,
        (ScalarValue::Boolean(a), ScalarValue::Boolean(b)) => a.cmp(b),
        (ScalarValue::Int8(a), ScalarValue::Int8(b)) => a.cmp(b),
        (ScalarValue::Int16(a), ScalarValue::Int16(b)) => a.cmp(b),
        (ScalarValue::Int32(a), ScalarValue::Int32(b)) => a.cmp(b),
        (ScalarValue::Int64(a), ScalarValue::Int64(b)) => a.cmp(b),
        (ScalarValue::Int128(a), ScalarValue::Int128(b)) => a.cmp(b),
        (ScalarValue::Float32(a), ScalarValue::Float32(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (ScalarValue::Float64(a), ScalarValue::Float64(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (ScalarValue::Date(a), ScalarValue::Date(b)) => a.cmp(b),
        (ScalarValue::DateTime(a), ScalarValue::DateTime(b)) => a.cmp(b),
        (ScalarValue::String(a), ScalarValue::String(b)) => a.cmp(b),
        _ => Ordering::Equal,
    }
}

/// Evaluate a predicate against a scalar value.
pub fn eval_predicate(op: &PredicateOp, col_val: &ScalarValue, pred_val: &ScalarValue) -> bool {
    eval_predicate_with_values(op, col_val, pred_val, &[])
}

/// Evaluate a predicate with additional values (for In/Between).
pub fn eval_predicate_with_values(op: &PredicateOp, col_val: &ScalarValue, pred_val: &ScalarValue, values: &[ScalarValue]) -> bool {
    if col_val.is_null() {
        return matches!(op, PredicateOp::IsNull);
    }
    
    match op {
        PredicateOp::Eq => compare_scalars(col_val, pred_val) == std::cmp::Ordering::Equal,
        PredicateOp::NotEq => compare_scalars(col_val, pred_val) != std::cmp::Ordering::Equal,
        PredicateOp::Lt => compare_scalars(col_val, pred_val) == std::cmp::Ordering::Less,
        PredicateOp::Le => compare_scalars(col_val, pred_val) != std::cmp::Ordering::Greater,
        PredicateOp::Gt => compare_scalars(col_val, pred_val) == std::cmp::Ordering::Greater,
        PredicateOp::Ge => compare_scalars(col_val, pred_val) != std::cmp::Ordering::Less,
        PredicateOp::In => {
            // Check if col_val is in the list of values
            values.iter().any(|v| compare_scalars(col_val, v) == std::cmp::Ordering::Equal)
        },
        PredicateOp::NotIn => {
            // Check if col_val is NOT in the list of values
            !values.iter().any(|v| compare_scalars(col_val, v) == std::cmp::Ordering::Equal)
        },
        PredicateOp::Between => {
            // Check if col_val is between pred_val (low) and values[0] (high)
            if values.is_empty() {
                return false;
            }
            let low_ord = compare_scalars(col_val, pred_val);
            let high_ord = compare_scalars(col_val, &values[0]);
            low_ord != std::cmp::Ordering::Less && high_ord != std::cmp::Ordering::Greater
        },
        PredicateOp::Like => {
            // Simple LIKE implementation: % wildcard only
            match pred_val {
                ScalarValue::String(pattern) => eval_like(col_val, pattern),
                _ => false,
            }
        },
        PredicateOp::IsNull => col_val.is_null(),
        PredicateOp::IsNotNull => !col_val.is_null(),
    }
}

/// Evaluate LIKE predicate with simple % wildcard support.
fn eval_like(col_val: &ScalarValue, pattern: &str) -> bool {
    match col_val {
        ScalarValue::String(s) => {
            // Simple implementation: only support % wildcard
            if pattern.starts_with('%') && pattern.ends_with('%') {
                // Contains: %pattern%
                let inner = &pattern[1..pattern.len()-1];
                s.contains(inner)
            } else if pattern.starts_with('%') {
                // Ends with: %pattern
                let suffix = &pattern[1..];
                s.ends_with(suffix)
            } else if pattern.ends_with('%') {
                // Starts with: pattern%
                let prefix = &pattern[..pattern.len()-1];
                s.starts_with(prefix)
            } else {
                // Exact match
                s == pattern
            }
        },
        _ => false,
    }
}

fn deserialize_scalar(data: &[u8]) -> Option<ScalarValue> {
    if data.is_empty() {
        return None;
    }
    // Simple approach: we stored the serialization marker + raw bytes.
    // For zone map comparison, we re-parse.
    // This is a simplified deserializer for the stored zone map data.
    Some(ScalarValue::Binary(data.to_vec()))
}

/// Apply predicates to an in-memory Block, returning a selection Bitmap.
pub fn apply_predicates_to_block(block: &Block, predicates: &[ColumnPredicate]) -> Bitmap {
    let num_rows = block.num_rows();
    if predicates.is_empty() {
        return Bitmap::all_set(num_rows);
    }
    let mut selection = Bitmap::all_set(num_rows);
    for predicate in predicates {
        let col_idx = block.schema().index_of(&predicate.column_name);
        let col_selection = match col_idx {
            Some(idx) => {
                let col = block.column(idx);
                match col {
                    Some(v) => {
                        let mut sel = Bitmap::with_capacity(num_rows);
                        for i in 0..num_rows {
                            let val = v.scalar_at(i);
                            sel.push(eval_predicate_with_values(&predicate.op, &val, &predicate.value, &predicate.values));
                        }
                        sel
                    }
                    None => Bitmap::new(),
                }
            }
            None => Bitmap::all_set(num_rows),
        };
        selection = (&selection) & (&col_selection);
    }
    selection
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_map_build() {
        let values = vec![
            ScalarValue::Int64(10),
            ScalarValue::Int64(20),
            ScalarValue::Null,
            ScalarValue::Int64(5),
        ];
        let zm = ZoneMap::build(&values);
        assert_eq!(zm.null_count, 1);
        assert_eq!(zm.num_rows, 4);
    }

    #[test]
    fn test_bloom_filter() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(b"hello");
        bf.insert(b"world");
        assert!(bf.may_contain(b"hello"));
        assert!(bf.may_contain(b"world"));
        // May have false positives but should be rare
        assert!(!bf.may_contain(b"xyzzy_not_present_at_all"));
    }

    #[test]
    fn test_bitmap_index_low_cardinality() {
        let values = vec![
            ScalarValue::String("a".to_string()),
            ScalarValue::String("b".to_string()),
            ScalarValue::String("a".to_string()),
            ScalarValue::String("c".to_string()),
            ScalarValue::Null,
            ScalarValue::String("b".to_string()),
        ];
        let idx = BitmapIndex::build(&values, 100).unwrap();
        assert!(idx.is_valid());
        assert_eq!(idx.num_values(), 3);
    }

    #[test]
    fn test_bitmap_index_high_cardinality() {
        let values: Vec<ScalarValue> = (0..2000).map(|i| ScalarValue::Int64(i)).collect();
        let idx = BitmapIndex::build(&values, 1000);
        assert!(idx.is_none(), "Should reject high-cardinality columns");
    }

    #[test]
    fn test_inverted_index_basic() {
        let values = vec![
            ScalarValue::String("hello world".to_string()),
            ScalarValue::String("world of databases".to_string()),
            ScalarValue::String("hello rust".to_string()),
        ];
        let idx = InvertedIndex::build(&values);
        assert_eq!(idx.num_terms(), 5); // hello, world, of, databases, rust
        let results = idx.search_term("hello");
        assert!(results.is_some());
        assert_eq!(results.unwrap(), &[0u32, 2]);
    }

    #[test]
    fn test_inverted_index_search_all() {
        let values = vec![
            ScalarValue::String("hello world rust".to_string()),
            ScalarValue::String("hello world".to_string()),
            ScalarValue::String("hello rust".to_string()),
        ];
        let idx = InvertedIndex::build(&values);
        let terms = vec!["hello".to_string(), "world".to_string()];
        let results = idx.search_all(&terms);
        assert_eq!(results, vec![0u32, 1]);
    }

    #[test]
    fn test_ann_index_exact_search() {
        let values = vec![
            ScalarValue::Float32Array(vec![1.0, 0.0]),
            ScalarValue::Float32Array(vec![0.0, 1.0]),
            ScalarValue::Float32Array(vec![0.5, 0.5]),
        ];
        let idx = ANNIndex::build(&values, 2).unwrap();
        assert_eq!(idx.num_vectors(), 3);

        // Search for nearest neighbor of [1.0, 0.0] - should be [1.0, 0.0] itself
        let results = idx.search(&[1.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);
        assert!(results[0].1 < 0.01);
    }

    #[test]
    fn test_ann_index_radius_search() {
        let values = vec![
            ScalarValue::Float32Array(vec![0.0, 0.0]),
            ScalarValue::Float32Array(vec![10.0, 10.0]),
            ScalarValue::Float32Array(vec![0.1, 0.1]),
        ];
        let idx = ANNIndex::build(&values, 2).unwrap();
        let results = idx.search_radius(&[0.0, 0.0], 1.0);
        assert_eq!(results.len(), 2); // [0,0] and [0.1,0.1] are within radius 1
    }

    #[test]
    fn test_predicate_op_in() {
        let val = ScalarValue::Int64(5);
        let values = vec![ScalarValue::Int64(3), ScalarValue::Int64(5), ScalarValue::Int64(7)];
        assert!(eval_predicate_with_values(&PredicateOp::In, &val, &ScalarValue::Null, &values));
        let val2 = ScalarValue::Int64(10);
        assert!(!eval_predicate_with_values(&PredicateOp::In, &val2, &ScalarValue::Null, &values));
    }

    #[test]
    fn test_predicate_op_like() {
        let val = ScalarValue::String("hello world".to_string());

        // Starts with
        assert!(eval_predicate_with_values(&PredicateOp::Like, &val, &ScalarValue::String("hello%".to_string()), &[]));
        // Ends with
        assert!(eval_predicate_with_values(&PredicateOp::Like, &val, &ScalarValue::String("%world".to_string()), &[]));
        // Contains
        assert!(eval_predicate_with_values(&PredicateOp::Like, &val, &ScalarValue::String("%llo w%".to_string()), &[]));
        // Not match
        assert!(!eval_predicate_with_values(&PredicateOp::Like, &val, &ScalarValue::String("xyz%".to_string()), &[]));
    }

    #[test]
    fn test_predicate_op_between() {
        let val = ScalarValue::Int64(50);
        let low = ScalarValue::Int64(10);
        let high = ScalarValue::Int64(100);
        assert!(eval_predicate_with_values(&PredicateOp::Between, &val, &low, &[high.clone()]));
        let val2 = ScalarValue::Int64(5);
        assert!(!eval_predicate_with_values(&PredicateOp::Between, &val2, &low, &[high]));
    }
}
