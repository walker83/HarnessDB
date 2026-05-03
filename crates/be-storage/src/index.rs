use serde::{Deserialize, Serialize};
use types::{Bitmap, Block, ScalarValue};

/// Zone map index: tracks min/max/null_count per page for one column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneMap {
    pub min_value: Option<Vec<u8>>,
    pub max_value: Option<Vec<u8>>,
    pub null_count: u64,
    pub num_rows: u64,
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
        let words = (bits + 63) / 64;
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
        k.max(1).min(20)
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

/// Predicate operators for pushdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

/// A single predicate for a column.
#[derive(Debug, Clone)]
pub struct ColumnPredicate {
    pub column_name: String,
    pub op: PredicateOp,
    pub value: ScalarValue,
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
    if col_val.is_null() {
        return false;
    }
    match compare_scalars(col_val, pred_val) {
        std::cmp::Ordering::Less => matches!(op, PredicateOp::Lt | PredicateOp::Le),
        std::cmp::Ordering::Equal => matches!(op, PredicateOp::Eq | PredicateOp::Le | PredicateOp::Ge),
        std::cmp::Ordering::Greater => matches!(op, PredicateOp::Gt | PredicateOp::Ge),
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
                            sel.push(eval_predicate(&predicate.op, &val, &predicate.value));
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
}
