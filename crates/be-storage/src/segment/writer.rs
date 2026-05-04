use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use types::{Block, DataType, Field, Schema, Vector};

use crate::codec::{self, EncodingType};
use crate::index::ZoneMap;

const DEFAULT_PAGE_SIZE: usize = 64 * 1024;
const MAGIC: &[u8; 8] = b"ROVSSEG\0";
const VERSION: u32 = 1;

/// Column page on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMeta {
    pub offset: u64,
    pub size: u64,
    pub num_rows: u32,
    pub encoding: EncodingType,
    pub zone_map: ZoneMap,
}

/// Column metadata in the footer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMeta {
    pub column_name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub pages: Vec<PageMeta>,
}

/// Segment footer containing schema and page offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentFooter {
    pub magic: [u8; 8],
    pub version: u32,
    pub num_rows: u64,
    pub columns: Vec<ColumnMeta>,
    pub schema: SchemaDesc,
}

/// Serializable schema descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDesc {
    pub fields: Vec<FieldDesc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDesc {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

impl From<&Schema> for SchemaDesc {
    fn from(schema: &Schema) -> Self {
        Self {
            fields: schema
                .fields()
                .iter()
                .map(|f| FieldDesc {
                    name: f.name.clone(),
                    data_type: f.data_type.clone(),
                    nullable: f.nullable,
                })
                .collect(),
        }
    }
}

/// Write a segment file from a Block.
pub struct SegmentWriter;

impl SegmentWriter {
    /// Write a block to a segment file at the given path.
    /// Uses column-oriented layout with pages, zone maps, and optional compression.
    pub fn write_segment(path: &Path, block: &Block) -> Result<u64, String> {
        let schema = block.schema();
        let num_rows = block.num_rows();
        if num_rows == 0 {
            return Err("Cannot write empty block".to_string());
        }

        let mut file = std::fs::File::create(path)
            .map_err(|e| format!("Failed to create segment file: {}", e))?;

        // Write magic header
        file.write_all(MAGIC)
            .map_err(|e| format!("Write error: {}", e))?;
        file.write_all(&VERSION.to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;

        let mut column_metas = Vec::new();
        let mut offset: u64 = (MAGIC.len() as u64) + 4; // magic(8) + version(4)

        // Flush to ensure header is written before continuing
        file.flush().map_err(|e| format!("Flush error: {}", e))?;

        // Write each column's pages
        for (col_idx, field) in schema.fields().iter().enumerate() {
            let column = block.column(col_idx).ok_or_else(|| {
                format!("Column index {} out of bounds", col_idx)
            })?;

            let col_meta = Self::write_column(&mut file, &mut offset, field, column)?;
            column_metas.push(col_meta);
        }

        // Build and write footer
        let footer = SegmentFooter {
            magic: MAGIC.clone(),
            version: VERSION,
            num_rows: num_rows as u64,
            columns: column_metas,
            schema: SchemaDesc::from(schema),
        };

        let footer_json = serde_json::to_vec(&footer)
            .map_err(|e| format!("Footer serialize error: {}", e))?;
        let footer_offset = offset;
        file.write_all(&footer_json)
            .map_err(|e| format!("Write footer error: {}", e))?;

        // Write footer offset + length at the very end
        file.write_all(&(footer_json.len() as u64).to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        file.write_all(&footer_offset.to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        file.flush().map_err(|e| format!("Flush error: {}", e))?;

        Ok(offset + footer_json.len() as u64 + 16)
    }

    fn write_column(
        file: &mut std::fs::File,
        offset: &mut u64,
        field: &Field,
        column: &Vector,
    ) -> Result<ColumnMeta, String> {
        let num_rows = column.len();
        let page_row_limit = Self::page_row_limit(field);

        let mut pages = Vec::new();
        let mut row_start = 0;

        while row_start < num_rows {
            let page_rows = page_row_limit.min(num_rows - row_start);
            let page_vec = column.slice(row_start, page_rows);

            let page_meta = Self::write_page(file, offset, &page_vec, field)?;
            pages.push(page_meta);
            row_start += page_rows;
        }

        Ok(ColumnMeta {
            column_name: field.name.clone(),
            data_type: field.data_type.clone(),
            nullable: field.nullable,
            pages,
        })
    }

    /// Determine how many rows fit in a single page based on type size.
    fn page_row_limit(field: &Field) -> usize {
        let type_size = field.data_type.size().max(1);
        (DEFAULT_PAGE_SIZE / type_size).max(1024)
    }

    fn write_page(
        file: &mut std::fs::File,
        offset: &mut u64,
        column: &Vector,
        field: &Field,
    ) -> Result<PageMeta, String> {
        // Serialize the column data to raw bytes
        let raw_data = Self::serialize_column(column);

        // Build zone map from scalar values
        let values: Vec<types::ScalarValue> = (0..column.len())
            .map(|i| column.scalar_at(i))
            .collect();
        let zone_map = ZoneMap::build(&values);

        // Choose encoding and optionally compress
        let cardinality_ratio = Self::estimate_cardinality(&values);
        let encoding = codec::choose_encoding(&field.data_type, cardinality_ratio, false);

        let encoded_data = match encoding {
            EncodingType::Lz4 => codec::lz4_compress(&raw_data),
            _ => raw_data.clone(),
        };

        // Write null bitmap
        let null_bitmap = Self::serialize_null_bitmap(column);
        let page_start = *offset;

        // Layout: [null_bitmap_len(8)] [null_bitmap] [data_len(8)] [data]
        use std::io::Write;
        file.write_all(&(null_bitmap.len() as u64).to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        *offset += 8;

        file.write_all(&null_bitmap)
            .map_err(|e| format!("Write error: {}", e))?;
        *offset += null_bitmap.len() as u64;

        file.write_all(&(encoded_data.len() as u64).to_le_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        *offset += 8;

        file.write_all(&encoded_data)
            .map_err(|e| format!("Write error: {}", e))?;
        *offset += encoded_data.len() as u64;

        let page_size = *offset - page_start;

        Ok(PageMeta {
            offset: page_start,
            size: page_size,
            num_rows: column.len() as u32,
            encoding,
            zone_map,
        })
    }

    fn serialize_column(column: &Vector) -> Vec<u8> {
        match column {
            Vector::Boolean(v) => {
                let mut buf = Vec::with_capacity(v.len());
                for i in 0..v.len() {
                    buf.push(if v.get(i).unwrap_or(false) { 1u8 } else { 0u8 });
                }
                buf
            }
            Vector::Int8(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::Int16(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::Int32(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::Int64(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::Int128(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::Float32(v) => {
                let data = v.data();
                data.iter().flat_map(|f| f.to_le_bytes()).collect()
            }
            Vector::Float64(v) => {
                let data = v.data();
                data.iter().flat_map(|f| f.to_le_bytes()).collect()
            }
            Vector::Date(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::DateTime(v) => {
                let data = v.data();
                data.iter().flat_map(|n| n.to_le_bytes()).collect()
            }
            Vector::String(v) => {
                // Encode as: num_strings(4) + [len(4) + bytes]...
                let len = v.len();
                let mut buf = Vec::new();
                buf.extend_from_slice(&(len as u32).to_le_bytes());
                for i in 0..len {
                    match v.get(i) {
                        Some(s) => {
                            let bytes = s.as_bytes();
                            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                            buf.extend_from_slice(bytes);
                        }
                        None => {
                            buf.extend_from_slice(&0u32.to_le_bytes());
                        }
                    }
                }
                buf
            }
            Vector::Json(v) => {
                // Encode JSON as serialized string
                let len = v.len();
                let mut buf = Vec::new();
                buf.extend_from_slice(&(len as u32).to_le_bytes());
                for i in 0..len {
                    match v.get(i) {
                        Some(val) => {
                            let json_str = serde_json::to_string(&val).unwrap_or_else(|_| "null".to_string());
                            let bytes = json_str.as_bytes();
                            buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                            buf.extend_from_slice(bytes);
                        }
                        None => {
                            buf.extend_from_slice(&0u32.to_le_bytes());
                        }
                    }
                }
                buf
            }
            Vector::Null(v) => {
                vec![0u8; v.len()]
            }
        }
    }

    fn serialize_null_bitmap(column: &Vector) -> Vec<u8> {
        let num_rows = column.len();
        let bitmap_words = (num_rows + 63) / 64;
        let mut bitmap = vec![0u64; bitmap_words];

        for i in 0..num_rows {
            let is_valid = match column {
                Vector::Boolean(v) => v.validity().is_valid(i),
                Vector::Int8(v) => v.validity().is_valid(i),
                Vector::Int16(v) => v.validity().is_valid(i),
                Vector::Int32(v) => v.validity().is_valid(i),
                Vector::Int64(v) => v.validity().is_valid(i),
                Vector::Int128(v) => v.validity().is_valid(i),
                Vector::Float32(v) => v.validity().is_valid(i),
                Vector::Float64(v) => v.validity().is_valid(i),
                Vector::Date(v) => v.validity().is_valid(i),
                Vector::DateTime(v) => v.validity().is_valid(i),
                Vector::String(v) => v.validity().is_valid(i),
                Vector::Json(v) => v.validity().is_valid(i),
                Vector::Null(_) => false,
            };
            if is_valid {
                let word_idx = i / 64;
                let bit_idx = i % 64;
                bitmap[word_idx] |= 1u64 << bit_idx;
            }
        }

        bitmap.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn estimate_cardinality(values: &[types::ScalarValue]) -> f64 {
        if values.is_empty() {
            return 1.0;
        }
        let non_null: Vec<&types::ScalarValue> = values.iter().filter(|v| !v.is_null()).collect();
        if non_null.is_empty() {
            return 0.0;
        }
        // Use a hash set approximation for cardinality
        let mut seen = std::collections::HashSet::new();
        for v in &non_null {
            seen.insert(format!("{:?}", v));
        }
        seen.len() as f64 / non_null.len() as f64
    }
}
