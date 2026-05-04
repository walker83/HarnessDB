use crate::{Bitmap, DataType, ScalarValue};
use crate::scalar::JsonValue;
use std::fmt;
use std::sync::Arc;

pub trait TypedVector: Clone + Send + Sync {
    type Primitive;
    
    fn new() -> Self;
    fn from_vec(data: Vec<Self::Primitive>) -> Self;
    fn from_nullable_vec(data: Vec<Option<Self::Primitive>>) -> Self;
    fn push(&mut self, val: Option<Self::Primitive>);
    fn get(&self, idx: usize) -> Option<Self::Primitive>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn null_count(&self) -> usize;
    fn filter(&self, selection: &Bitmap) -> Self;
    fn slice(&self, offset: usize, len: usize) -> Self;
}

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
    Json(JsonVector),
    Null(NullVector),
    Float32Array(Float32ArrayVector),
}

macro_rules! impl_typed_vector {
    ($name:ident, $variant:ident, $prim:ty, $scalar_variant:ident, $zero:expr) => {
        #[derive(Clone)]
        pub struct $name {
            data: Vec<$prim>,
            validity: Bitmap,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
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
                let len = selection.set_count();
                let mut data = Vec::with_capacity(len);
                let mut validity = Bitmap::with_capacity(len);

                for idx in selection.iter_set_bits() {
                    data.push(self.data[idx]);
                    validity.push(self.validity.is_valid(idx));
                }
                Self { data, validity }
            }

            pub fn count_batch(&self) -> usize {
                self.validity.set_count()
            }

            pub fn min_batch(&self) -> Option<$prim> {
                if self.data.is_empty() {
                    return None;
                }
                let mut min: Option<$prim> = None;
                for (i, val) in self.data.iter().enumerate() {
                    if self.validity.is_valid(i) {
                        min = Some(min.map_or(*val, |m| if m <= *val { m } else { *val }));
                    }
                }
                min
            }

            pub fn max_batch(&self) -> Option<$prim> {
                if self.data.is_empty() {
                    return None;
                }
                let mut max: Option<$prim> = None;
                for (i, val) in self.data.iter().enumerate() {
                    if self.validity.is_valid(i) {
                        max = Some(max.map_or(*val, |m| if m >= *val { m } else { *val }));
                    }
                }
                max
            }
        }

        impl TypedVector for $name {
            type Primitive = $prim;

            fn new() -> Self {
                Self::new()
            }

            fn from_vec(data: Vec<$prim>) -> Self {
                Self::from_vec(data)
            }

            fn from_nullable_vec(data: Vec<Option<$prim>>) -> Self {
                Self::from_nullable_vec(data)
            }

            fn push(&mut self, val: Option<$prim>) {
                Self::push(self, val)
            }

            fn get(&self, idx: usize) -> Option<$prim> {
                Self::get(self, idx)
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn is_empty(&self) -> bool {
                Self::is_empty(self)
            }

            fn null_count(&self) -> usize {
                Self::null_count(self)
            }

            fn filter(&self, selection: &Bitmap) -> Self {
                Self::filter(self, selection)
            }

            fn slice(&self, offset: usize, len: usize) -> Self {
                Self::slice(self, offset, len)
            }
        }
    };
}

macro_rules! impl_numeric_vector {
    ($name:ident, $variant:ident, $prim:ty, $scalar_variant:ident, $zero:expr) => {
        impl_typed_vector!($name, $variant, $prim, $scalar_variant, $zero);

        impl $name {
            pub fn sum_batch(&self) -> Option<$prim> {
                if self.data.is_empty() {
                    return None;
                }
                let mut sum: $prim = $zero;
                let mut count = 0;
                for (i, val) in self.data.iter().enumerate() {
                    if self.validity.is_valid(i) {
                        sum += *val;
                        count += 1;
                    }
                }
                if count > 0 {
                    Some(sum)
                } else {
                    None
                }
            }
        }
    };
}

impl_typed_vector!(BooleanVector, Boolean, bool, Boolean, false);

impl BooleanVector {
    pub fn sum_batch(&self) -> Option<i64> {
        if self.data.is_empty() {
            return None;
        }
        let mut count = 0;
        for (i, val) in self.data.iter().enumerate() {
            if self.validity.is_valid(i) && *val {
                count += 1;
            }
        }
        Some(count)
    }
}

impl_numeric_vector!(Int8Vector, Int8, i8, Int8, 0);
impl_numeric_vector!(Int16Vector, Int16, i16, Int16, 0);
impl_numeric_vector!(Int32Vector, Int32, i32, Int32, 0);
impl_numeric_vector!(Int64Vector, Int64, i64, Int64, 0);
impl_numeric_vector!(Int128Vector, Int128, i128, Int128, 0);
impl_numeric_vector!(Float32Vector, Float32, f32, Float32, 0.0);
impl_numeric_vector!(Float64Vector, Float64, f64, Float64, 0.0);
impl_numeric_vector!(DateVector, Date, i32, Date, 0);
impl_numeric_vector!(DateTimeVector, DateTime, i64, DateTime, 0);

#[derive(Clone)]
pub struct StringVector {
    offsets: Vec<u32>,
    data: Vec<u8>,
    validity: Bitmap,
}

impl Default for StringVector {
    fn default() -> Self {
        Self::new()
    }
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
        let num_selected = selection.set_count();
        let mut offsets = vec![0u32];
        let mut data = Vec::with_capacity(num_selected * 16);
        let mut validity = Bitmap::with_capacity(num_selected);

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

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn offsets(&self) -> &[u32] {
        &self.offsets
    }
}

impl TypedVector for StringVector {
    type Primitive = String;

    fn new() -> Self {
        Self { offsets: vec![0], data: Vec::new(), validity: Bitmap::new() }
    }

    fn from_vec(data: Vec<String>) -> Self {
        let mut offsets = vec![0u32];
        let mut str_data = Vec::new();
        let mut validity = Bitmap::with_capacity(data.len());
        for s in &data {
            str_data.extend_from_slice(s.as_bytes());
            offsets.push(str_data.len() as u32);
            validity.push(true);
        }
        Self { offsets, data: str_data, validity }
    }

    fn from_nullable_vec(data: Vec<Option<String>>) -> Self {
        let mut offsets = vec![0u32];
        let mut str_data = Vec::new();
        let mut validity = Bitmap::with_capacity(data.len());
        for s in &data {
            match s {
                Some(s) => {
                    str_data.extend_from_slice(s.as_bytes());
                    offsets.push(str_data.len() as u32);
                    validity.push(true);
                }
                None => {
                    offsets.push(str_data.len() as u32);
                    validity.push(false);
                }
            }
        }
        Self { offsets, data: str_data, validity }
    }

    fn push(&mut self, val: Option<String>) {
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

    fn get(&self, idx: usize) -> Option<String> {
        if self.validity.is_valid(idx) && idx + 1 < self.offsets.len() {
            let start = self.offsets[idx] as usize;
            let end = self.offsets[idx + 1] as usize;
            Some(std::str::from_utf8(&self.data[start..end]).unwrap_or("").to_string())
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn null_count(&self) -> usize {
        self.validity.null_count()
    }

    fn filter(&self, selection: &Bitmap) -> Self {
        let num_selected = selection.set_count();
        let mut offsets = vec![0u32];
        let mut data = Vec::with_capacity(num_selected * 16);
        let mut validity = Bitmap::with_capacity(num_selected);

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

    fn slice(&self, start: usize, len: usize) -> Self {
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
pub struct StringViewVector {
    offsets: Vec<u32>,
    data_ref: Arc<Vec<u8>>,
    validity: Bitmap,
    indices: Option<Vec<usize>>,
    owned_data: Option<Vec<u8>>,
}

impl Default for StringViewVector {
    fn default() -> Self {
        Self::new()
    }
}

impl StringViewVector {
    pub fn new() -> Self {
        Self {
            offsets: vec![0],
            data_ref: Arc::new(Vec::new()),
            validity: Bitmap::new(),
            indices: None,
            owned_data: None,
        }
    }

    pub fn from_string_vector(vec: &StringVector) -> Self {
        Self {
            offsets: vec.offsets.clone(),
            data_ref: Arc::new(vec.data.clone()),
            validity: vec.validity.clone(),
            indices: None,
            owned_data: None,
        }
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
        Self {
            offsets,
            data_ref: Arc::new(data),
            validity,
            indices: None,
            owned_data: None,
        }
    }

    pub fn get(&self, idx: usize) -> Option<&str> {
        if !self.validity.is_valid(idx) {
            return None;
        }

        let actual_idx = if let Some(indices) = &self.indices {
            if idx >= indices.len() {
                return None;
            }
            indices[idx]
        } else {
            idx
        };

        if actual_idx + 1 >= self.offsets.len() {
            return None;
        }

        let start = self.offsets[actual_idx] as usize;
        let end = self.offsets[actual_idx + 1] as usize;
        let data = self.owned_data.as_ref().unwrap_or(&*self.data_ref);
        Some(std::str::from_utf8(&data[start..end]).unwrap_or(""))
    }

    pub fn len(&self) -> usize {
        if let Some(indices) = &self.indices {
            indices.len()
        } else {
            self.offsets.len().saturating_sub(1)
        }
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

    pub fn filter_zero_copy(&self, selection: &Bitmap) -> Self {
        let indices: Vec<usize> = selection.iter_set_bits().collect();
        let mut validity = Bitmap::with_capacity(indices.len());
        
        for &idx in &indices {
            validity.push(self.validity.is_valid(idx));
        }

        Self {
            offsets: self.offsets.clone(),
            data_ref: self.data_ref.clone(),
            validity,
            indices: Some(indices),
            owned_data: None,
        }
    }

    pub fn slice_zero_copy(&self, start: usize, len: usize) -> Self {
        let end = (start + len).min(self.len());
        
        let new_indices = if let Some(indices) = &self.indices {
            indices[start..end].to_vec()
        } else {
            (start..end).collect()
        };

        let mut validity = Bitmap::with_capacity(len);
        for &idx in &new_indices {
            validity.push(self.validity.is_valid(idx));
        }

        Self {
            offsets: self.offsets.clone(),
            data_ref: self.data_ref.clone(),
            validity,
            indices: Some(new_indices),
            owned_data: None,
        }
    }

    pub fn to_owned(&mut self) {
        if self.owned_data.is_none() {
            self.owned_data = Some((*self.data_ref).clone());
        }
    }

    pub fn into_owned(self) -> Self {
        if self.owned_data.is_none() {
            Self {
                offsets: self.offsets,
                data_ref: Arc::new(Vec::new()),
                validity: self.validity,
                indices: self.indices,
                owned_data: Some((*self.data_ref).clone()),
            }
        } else {
            self
        }
    }
}

#[derive(Clone)]
pub struct JsonVector {
    data: Vec<ScalarValue>,
    validity: Bitmap,
}

impl Default for JsonVector {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonVector {
    pub fn new() -> Self {
        Self { data: Vec::new(), validity: Bitmap::new() }
    }

    pub fn from_vec(vals: Vec<ScalarValue>) -> Self {
        let mut validity = Bitmap::with_capacity(vals.len());
        for v in &vals {
            validity.push(!matches!(v, ScalarValue::Null));
        }
        Self { data: vals, validity }
    }

    pub fn from_option_vec(vals: Vec<Option<ScalarValue>>) -> Self {
        let mut validity = Bitmap::with_capacity(vals.len());
        let data: Vec<ScalarValue> = vals.into_iter().map(|v| {
            let _is_valid = v.is_some();
            if let Some(val) = v {
                validity.push(true);
                val
            } else {
                validity.push(false);
                ScalarValue::Json(JsonValue::Null)
            }
        }).collect();
        Self { data, validity }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<ScalarValue> {
        if self.validity.is_valid(idx) {
            Some(self.data[idx].clone())
        } else {
            None
        }
    }

    pub fn validity(&self) -> &Bitmap {
        &self.validity
    }

    pub fn null_count(&self) -> usize {
        self.validity.null_count()
    }

    pub fn push(&mut self, val: Option<ScalarValue>) {
        match val {
            Some(v) => {
                self.data.push(v);
                self.validity.push(true);
            }
            None => {
                self.data.push(ScalarValue::Json(JsonValue::Null));
                self.validity.push(false);
            }
        }
    }

    pub fn count_batch(&self) -> usize {
        self.validity.set_count()
    }

    pub fn filter(&self, selection: &crate::Bitmap) -> Self {
        let mut data = Vec::new();
        let mut validity = Bitmap::new();
        for idx in selection.iter_set_bits() {
            if let Some(v) = self.get(idx) {
                data.push(v);
                validity.push(true);
            } else {
                data.push(ScalarValue::Json(JsonValue::Null));
                validity.push(false);
            }
        }
        Self { data, validity }
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        let end = (start + len).min(self.len());
        let data: Vec<ScalarValue> = (start..end).filter_map(|i| self.get(i)).collect();
        let mut validity = Bitmap::with_capacity(len);
        for i in start..end {
            validity.push(self.validity.is_valid(i));
        }
        Self { data, validity }
    }
}

impl TypedVector for JsonVector {
    type Primitive = ScalarValue;

    fn new() -> Self {
        Self { data: Vec::new(), validity: Bitmap::new() }
    }

    fn from_vec(data: Vec<ScalarValue>) -> Self {
        let mut validity = Bitmap::with_capacity(data.len());
        for v in &data {
            validity.push(!matches!(v, ScalarValue::Null));
        }
        Self { data, validity }
    }

    fn from_nullable_vec(data: Vec<Option<ScalarValue>>) -> Self {
        let mut validity = Bitmap::with_capacity(data.len());
        let values: Vec<ScalarValue> = data.into_iter().map(|v| {
            if let Some(val) = v {
                validity.push(true);
                val
            } else {
                validity.push(false);
                ScalarValue::Json(JsonValue::Null)
            }
        }).collect();
        Self { data: values, validity }
    }

    fn push(&mut self, val: Option<ScalarValue>) {
        match val {
            Some(v) => {
                self.data.push(v);
                self.validity.push(true);
            }
            None => {
                self.data.push(ScalarValue::Json(JsonValue::Null));
                self.validity.push(false);
            }
        }
    }

    fn get(&self, idx: usize) -> Option<ScalarValue> {
        if self.validity.is_valid(idx) {
            Some(self.data[idx].clone())
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn null_count(&self) -> usize {
        self.validity.null_count()
    }

    fn filter(&self, selection: &Bitmap) -> Self {
        let mut data = Vec::new();
        let mut validity = Bitmap::new();
        for idx in selection.iter_set_bits() {
            if let Some(v) = self.get(idx) {
                data.push(v);
                validity.push(true);
            } else {
                data.push(ScalarValue::Json(JsonValue::Null));
                validity.push(false);
            }
        }
        Self { data, validity }
    }

    fn slice(&self, start: usize, len: usize) -> Self {
        let end = (start + len).min(self.len());
        let data: Vec<ScalarValue> = (start..end).filter_map(|i| self.get(i)).collect();
        let mut validity = Bitmap::with_capacity(len);
        for i in start..end {
            validity.push(self.validity.is_valid(i));
        }
        Self { data, validity }
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

impl TypedVector for NullVector {
    type Primitive = ();

    fn new() -> Self {
        Self { len: 0 }
    }

    fn from_vec(data: Vec<Self::Primitive>) -> Self {
        Self { len: data.len() }
    }

    fn from_nullable_vec(data: Vec<Option<Self::Primitive>>) -> Self {
        Self { len: data.len() }
    }

    fn push(&mut self, _val: Option<Self::Primitive>) {
        self.len += 1;
    }

    fn get(&self, _idx: usize) -> Option<Self::Primitive> {
        None
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn null_count(&self) -> usize {
        self.len
    }

    fn filter(&self, selection: &Bitmap) -> Self {
        Self { len: selection.set_count() }
    }

    fn slice(&self, offset: usize, len: usize) -> Self {
        Self { len: len.min(self.len.saturating_sub(offset)) }
    }
}

/// Vector of f32 arrays for ANN Index support.
#[derive(Clone)]
pub struct Float32ArrayVector {
    data: Vec<Vec<f32>>,
    validity: Bitmap,
}

impl Float32ArrayVector {
    pub fn new() -> Self {
        Self { data: Vec::new(), validity: Bitmap::new() }
    }

    pub fn from_vec(data: Vec<Vec<f32>>) -> Self {
        let len = data.len();
        Self { data, validity: Bitmap::all_set(len) }
    }

    pub fn push(&mut self, val: Option<Vec<f32>>) {
        match val {
            Some(v) => {
                self.data.push(v);
                self.validity.push(true);
            }
            None => {
                self.data.push(Vec::new());
                self.validity.push(false);
            }
        }
    }

    pub fn get(&self, idx: usize) -> Option<&[f32]> {
        if self.validity.is_valid(idx) {
            Some(&self.data[idx])
        } else {
            None
        }
    }

    pub fn data(&self) -> &[Vec<f32>] {
        &self.data
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

    pub fn filter(&self, selection: &Bitmap) -> Self {
        let mut new_data = Vec::new();
        let mut new_validity = Bitmap::with_capacity(selection.set_count());
        for i in 0..self.len() {
            if self.validity.is_valid(i) && selection.is_valid(i) {
                new_data.push(self.data[i].clone());
                new_validity.push(true);
            } else if !self.validity.is_valid(i) && selection.is_valid(i) {
                new_data.push(Vec::new());
                new_validity.push(false);
            }
        }
        Self { data: new_data, validity: new_validity }
    }

    pub fn slice(&self, offset: usize, len: usize) -> Self {
        let end = (offset + len).min(self.len());
        if offset >= self.len() {
            return Self::new();
        }
        let data = self.data[offset..end].to_vec();
        Self::from_vec(data)
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
            Self::Json(_) => DataType::Json,
            Self::Null(_) => DataType::Null,
            Self::Float32Array(_) => DataType::Float32Vector(0),
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
            Self::Json(v) => v.len(),
            Self::Null(v) => v.len(),
            Self::Float32Array(v) => v.len(),
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
            Self::Json(v) => match v.get(idx) {
            Some(j) => j,
            None => ScalarValue::Null,
        },
            Self::Float32Array(v) => match v.get(idx) {
                Some(arr) => ScalarValue::Float32Array(arr.to_vec()),
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
            Self::Json(v) => Self::Json(v.filter(selection)),
            Self::Null(v) => Self::Null(NullVector::new(
                (0..v.len()).filter(|&i| selection.get(i)).count()
            )),
            Self::Float32Array(v) => Self::Float32Array(v.filter(selection)),
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
            Self::Json(v) => Self::Json(v.slice(start, len)),
            Self::Null(v) => Self::Null(NullVector::new(len.min(v.len().saturating_sub(start)))),
            Self::Float32Array(v) => Self::Float32Array(v.slice(start, len)),
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
            Self::Json(v) => v.null_count(),
            Self::Null(v) => v.len(),
            Self::Float32Array(v) => v.null_count(),
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
            ScalarValue::Json(j) => Self::Json(JsonVector::from_vec(vec![ScalarValue::Json(j.clone()); len])),
            _ => Self::Null(NullVector::new(len)),
        }
    }

    pub fn sum_batch(&self) -> Option<ScalarValue> {
        match self {
            Self::Boolean(v) => v.sum_batch().map(ScalarValue::Int64),
            Self::Int8(v) => v.sum_batch().map(ScalarValue::Int8),
            Self::Int16(v) => v.sum_batch().map(ScalarValue::Int16),
            Self::Int32(v) => v.sum_batch().map(ScalarValue::Int32),
            Self::Int64(v) => v.sum_batch().map(ScalarValue::Int64),
            Self::Int128(v) => v.sum_batch().map(ScalarValue::Int128),
            Self::Float32(v) => v.sum_batch().map(ScalarValue::Float32),
            Self::Float64(v) => v.sum_batch().map(ScalarValue::Float64),
            Self::Date(v) => v.sum_batch().map(ScalarValue::Date),
            Self::DateTime(v) => v.sum_batch().map(ScalarValue::DateTime),
            _ => None,
        }
    }

    pub fn count_batch(&self) -> usize {
        match self {
            Self::Boolean(v) => v.count_batch(),
            Self::Int8(v) => v.count_batch(),
            Self::Int16(v) => v.count_batch(),
            Self::Int32(v) => v.count_batch(),
            Self::Int64(v) => v.count_batch(),
            Self::Int128(v) => v.count_batch(),
            Self::Float32(v) => v.count_batch(),
            Self::Float64(v) => v.count_batch(),
            Self::String(v) => v.validity().set_count(),
            Self::Date(v) => v.count_batch(),
            Self::DateTime(v) => v.count_batch(),
            Self::Json(v) => v.count_batch(),
            Self::Null(v) => v.len(),
            Self::Float32Array(v) => v.validity().set_count(),
        }
    }

    pub fn min_batch(&self) -> Option<ScalarValue> {
        match self {
            Self::Int8(v) => v.min_batch().map(ScalarValue::Int8),
            Self::Int16(v) => v.min_batch().map(ScalarValue::Int16),
            Self::Int32(v) => v.min_batch().map(ScalarValue::Int32),
            Self::Int64(v) => v.min_batch().map(ScalarValue::Int64),
            Self::Int128(v) => v.min_batch().map(ScalarValue::Int128),
            Self::Float32(v) => v.min_batch().map(ScalarValue::Float32),
            Self::Float64(v) => v.min_batch().map(ScalarValue::Float64),
            Self::Date(v) => v.min_batch().map(ScalarValue::Date),
            Self::DateTime(v) => v.min_batch().map(ScalarValue::DateTime),
            Self::Boolean(v) => v.min_batch().map(ScalarValue::Boolean),
            Self::String(v) => {
                let mut min: Option<&str> = None;
                for i in 0..v.len() {
                    if v.validity().is_valid(i)
                        && let Some(s) = v.get(i) {
                            min = Some(min.map_or(s, |m| m.min(s)));
                        }
                }
                min.map(|s| ScalarValue::String(s.to_string()))
            },
            _ => None,
        }
    }

    pub fn max_batch(&self) -> Option<ScalarValue> {
        match self {
            Self::Int8(v) => v.max_batch().map(ScalarValue::Int8),
            Self::Int16(v) => v.max_batch().map(ScalarValue::Int16),
            Self::Int32(v) => v.max_batch().map(ScalarValue::Int32),
            Self::Int64(v) => v.max_batch().map(ScalarValue::Int64),
            Self::Int128(v) => v.max_batch().map(ScalarValue::Int128),
            Self::Float32(v) => v.max_batch().map(ScalarValue::Float32),
            Self::Float64(v) => v.max_batch().map(ScalarValue::Float64),
            Self::Date(v) => v.max_batch().map(ScalarValue::Date),
            Self::DateTime(v) => v.max_batch().map(ScalarValue::DateTime),
            Self::Boolean(v) => v.max_batch().map(ScalarValue::Boolean),
            Self::String(v) => {
                let mut max: Option<&str> = None;
                for i in 0..v.len() {
                    if v.validity().is_valid(i)
                        && let Some(s) = v.get(i) {
                            max = Some(max.map_or(s, |m| m.max(s)));
                        }
                }
                max.map(|s| ScalarValue::String(s.to_string()))
            },
            _ => None,
        }
    }

    pub fn avg_batch(&self) -> Option<ScalarValue> {
        match self {
            Self::Int8(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Int16(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Int32(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Int64(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Int128(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Float32(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s as f64 / count as f64))
                } else {
                    None
                }
            },
            Self::Float64(v) => {
                let count = v.count_batch();
                if count > 0 {
                    v.sum_batch().map(|s| ScalarValue::Float64(s / count as f64))
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn compare_at(&self, idx_a: usize, idx_b: usize) -> std::cmp::Ordering {
        match self {
            Self::Boolean(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Int8(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Int16(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Int32(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Int64(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Int128(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Float32(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Float64(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::String(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Date(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::DateTime(v) => {
                let a = v.get(idx_a);
                let b = v.get(idx_b);
                match (a, b) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    (Some(_), None) => std::cmp::Ordering::Greater,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            Self::Json(_) | Self::Null(_) | Self::Float32Array(_) => std::cmp::Ordering::Equal,
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
