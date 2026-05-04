use serde::{Deserialize, Serialize};
use crate::ScalarValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeFilterType {
    Bloom,
    MinMax,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinMaxFilter {
    pub min_value: Option<ScalarValue>,
    pub max_value: Option<ScalarValue>,
}

impl MinMaxFilter {
    pub fn new() -> Self {
        Self {
            min_value: None,
            max_value: None,
        }
    }

    pub fn update(&mut self, value: &ScalarValue) {
        if value.is_null() {
            return;
        }
        match &self.min_value {
            None => {
                self.min_value = Some(value.clone());
                self.max_value = Some(value.clone());
            }
            Some(min) => {
                if compare_scalars(value, min) == std::cmp::Ordering::Less {
                    self.min_value = Some(value.clone());
                }
            }
        }
        match &self.max_value {
            None => {
                self.max_value = Some(value.clone());
            }
            Some(max) => {
                if compare_scalars(value, max) == std::cmp::Ordering::Greater {
                    self.max_value = Some(value.clone());
                }
            }
        }
    }

    pub fn may_contain(&self, value: &ScalarValue) -> bool {
        if value.is_null() {
            return false;
        }
        match (&self.min_value, &self.max_value) {
            (Some(min), Some(max)) => {
                compare_scalars(value, min) != std::cmp::Ordering::Less
                    && compare_scalars(value, max) != std::cmp::Ordering::Greater
            }
            _ => true,
        }
    }
}

impl Default for MinMaxFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InFilter {
    pub values: Vec<ScalarValue>,
}

impl InFilter {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, value: ScalarValue) {
        if !value.is_null() && !self.values.contains(&value) {
            self.values.push(value);
        }
    }

    pub fn may_contain(&self, value: &ScalarValue) -> bool {
        if value.is_null() {
            return false;
        }
        self.values.iter().any(|v| compare_scalars(v, value) == std::cmp::Ordering::Equal)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Default for InFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeFilter {
    pub id: RuntimeFilterId,
    pub filter_type: RuntimeFilterType,
    pub build_keys: Vec<String>,
    pub probe_keys: Vec<String>,
}

impl RuntimeFilter {
    pub fn new(id: RuntimeFilterId, filter_type: RuntimeFilterType, build_keys: Vec<String>, probe_keys: Vec<String>) -> Self {
        Self {
            id,
            filter_type,
            build_keys,
            probe_keys,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeFilterId(pub u64);

impl RuntimeFilterId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for RuntimeFilterId {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_filter() {
        let mut filter = MinMaxFilter::new();
        filter.update(&ScalarValue::Int64(10));
        filter.update(&ScalarValue::Int64(5));
        filter.update(&ScalarValue::Int64(20));

        assert!(filter.may_contain(&ScalarValue::Int64(10)));
        assert!(filter.may_contain(&ScalarValue::Int64(5)));
        assert!(filter.may_contain(&ScalarValue::Int64(20)));
        assert!(!filter.may_contain(&ScalarValue::Int64(1)));
        assert!(!filter.may_contain(&ScalarValue::Int64(25)));
    }

    #[test]
    fn test_in_filter() {
        let mut filter = InFilter::new();
        filter.insert(ScalarValue::Int64(10));
        filter.insert(ScalarValue::Int64(20));
        filter.insert(ScalarValue::Int64(10));

        assert!(filter.may_contain(&ScalarValue::Int64(10)));
        assert!(filter.may_contain(&ScalarValue::Int64(20)));
        assert!(!filter.may_contain(&ScalarValue::Int64(5)));
        assert_eq!(filter.len(), 2);
    }
}