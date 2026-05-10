use std::path::Path;

use types::{Block, DataType, Field, Schema, Vector, Bitmap};

use crate::codec;
use crate::index::{ColumnPredicate, eval_predicate};
use super::writer::SegmentFooter;

/// Read and scan segment files with column projection and predicate pushdown.
pub struct SegmentReader;

impl SegmentReader {
    /// Read a segment file footer.
    pub fn read_footer(path: &Path) -> Result<SegmentFooter, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read segment file: {}", e))?;

        if data.len() < 16 {
            return Err("Segment file too small".to_string());
        }

        // Last 16 bytes: footer_length(8) + footer_offset(8)
        let len = data.len();
        let footer_len = u64::from_le_bytes(
            data[len - 16..len - 8].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?
        ) as usize;
        let footer_offset = u64::from_le_bytes(
            data[len - 8..len].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?
        ) as usize;

        if footer_offset + footer_len + 16 > len {
            return Err(format!(
                "Footer offset+len ({}) exceeds file size ({})",
                footer_offset + footer_len + 16,
                len
            ));
        }

        let footer_bytes = &data[footer_offset..footer_offset + footer_len];
        serde_json::from_slice(footer_bytes)
            .map_err(|e| format!("Footer deserialize error: {}", e))
    }

    /// Scan a segment file, reading only the requested columns and applying predicates.
    pub fn scan_segment(
        path: &Path,
        projection: Option<&[usize]>,
        predicates: &[ColumnPredicate],
    ) -> Result<Block, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read segment file: {}", e))?;

        let footer = Self::read_footer(path)?;

        // Build schema from footer
        let schema = Self::build_schema(&footer);

        // Determine which columns to actually read
        let col_indices: Vec<usize> = if let Some(proj) = projection {
            proj.to_vec()
        } else {
            (0..schema.num_fields()).collect()
        };

        // Also figure out which columns are needed for predicates
        let pred_col_indices: Vec<usize> = predicates
            .iter()
            .filter_map(|p| schema.index_of(&p.column_name))
            .collect();

        // All columns we need to read
        let mut all_needed = col_indices.clone();
        for idx in &pred_col_indices {
            if !all_needed.contains(idx) {
                all_needed.push(*idx);
            }
        }
        all_needed.sort();

        // Read needed columns
        let mut columns: Vec<(usize, Vector)> = Vec::new();
        for &col_idx in &all_needed {
            let col_meta = footer.columns.get(col_idx).ok_or_else(|| {
                format!("Column index {} out of bounds", col_idx)
            })?;

            let vector = Self::read_column(&data, col_meta)?;
            columns.push((col_idx, vector));
        }

        // Apply predicates to build selection bitmap
        let num_rows = footer.num_rows as usize;
        let selection = if predicates.is_empty() {
            Bitmap::all_set(num_rows)
        } else {
            Self::apply_predicates(&columns, &pred_col_indices, predicates, num_rows, &schema)?
        };

        // Project to requested columns and apply selection
        let proj_schema = if let Some(proj) = projection {
            schema.project(proj)
        } else {
            schema
        };

        let mut proj_columns = Vec::new();
        for &col_idx in &col_indices {
            if let Some((_, vec)) = columns.iter().find(|(idx, _)| *idx == col_idx) {
                proj_columns.push(vec.filter(&selection));
            }
        }

        Ok(Block::new(proj_schema, proj_columns))
    }

    fn build_schema(footer: &SegmentFooter) -> Schema {
        let fields: Vec<Field> = footer
            .schema
            .fields
            .iter()
            .map(|f| Field::new(&f.name, f.data_type.clone(), f.nullable))
            .collect();
        Schema::new(fields)
    }

    fn read_column(
        file_data: &[u8],
        col_meta: &super::writer::ColumnMeta,
    ) -> Result<Vector, String> {
        let mut page_vectors: Vec<Vector> = Vec::new();

        for page_meta in &col_meta.pages {
            // Zone map pruning is done at a higher level; here we read the page
            let vec = Self::read_page(file_data, page_meta, &col_meta.data_type)?;
            page_vectors.push(vec);
        }

        if page_vectors.is_empty() {
            return Ok(Self::empty_vector(&col_meta.data_type));
        }

        // Concatenate pages
        let mut result = page_vectors.swap_remove(0);
        for v in page_vectors {
            result.append_vector(&v);
        }
        Ok(result)
    }

    fn read_page(
        file_data: &[u8],
        page_meta: &super::writer::PageMeta,
        data_type: &DataType,
    ) -> Result<Vector, String> {
        let start = page_meta.offset as usize;
        let page_end = start + page_meta.size as usize;
        if page_end > file_data.len() {
            return Err("Page extends beyond file".to_string());
        }

        let page_data = &file_data[start..page_end];

        // Layout: [null_bitmap_len(8)] [null_bitmap] [data_len(8)] [data]
        if page_data.len() < 8 {
            return Err("Page data too small".to_string());
        }

        let null_bitmap_len = u64::from_le_bytes(
            page_data[0..8].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?
        ) as usize;

        let null_bitmap_end = 8 + null_bitmap_len;
        if null_bitmap_end + 8 > page_data.len() {
            return Err("Page data truncated (null bitmap)".to_string());
        }

        let null_bitmap_data = &page_data[8..null_bitmap_end];
        let null_bitmap = Self::deserialize_null_bitmap(null_bitmap_data, page_meta.num_rows as usize);

        let data_len = u64::from_le_bytes(
            page_data[null_bitmap_end..null_bitmap_end + 8]
                .try_into()
                .map_err(|e: std::array::TryFromSliceError| e.to_string())?
        ) as usize;

        let encoded_data = &page_data[null_bitmap_end + 8..null_bitmap_end + 8 + data_len];

        // Decompress if needed
        let raw_data = match page_meta.encoding {
            codec::EncodingType::Lz4 => {
                // We need the original size - use type size * num_rows as estimate
                let est_size = data_type.size().max(1) * page_meta.num_rows as usize;
                codec::lz4_decompress(encoded_data, est_size)?
            }
            codec::EncodingType::Zstd => {
                codec::zstd_decompress(encoded_data)?
            }
            _ => encoded_data.to_vec(),
        };

        // Deserialize into a vector
        Self::deserialize_vector(&raw_data, data_type, &null_bitmap, page_meta.num_rows as usize)
    }

    fn deserialize_null_bitmap(data: &[u8], num_rows: usize) -> Bitmap {
        let words: Vec<u64> = data
            .chunks(8)
            .map(|chunk| {
                let mut bytes = [0u8; 8];
                let len = chunk.len().min(8);
                bytes[..len].copy_from_slice(&chunk[..len]);
                u64::from_le_bytes(bytes)
            })
            .collect();

        // Reconstruct Bitmap
        let mut bm = Bitmap::with_capacity(num_rows);
        // We can't directly set the internal data, so use push
        // But for efficiency, let's read from the words
        for i in 0..num_rows {
            let word_idx = i / 64;
            let bit_idx = i % 64;
            let is_valid = if word_idx < words.len() {
                (words[word_idx] >> bit_idx) & 1 == 1
            } else {
                false
            };
            bm.push(is_valid);
        }
        bm
    }

    fn deserialize_vector(
        data: &[u8],
        data_type: &DataType,
        null_bitmap: &Bitmap,
        num_rows: usize,
    ) -> Result<Vector, String> {
        match data_type {
            DataType::Boolean => {
                let mut vec = types::vector::BooleanVector::new();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < data.len() {
                        vec.push(Some(data[i] != 0));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Boolean(vec))
            }
            DataType::Int8 => {
                let mut vec = types::vector::Int8Vector::new();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < data.len() {
                        vec.push(Some(data[i] as i8));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Int8(vec))
            }
            DataType::Int16 => {
                let mut vec = types::vector::Int16Vector::new();
                let values: Vec<i16> = data
                    .chunks(2)
                    .map(|c| {
                        let mut b = [0u8; 2];
                        b[..c.len().min(2)].copy_from_slice(&c[..c.len().min(2)]);
                        i16::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Int16(vec))
            }
            DataType::Int32 => {
                let mut vec = types::vector::Int32Vector::new();
                let values: Vec<i32> = data
                    .chunks(4)
                    .map(|c| {
                        let mut b = [0u8; 4];
                        b[..c.len().min(4)].copy_from_slice(&c[..c.len().min(4)]);
                        i32::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Int32(vec))
            }
            DataType::Int64 | DataType::DateTime => {
                let values: Vec<i64> = data
                    .chunks(8)
                    .map(|c| {
                        let mut b = [0u8; 8];
                        b[..c.len().min(8)].copy_from_slice(&c[..c.len().min(8)]);
                        i64::from_le_bytes(b)
                    })
                    .collect();
                if matches!(data_type, DataType::DateTime) {
                    let mut v = types::vector::DateTimeVector::new();
                    for i in 0..num_rows {
                        if null_bitmap.is_valid(i) && i < values.len() {
                            v.push(Some(values[i]));
                        } else {
                            v.push(None);
                        }
                    }
                    Ok(Vector::DateTime(v))
                } else {
                    let mut v = types::vector::Int64Vector::new();
                    for i in 0..num_rows {
                        if null_bitmap.is_valid(i) && i < values.len() {
                            v.push(Some(values[i]));
                        } else {
                            v.push(None);
                        }
                    }
                    Ok(Vector::Int64(v))
                }
            }
            DataType::Int128 => {
                let mut vec = types::vector::Int128Vector::new();
                let values: Vec<i128> = data
                    .chunks(16)
                    .map(|c| {
                        let mut b = [0u8; 16];
                        b[..c.len().min(16)].copy_from_slice(&c[..c.len().min(16)]);
                        i128::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Int128(vec))
            }
            DataType::Float32 => {
                let mut vec = types::vector::Float32Vector::new();
                let values: Vec<f32> = data
                    .chunks(4)
                    .map(|c| {
                        let mut b = [0u8; 4];
                        b[..c.len().min(4)].copy_from_slice(&c[..c.len().min(4)]);
                        f32::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Float32(vec))
            }
            DataType::Float64 => {
                let mut vec = types::vector::Float64Vector::new();
                let values: Vec<f64> = data
                    .chunks(8)
                    .map(|c| {
                        let mut b = [0u8; 8];
                        b[..c.len().min(8)].copy_from_slice(&c[..c.len().min(8)]);
                        f64::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Float64(vec))
            }
            DataType::Date => {
                let mut vec = types::vector::DateVector::new();
                let values: Vec<i32> = data
                    .chunks(4)
                    .map(|c| {
                        let mut b = [0u8; 4];
                        b[..c.len().min(4)].copy_from_slice(&c[..c.len().min(4)]);
                        i32::from_le_bytes(b)
                    })
                    .collect();
                for i in 0..num_rows {
                    if null_bitmap.is_valid(i) && i < values.len() {
                        vec.push(Some(values[i]));
                    } else {
                        vec.push(None);
                    }
                }
                Ok(Vector::Date(vec))
            }
            DataType::String => {
                let mut vec = types::vector::StringVector::new();
                // Read num_strings
                if data.len() < 4 {
                    return Ok(Vector::String(vec));
                }
                let num_strings = u32::from_le_bytes(
                    data[0..4].try_into().map_err(|e: std::array::TryFromSliceError| e.to_string())?
                ) as usize;
                let mut offset = 4;

                for i in 0..num_strings.min(num_rows) {
                    if null_bitmap.is_valid(i) {
                        if offset + 4 > data.len() {
                            vec.push(None);
                            continue;
                        }
                        let str_len = u32::from_le_bytes(
                            data[offset..offset + 4]
                                .try_into()
                                .map_err(|e: std::array::TryFromSliceError| e.to_string())?
                        ) as usize;
                        offset += 4;
                        if str_len > 0 && offset + str_len <= data.len() {
                            let s = std::str::from_utf8(&data[offset..offset + str_len])
                                .unwrap_or("");
                            vec.push(Some(s));
                            offset += str_len;
                        } else {
                        vec.push(None);
                        }
                    } else {
                        // Skip the string entry (read past it)
                        if offset + 4 <= data.len() {
                            let str_len = u32::from_le_bytes(
                                data[offset..offset + 4]
                                    .try_into()
                                    .unwrap_or([0u8; 4])
                            ) as usize;
                            offset += 4 + str_len;
                        }
                        vec.push(None);
                    }
                }
                Ok(Vector::String(vec))
            }
            _ => {
                // Fallback: treat as null vector
                Ok(Vector::Null(types::vector::NullVector::new(num_rows)))
            }
        }
    }

    fn empty_vector(data_type: &DataType) -> Vector {
        match data_type {
            DataType::Boolean => Vector::Boolean(types::vector::BooleanVector::new()),
            DataType::Int8 => Vector::Int8(types::vector::Int8Vector::new()),
            DataType::Int16 => Vector::Int16(types::vector::Int16Vector::new()),
            DataType::Int32 => Vector::Int32(types::vector::Int32Vector::new()),
            DataType::Int64 => Vector::Int64(types::vector::Int64Vector::new()),
            DataType::Int128 => Vector::Int128(types::vector::Int128Vector::new()),
            DataType::Float32 => Vector::Float32(types::vector::Float32Vector::new()),
            DataType::Float64 => Vector::Float64(types::vector::Float64Vector::new()),
            DataType::String => Vector::String(types::vector::StringVector::new()),
            DataType::Date => Vector::Date(types::vector::DateVector::new()),
            DataType::DateTime => Vector::DateTime(types::vector::DateTimeVector::new()),
            _ => Vector::Null(types::vector::NullVector::new(0)),
        }
    }

    /// Apply predicates to columns and return a selection bitmap.
    fn apply_predicates(
        columns: &[(usize, Vector)],
        _pred_col_indices: &[usize],
        predicates: &[ColumnPredicate],
        num_rows: usize,
        schema: &Schema,
    ) -> Result<Bitmap, String> {
        let mut selection = Bitmap::all_set(num_rows);

        for predicate in predicates {
            let col_idx = schema
                .index_of(&predicate.column_name)
                .ok_or_else(|| format!("Column '{}' not found", predicate.column_name))?;

            let column = columns
                .iter()
                .find(|(idx, _)| *idx == col_idx)
                .map(|(_, v)| v)
                .ok_or_else(|| format!("Column '{}' not loaded", predicate.column_name))?;

            let mut col_selection = Bitmap::with_capacity(num_rows);
            for i in 0..num_rows {
                let val = column.scalar_at(i);
                col_selection.push(eval_predicate(&predicate.op, &val, &predicate.value));
            }

            // AND with running selection
            selection = (&selection) & (&col_selection);
        }

        Ok(selection)
    }
}
