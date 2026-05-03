use std::ops::{BitAnd, BitOr, BitXor, Not};

#[derive(Debug, Clone)]
pub struct Bitmap {
    data: Vec<u64>,
    len: usize,
}

impl Bitmap {
    pub fn new() -> Self {
        Self { data: Vec::new(), len: 0 }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let words = (capacity + 63) / 64;
        Self { data: vec![0; words], len: 0 }
    }

    pub fn from_bools(bools: &[bool]) -> Self {
        let mut bm = Self::with_capacity(bools.len());
        for &b in bools {
            bm.push(b);
        }
        bm
    }

    pub fn all_set(len: usize) -> Self {
        let words = (len + 63) / 64;
        let mut data = vec![u64::MAX; words];
        if len % 64 != 0 {
            if let Some(last) = data.last_mut() {
                *last &= (1u64 << (len % 64)) - 1;
            }
        }
        Self { data, len }
    }

    pub fn push(&mut self, val: bool) {
        let word_idx = self.len / 64;
        let bit_idx = self.len % 64;
        if word_idx >= self.data.len() {
            self.data.push(0);
        }
        if val {
            self.data[word_idx] |= 1u64 << bit_idx;
        }
        self.len += 1;
    }

    pub fn get(&self, idx: usize) -> bool {
        if idx >= self.len {
            return false;
        }
        let word_idx = idx / 64;
        let bit_idx = idx % 64;
        (self.data[word_idx] >> bit_idx) & 1 == 1
    }

    pub fn set(&mut self, idx: usize, val: bool) {
        let word_idx = idx / 64;
        let bit_idx = idx % 64;
        if val {
            self.data[word_idx] |= 1u64 << bit_idx;
        } else {
            self.data[word_idx] &= !(1u64 << bit_idx);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn null_count(&self) -> usize {
        self.len - self.set_count()
    }

    pub fn set_count(&self) -> usize {
        self.data.iter().map(|w| w.count_ones() as usize).sum::<usize>()
    }

    pub fn is_valid(&self, idx: usize) -> bool {
        self.get(idx)
    }
}

impl BitAnd for &Bitmap {
    type Output = Bitmap;
    fn bitand(self, rhs: &Bitmap) -> Bitmap {
        let len = self.len.min(rhs.len);
        let data: Vec<u64> = self.data.iter()
            .zip(rhs.data.iter())
            .map(|(a, b)| a & b)
            .collect();
        Bitmap { data, len }
    }
}

impl BitOr for &Bitmap {
    type Output = Bitmap;
    fn bitor(self, rhs: &Bitmap) -> Bitmap {
        let len = self.len.max(rhs.len);
        let mut data = self.data.clone();
        for (i, &v) in rhs.data.iter().enumerate() {
            if i < data.len() { data[i] |= v; } else { data.push(v); }
        }
        Bitmap { data, len }
    }
}

impl Not for &Bitmap {
    type Output = Bitmap;
    fn not(self) -> Bitmap {
        let data: Vec<u64> = self.data.iter().map(|w| !w).collect();
        Bitmap { data: data.clone(), len: self.len }
    }
}
