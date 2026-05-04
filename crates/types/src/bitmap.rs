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
        let mut count = 0usize;
        for chunk in self.data.chunks(8) {
            for word in chunk {
                count += word.count_ones() as usize;
            }
        }
        count
    }

    pub fn is_valid(&self, idx: usize) -> bool {
        self.get(idx)
    }

    /// Get raw word at word index for batch operations
    pub fn word_at(&self, word_idx: usize) -> u64 {
        self.data.get(word_idx).copied().unwrap_or(0)
    }

    /// Get number of words in the bitmap
    pub fn words(&self) -> usize {
        self.data.len()
    }

    /// Clear all bits
    pub fn clear(&mut self) {
        for word in &mut self.data {
            *word = 0;
        }
        self.len = 0;
    }

    /// Iterate over set bits efficiently using trailing_zeros
    pub fn iter_set_bits(&self) -> SetBitIter {
        let first_word = self.data.first().copied().unwrap_or(0);
        // Apply mask to first word to ignore bits beyond len
        let masked_word = if self.len < 64 {
            first_word & ((1u64 << self.len) - 1)
        } else {
            first_word
        };
        SetBitIter {
            data: &self.data,
            word_idx: 0,
            word: masked_word,
            len: self.len,
            consumed: 0,
        }
    }

    /// Batch AND operation - optimized with early termination
    pub fn and_inplace(&mut self, other: &Bitmap) {
        let len = self.len.min(other.len);
        self.len = len;
        for i in 0..self.data.len().min(other.data.len()) {
            self.data[i] &= other.data[i];
        }
        for i in self.data.len()..other.data.len() {
            if i < self.data.len() {
                self.data[i] = 0;
            }
        }
    }

    /// Batch OR operation - optimized
    pub fn or_inplace(&mut self, other: &Bitmap) {
        let len = self.len.max(other.len);
        if other.data.len() > self.data.len() {
            self.data.resize(other.data.len(), 0);
        }
        for i in 0..other.data.len() {
            self.data[i] |= other.data[i];
        }
        self.len = len;
    }

    /// Batch NOT operation - optimized
    pub fn not_inplace(&mut self) {
        for word in &mut self.data {
            *word = !*word;
        }
        if self.len % 64 != 0 && !self.data.is_empty() {
            if let Some(last) = self.data.last_mut() {
                *last &= (1u64 << (self.len % 64)) - 1;
            }
        }
    }

    /// In-place AND with another bitmap
    pub fn and_with(&mut self, other: &Bitmap) {
        self.and_inplace(other);
    }

    /// In-place OR with another bitmap
    pub fn or_with(&mut self, other: &Bitmap) {
        self.or_inplace(other);
    }
}

pub struct SetBitIter<'a> {
    data: &'a Vec<u64>,
    word_idx: usize,
    word: u64,
    len: usize,
    consumed: usize,
}

impl<'a> Iterator for SetBitIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.consumed >= self.len {
                return None;
            }
            if self.word != 0 {
                let tz = self.word.trailing_zeros() as usize;
                let global_pos = self.word_idx * 64 + tz;
                self.word &= !(1u64 << tz);
                self.consumed += 1;
                return Some(global_pos);
            } else {
                // Advance to next word
                self.word_idx += 1;
                if self.word_idx >= self.data.len() {
                    self.consumed = self.len;
                    return None;
                }
                self.word = self.data[self.word_idx];
                // Apply mask if less than 64 bits remain
                let remaining = self.len.saturating_sub(self.word_idx * 64);
                if remaining < 64 {
                    self.word &= (1u64 << remaining) - 1;
                }
            }
        }
    }
}

impl BitAnd for &Bitmap {
    type Output = Bitmap;
    fn bitand(self, rhs: &Bitmap) -> Bitmap {
        let len = self.len.min(rhs.len);
        let words = (len + 63) / 64;
        let mut data = Vec::with_capacity(words);
        
        let min_len = self.data.len().min(rhs.data.len());
        for i in 0..min_len {
            data.push(self.data[i] & rhs.data[i]);
        }
        
        Bitmap { data, len }
    }
}

impl BitOr for &Bitmap {
    type Output = Bitmap;
    fn bitor(self, rhs: &Bitmap) -> Bitmap {
        let len = self.len.max(rhs.len);
        let words = (len + 63) / 64;
        let mut data = Vec::with_capacity(words);
        
        let max_len = self.data.len().max(rhs.data.len());
        for i in 0..max_len {
            let left = self.data.get(i).copied().unwrap_or(0);
            let right = rhs.data.get(i).copied().unwrap_or(0);
            data.push(left | right);
        }
        
        Bitmap { data, len }
    }
}

impl Not for &Bitmap {
    type Output = Bitmap;
    fn not(self) -> Bitmap {
        let words = (self.len + 63) / 64;
        let mut data = Vec::with_capacity(words);
        
        for i in 0..words {
            let word = self.data.get(i).copied().unwrap_or(0);
            data.push(!word);
        }
        
        if self.len % 64 != 0 && !data.is_empty() {
            if let Some(last) = data.last_mut() {
                *last &= (1u64 << (self.len % 64)) - 1;
            }
        }
        Bitmap { data, len: self.len }
    }
}
