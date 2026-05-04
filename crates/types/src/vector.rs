use crate::{Bitmap, DataType, ScalarValue};
use std::fmt;

#[derive(Clone)]
pub enum Vector {
    Boolean(BooleanVector),
    Int8(Int8Vector),
    Int16(Int16Vector),
    Int32(Int32Vector),
    Int64(Int64Vector),
    Int128(Int128Vector),
    Float32(Float32Vector),
    Float64(Float64Vector),
    String(StringVector),
    Date(DateVector),
    DateTime(DateTimeVector),
    Null(NullVector),
}

macro_rules! impl_typed_vector {
    ($name:ident, $variant:ident, $prim:ty, $scalar_variant:ident, $zero:expr) => {
        #[derive(Clone)]
        pub struct $name {
            data: Vec<$prim>,
            validity: Bitmap,
        }

        impl $name {
            pub fn new() -> Self {
                Self { data: Vec::new(), validity: Bitmap::new() }
            }

            pub fn from_vec(data: Vec<$prim>) -> Self {
                let len = data.len();
                Self { data, validity: Bitmap::all_set(len) }
            }

            pub fn from_nullable_vec(data: Vec<Option<$prim>>) -> Self {
                let len = data.len();
                let mut validity = Bitmap::with_capacity(len);
                let values: Vec<$prim> = data.into_iter().map(|v| {
                    let is_some = v.is_some();
                    validity.push(is_some);
                    v.unwrap_or_else(|| $zero)
                }).collect();
                Self { data: values, validity }
            }

            pub fn push(&mut self, val: Option<$prim>) {
                match val {
                    Some(v) => {
                        self.data.push(v);
                        self.validity.push(true);
                    }
                    None => {
                        self.data.push($zero);
                        self.validity.push(false);
                    }
                }
            }

            pub fn get(&self, idx: usize) -> Option<$prim> {
                if self.validity.is_valid(idx) {
                    Some(self.data[idx])
                } else {
                    None
                }
            }

            pub fn get_checked(&self, idx: usize) -> $prim {
                self.data[idx]
            }

            pub fn data(&self) -> &[$prim] {
                &self.data
            }

            pub fn mut_data(&mut self) -> &mut Vec<$prim> {
                &mut self.data
            }

            pub fn validity(&self) -> &Bitmap {
                &self.validity
            }

            pub fn len(&self) -> usize {
                self.data.len()
            }

            pub fn is_empty(&self) -> bool {
                self.data.is_empty()
            }

            pub fn null_count(&self) -> usize {
                self.validity.null_count()
            }

            pub fn append(&mut self, other: &Self) {
                for i in 0..other.len() {
                    self.push(other.get(i));
                }
            }

            pub fn slice(&self, start: usize, len: usize) -> Self {
                let end = (start + len).min(self.data.len());
                let data = self.data[start..end].to_vec();
                let mut validity = Bitmap::with_capacity(len);
                for i in start..end {
                    validity.push(self.validity.is_valid(i));
                }
                Self { data, validity }
            }

            pub fn filter(&self, selection: &Bitmap) -> Self {
                // Use set_count for preallocation (faster than iterating)
                let len = selection.set_count();
                let mut data = Vec::with_capacity(len);
                let mut validity = Bitmap::with_capacity(len);

                // Optimized: use iter_set_bits for fast bitmap traversal
                for idx in selection.iter_set_bits() {
                    data.push(self.data[idx]);
                    validity.push(self.validity.is_valid(idx));
                }
                Self { data, validity }
            }
        }
    };
}

impl_typed_vector!(BooleanVector, Boolean, bool, Boolean, false);
impl_typed_vector!(Int8Vector, Int8, i8, Int8, 0);
impl_typed_vector!(Int16Vector, Int16, i16, Int16, 0);
impl_typed_vector!(Int32Vector, Int32, i32, Int32, 0);
impl_typed_vector!(Int64Vector, Int64, i64, Int64, 0);
impl_typed_vector!(Int128Vector, Int128, i128, Int128, 0);
impl_typed_vector!(Float32Vector, Float32, f32, Float32, 0.0);
impl_typed_vector!(Float64Vector, Float64, f64, Float64, 0.0);
impl_typed_vector!(DateVector, Date, i32, Date, 0);
impl_typed_vector!(DateTimeVector, DateTime, i64, DateTime, 0);

#[derive(Clone)]
pub struct StringVector {
    offsets: Vec<u32>,
    data: Vec<u8>,
    validity: Bitmap,
}

impl StringVector {
    pub fn new() -> Self {
        Self { offsets: vec![0], data: Vec::new(), validity: Bitmap::new() }
    }

    pub fn from_vec(vals: Vec<&str>) -> Self {
        let mut offsets = vec![0u32];
        let mut data = Vec::new();
        let mut validity = Bitmap::with_capacity(vals.len());
        for s in &vals {
            data.extend_from_slice(s.as_bytes());
            offsets.push(data.len() as u32);
            validity.push(true);
        }
        Self { offsets, data, validity }
    }

    pub fn from_option_vec(vals: Vec<Option<String>>) -> Self {
        let mut offsets = vec![0u32];
        let mut data = Vec::new();
        let mut validity = Bitmap::with_capacity(vals.len());
        for s in &vals {
            match s {
                Some(s) => {
                    data.extend_from_slice(s.as_bytes());
                    offsets.push(data.len() as u32);
                    validity.push(true);
                }
                None => {
                    offsets.push(data.len() as u32);
                    validity.push(false);
                }
            }
        }
        Self { offsets, data, validity }
    }

    pub fn push(&mut self, val: Option<&str>) {
        match val {
            Some(s) => {
                self.data.extend_from_slice(s.as_bytes());
                self.offsets.push(self.data.len() as u32);
                self.validity.push(true);
            }
            None => {
                self.offsets.push(self.data.len() as u32);
                self.validity.push(false);
            }
        }
    }

    pub fn get(&self, idx: usize) -> Option<&str> {
        if self.validity.is_valid(idx) && idx + 1 < self.offsets.len() {
            let start = self.offsets[idx] as usize;
            let end = self.offsets[idx + 1] as usize;
            Some(std::str::from_utf8(&self.data[start..end]).unwrap_or(""))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn validity(&self) -> &Bitmap {
        &self.validity
    }

    pub fn null_count(&self) -> usize {
        self.validity.null_count()
    }

    pub fn filter(&self, selection: &Bitmap) -> Self {
        let mut offsets = vec![0u32];
        let mut data = Vec::with_capacity(selection.len() * 16); // est 16 bytes avg
        let mut validity = Bitmap::with_capacity(selection.len());

        // Optimized: use iter_set_bits for fast bitmap traversal
        for idx in selection.iter_set_bits() {
            if let Some(s) = self.get(idx) {
                data.extend_from_slice(s.as_bytes());
                offsets.push(data.len() as u32);
                validity.push(self.validity.is_valid(idx));
            } else {
                offsets.push(data.len() as u32);
                validity.push(false);
            }
        }
        Self { offsets, data, validity }
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        let end = (start + len).min(self.len());
        let mut offsets = vec![0u32];
        let mut data = Vec::new();
        let mut validity = Bitmap::with_capacity(len);
        for i in start..end {
            if let Some(s) = self.get(i) {
                data.extend_from_slice(s.as_bytes());
                offsets.push(data.len() as u32);
                validity.push(self.validity.is_valid(i));
            } else {
                offsets.push(data.len() as u32);
                validity.push(false);
            }
        }
        Self { offsets, data, validity }
    }
}

#[derive(Clone)]
pub struct NullVector {
    len: usize,
}

impl NullVector {
    pub fn new(len: usize) -> Self {
        Self { len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// Vector enum dispatch methods
impl Vector {
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Boolean(_) => DataType::Boolean,
            Self::Int8(_) => DataType::Int8,
            Self::Int16(_) => DataType::Int16,
            Self::Int32(_) => DataType::Int32,
            Self::Int64(_) => DataType::Int64,
            Self::Int128(_) => DataType::Int128,
            Self::Float32(_) => DataType::Float32,
            Self::Float64(_) => DataType::Float64,
            Self::String(_) => DataType::String,
            Self::Date(_) => DataType::Date,
            Self::DateTime(_) => DataType::DateTime,
            Self::Null(_) => DataType::Null,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Boolean(v) => v.len(),
            Self::Int8(v) => v.len(),
            Self::Int16(v) => v.len(),
            Self::Int32(v) => v.len(),
            Self::Int64(v) => v.len(),
            Self::Int128(v) => v.len(),
            Self::Float32(v) => v.len(),
            Self::Float64(v) => v.len(),
            Self::String(v) => v.len(),
            Self::Date(v) => v.len(),
            Self::DateTime(v) => v.len(),
            Self::Null(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn scalar_at(&self, idx: usize) -> ScalarValue {
        match self {
            Self::Boolean(v) => match v.get(idx) {
                Some(b) => ScalarValue::Boolean(b),
                None => ScalarValue::Null,
            },
            Self::Int8(v) => match v.get(idx) {
                Some(n) => ScalarValue::Int8(n),
                None => ScalarValue::Null,
            },
            Self::Int16(v) => match v.get(idx) {
                Some(n) => ScalarValue::Int16(n),
                None => ScalarValue::Null,
            },
            Self::Int32(v) => match v.get(idx) {
                Some(n) => ScalarValue::Int32(n),
                None => ScalarValue::Null,
            },
            Self::Int64(v) => match v.get(idx) {
                Some(n) => ScalarValue::Int64(n),
                None => ScalarValue::Null,
            },
            Self::Int128(v) => match v.get(idx) {
                Some(n) => ScalarValue::Int128(n),
                None => ScalarValue::Null,
            },
            Self::Float32(v) => match v.get(idx) {
                Some(n) => ScalarValue::Float32(n),
                None => ScalarValue::Null,
            },
            Self::Float64(v) => match v.get(idx) {
                Some(n) => ScalarValue::Float64(n),
                None => ScalarValue::Null,
            },
            Self::String(v) => match v.get(idx) {
                Some(s) => ScalarValue::String(s.to_string()),
                None => ScalarValue::Null,
            },
            Self::Date(v) => match v.get(idx) {
                Some(n) => ScalarValue::Date(n),
                None => ScalarValue::Null,
            },
            Self::DateTime(v) => match v.get(idx) {
                Some(n) => ScalarValue::DateTime(n),
                None => ScalarValue::Null,
            },
            Self::Null(_) => ScalarValue::Null,
        }
    }

    pub fn filter(&self, selection: &Bitmap) -> Self {
        match self {
            Self::Boolean(v) => Self::Boolean(v.filter(selection)),
            Self::Int8(v) => Self::Int8(v.filter(selection)),
            Self::Int16(v) => Self::Int16(v.filter(selection)),
            Self::Int32(v) => Self::Int32(v.filter(selection)),
            Self::Int64(v) => Self::Int64(v.filter(selection)),
            Self::Int128(v) => Self::Int128(v.filter(selection)),
            Self::Float32(v) => Self::Float32(v.filter(selection)),
            Self::Float64(v) => Self::Float64(v.filter(selection)),
            Self::String(v) => Self::String(v.filter(selection)),
            Self::Date(v) => Self::Date(v.filter(selection)),
            Self::DateTime(v) => Self::DateTime(v.filter(selection)),
            Self::Null(v) => Self::Null(NullVector::new(
                (0..v.len()).filter(|&i| selection.get(i)).count()
            )),
        }
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        match self {
            Self::Boolean(v) => Self::Boolean(v.slice(start, len)),
            Self::Int8(v) => Self::Int8(v.slice(start, len)),
            Self::Int16(v) => Self::Int16(v.slice(start, len)),
            Self::Int32(v) => Self::Int32(v.slice(start, len)),
            Self::Int64(v) => Self::Int64(v.slice(start, len)),
            Self::Int128(v) => Self::Int128(v.slice(start, len)),
            Self::Float32(v) => Self::Float32(v.slice(start, len)),
            Self::Float64(v) => Self::Float64(v.slice(start, len)),
            Self::String(v) => Self::String(v.slice(start, len)),
            Self::Date(v) => Self::Date(v.slice(start, len)),
            Self::DateTime(v) => Self::DateTime(v.slice(start, len)),
            Self::Null(v) => Self::Null(NullVector::new(len.min(v.len().saturating_sub(start)))),
        }
    }

    pub fn null_count(&self) -> usize {
        match self {
            Self::Boolean(v) => v.null_count(),
            Self::Int8(v) => v.null_count(),
            Self::Int16(v) => v.null_count(),
            Self::Int32(v) => v.null_count(),
            Self::Int64(v) => v.null_count(),
            Self::Int128(v) => v.null_count(),
            Self::Float32(v) => v.null_count(),
            Self::Float64(v) => v.null_count(),
            Self::String(v) => v.validity().null_count(),
            Self::Date(v) => v.null_count(),
            Self::DateTime(v) => v.null_count(),
            Self::Null(v) => v.len(),
        }
    }

    pub fn append_vector(&mut self, other: &Vector) {
        match (self, other) {
            (Self::Int64(a), Self::Int64(b)) => a.append(b),
            (Self::Int32(a), Self::Int32(b)) => a.append(b),
            (Self::Float64(a), Self::Float64(b)) => a.append(b),
            (Self::Float32(a), Self::Float32(b)) => a.append(b),
            (Self::Boolean(a), Self::Boolean(b)) => a.append(b),
            (Self::Int8(a), Self::Int8(b)) => a.append(b),
            (Self::Int16(a), Self::Int16(b)) => a.append(b),
            (Self::Int128(a), Self::Int128(b)) => a.append(b),
            (Self::Date(a), Self::Date(b)) => a.append(b),
            (Self::DateTime(a), Self::DateTime(b)) => a.append(b),
            _ => {}
        }
    }

    pub fn from_scalar(val: &ScalarValue, len: usize) -> Self {
        match val {
            ScalarValue::Boolean(b) => Self::Boolean(BooleanVector::from_vec(vec![*b; len])),
            ScalarValue::Int8(n) => Self::Int8(Int8Vector::from_vec(vec![*n; len])),
            ScalarValue::Int16(n) => Self::Int16(Int16Vector::from_vec(vec![*n; len])),
            ScalarValue::Int32(n) => Self::Int32(Int32Vector::from_vec(vec![*n; len])),
            ScalarValue::Int64(n) => Self::Int64(Int64Vector::from_vec(vec![*n; len])),
            ScalarValue::Int128(n) => Self::Int128(Int128Vector::from_vec(vec![*n; len])),
            ScalarValue::Float32(n) => Self::Float32(Float32Vector::from_vec(vec![*n; len])),
            ScalarValue::Float64(n) => Self::Float64(Float64Vector::from_vec(vec![*n; len])),
            ScalarValue::String(s) => {
                let v: Vec<&str> = (0..len).map(|_| s.as_str()).collect();
                Self::String(StringVector::from_vec(v))
            }
            _ => Self::Null(NullVector::new(len)),
        }
    }
}

impl fmt::Debug for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let preview_len = self.len().min(20);
        let vals: Vec<String> = (0..preview_len)
            .map(|i| {
                let s = self.scalar_at(i);
                format!("{:?}", s)
            })
            .collect();
        write!(f, "Vector[{}] {{ {} }}", self.data_type(), vals.join(", "))
    }
}
