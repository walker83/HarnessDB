use types::{DataType, ScalarValue, Vector};
use std::collections::HashSet;

pub trait Accumulator: Send + Sync {
    fn update(&mut self, values: &[ScalarValue]);
    fn update_batch(&mut self, column: &Vector);
    fn get_value(&self) -> ScalarValue;
    fn reset(&mut self);
    fn clone_box(&self) -> Box<dyn Accumulator>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
    GroupConcat,
}

impl AggregateFunction {
    pub fn create_accumulator(&self, data_type: DataType) -> Box<dyn Accumulator> {
        match self {
            Self::Count => Box::new(CountAccumulator { count: 0 }),
            Self::CountDistinct => Box::new(CountDistinctAccumulator { values: HashSet::new(), data_type }),
            Self::Sum => match data_type {
                DataType::Float64 | DataType::Float32 => Box::new(FloatSumAccumulator { sum: 0.0 }),
                _ => Box::new(IntSumAccumulator { sum: 0 }),
            },
            Self::Avg => Box::new(AvgAccumulator { sum: 0.0, count: 0 }),
            Self::Min => Box::new(MinMaxAccumulator { value: None, is_min: true, data_type }),
            Self::Max => Box::new(MinMaxAccumulator { value: None, is_min: false, data_type }),
            Self::GroupConcat => Box::new(GroupConcatAccumulator { values: Vec::new(), separator: ",".into() }),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "count" => Some(Self::Count),
            "count_distinct" | "countdistinct" => Some(Self::CountDistinct),
            "sum" => Some(Self::Sum),
            "avg" | "average" => Some(Self::Avg),
            "min" | "minimum" => Some(Self::Min),
            "max" | "maximum" => Some(Self::Max),
            "group_concat" => Some(Self::GroupConcat),
            _ => None,
        }
    }
}

struct CountAccumulator { count: i64 }

impl Accumulator for CountAccumulator {
    fn update(&mut self, values: &[ScalarValue]) { self.count += values.iter().filter(|v| **v != ScalarValue::Null).count() as i64; }
    fn update_batch(&mut self, column: &Vector) { self.count += (column.len() - column.null_count()) as i64; }
    fn get_value(&self) -> ScalarValue { ScalarValue::Int64(self.count) }
    fn reset(&mut self) { self.count = 0; }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(CountAccumulator { count: self.count }) }
}

struct CountDistinctAccumulator { values: HashSet<String>, data_type: DataType }

impl Accumulator for CountDistinctAccumulator {
    fn update(&mut self, values: &[ScalarValue]) { for v in values { if *v != ScalarValue::Null { self.values.insert(format!("{:?}", v)); } } }
    fn update_batch(&mut self, column: &Vector) { for i in 0..column.len() { let v = column.scalar_at(i); if v != ScalarValue::Null { self.values.insert(format!("{:?}", v)); } } }
    fn get_value(&self) -> ScalarValue { ScalarValue::Int64(self.values.len() as i64) }
    fn reset(&mut self) { self.values.clear(); }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(CountDistinctAccumulator { values: self.values.clone(), data_type: self.data_type.clone() }) }
}

struct IntSumAccumulator { sum: i64 }

impl Accumulator for IntSumAccumulator {
    fn update(&mut self, values: &[ScalarValue]) {
        for v in values {
            match v { ScalarValue::Int64(n) => self.sum += n, ScalarValue::Int32(n) => self.sum += *n as i64, _ => {} }
        }
    }
    fn update_batch(&mut self, column: &Vector) {
        match column {
            Vector::Int64(v) => self.sum += v.data().iter().sum::<i64>(),
            Vector::Int32(v) => self.sum += v.data().iter().map(|&n| n as i64).sum::<i64>(),
            _ => {}
        }
    }
    fn get_value(&self) -> ScalarValue { ScalarValue::Int64(self.sum) }
    fn reset(&mut self) { self.sum = 0; }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(IntSumAccumulator { sum: self.sum }) }
}

struct FloatSumAccumulator { sum: f64 }

impl Accumulator for FloatSumAccumulator {
    fn update(&mut self, values: &[ScalarValue]) {
        for v in values {
            match v { ScalarValue::Float64(n) => self.sum += n, ScalarValue::Float32(n) => self.sum += *n as f64, _ => {} }
        }
    }
    fn update_batch(&mut self, column: &Vector) {
        match column {
            Vector::Float64(v) => self.sum += v.data().iter().sum::<f64>(),
            Vector::Float32(v) => self.sum += v.data().iter().map(|&n| n as f64).sum::<f64>(),
            _ => {}
        }
    }
    fn get_value(&self) -> ScalarValue { ScalarValue::Float64(self.sum) }
    fn reset(&mut self) { self.sum = 0.0; }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(FloatSumAccumulator { sum: self.sum }) }
}

struct AvgAccumulator { sum: f64, count: i64 }

impl Accumulator for AvgAccumulator {
    fn update(&mut self, values: &[ScalarValue]) {
        for v in values {
            match v {
                ScalarValue::Int64(n) => { self.sum += *n as f64; self.count += 1; }
                ScalarValue::Float64(n) => { self.sum += n; self.count += 1; }
                _ => {}
            }
        }
    }
    fn update_batch(&mut self, column: &Vector) {
        match column {
            Vector::Int64(v) => { self.sum += v.data().iter().map(|&n| n as f64).sum::<f64>(); self.count += v.data().len() as i64; }
            Vector::Float64(v) => { self.sum += v.data().iter().sum::<f64>(); self.count += v.data().len() as i64; }
            _ => {}
        }
    }
    fn get_value(&self) -> ScalarValue { if self.count == 0 { ScalarValue::Null } else { ScalarValue::Float64(self.sum / self.count as f64) } }
    fn reset(&mut self) { self.sum = 0.0; self.count = 0; }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(AvgAccumulator { sum: self.sum, count: self.count }) }
}

struct MinMaxAccumulator { value: Option<ScalarValue>, is_min: bool, data_type: DataType }

impl Accumulator for MinMaxAccumulator {
    fn update(&mut self, values: &[ScalarValue]) {
        for v in values {
            if *v == ScalarValue::Null { continue; }
            match &self.value {
                None => self.value = Some(v.clone()),
                Some(current) => {
                    let should_update = if self.is_min { compare_scalar(v, current) < 0 } else { compare_scalar(v, current) > 0 };
                    if should_update { self.value = Some(v.clone()); }
                }
            }
        }
    }
    fn update_batch(&mut self, column: &Vector) {
        for i in 0..column.len() {
            let v = column.scalar_at(i);
            self.update(&[v]);
        }
    }
    fn get_value(&self) -> ScalarValue { self.value.clone().unwrap_or(ScalarValue::Null) }
    fn reset(&mut self) { self.value = None; }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(MinMaxAccumulator { value: self.value.clone(), is_min: self.is_min, data_type: self.data_type.clone() }) }
}

struct GroupConcatAccumulator { values: Vec<String>, separator: String }

impl Accumulator for GroupConcatAccumulator {
    fn update(&mut self, values: &[ScalarValue]) {
        for v in values {
            match v {
                ScalarValue::String(s) => self.values.push(s.clone()),
                other => self.values.push(format!("{:?}", other)),
            }
        }
    }
    fn update_batch(&mut self, column: &Vector) {
        for i in 0..column.len() { self.update(&[column.scalar_at(i)]); }
    }
    fn get_value(&self) -> ScalarValue { ScalarValue::String(self.values.join(&self.separator)) }
    fn reset(&mut self) { self.values.clear(); }
    fn clone_box(&self) -> Box<dyn Accumulator> { Box::new(GroupConcatAccumulator { values: self.values.clone(), separator: self.separator.clone() }) }
}

fn compare_scalar(a: &ScalarValue, b: &ScalarValue) -> i32 {
    match (a, b) {
        (ScalarValue::Int64(a), ScalarValue::Int64(b)) => a.cmp(b) as i32,
        (ScalarValue::Float64(a), ScalarValue::Float64(b)) => a.partial_cmp(b).map(|o| o as i32).unwrap_or(0),
        (ScalarValue::String(a), ScalarValue::String(b)) => a.cmp(b) as i32,
        _ => 0,
    }
}
