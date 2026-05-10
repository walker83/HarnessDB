// Block ↔ Arrow RecordBatch conversion utilities.
//
// RorisDB has its own type system (types::Vector / types::Block).
// DataFusion works with Arrow (RecordBatch / Array).
// This module bridges the two.

use std::sync::Arc;

use arrow_array::array::*;
use arrow_array::RecordBatch;
use arrow_schema::Field;

use types::{Block, Vector, DataType as RorisType};
use types::vector::TypedVector;

// ---------------------------------------------------------------------------
// Block → RecordBatch  (IMPLEMENTED)
// ---------------------------------------------------------------------------

/// Convert a RorisDB `Block` into an Arrow `RecordBatch`.
pub fn block_to_record_batch(block: &Block) -> Result<RecordBatch, String> {
    let schema = block.schema();
    let arrow_fields: Vec<Field> = schema
        .fields()
        .iter()
        .map(|f| {
            Field::new(
                &f.name,
                crate::types::to_arrow_data_type(&f.data_type),
                f.nullable,
            )
        })
        .collect();
    let arrow_schema = Arc::new(arrow_schema::Schema::new(arrow_fields));

    let columns: Vec<Arc<dyn arrow_array::Array>> = block
        .columns()
        .iter()
        .map(|col| vector_to_array(col))
        .collect::<Result<_, _>>()?;

    RecordBatch::try_new(arrow_schema, columns)
        .map_err(|e| format!("RecordBatch build failed: {}", e))
}

fn vector_to_array(vec: &Vector) -> Result<Arc<dyn arrow_array::Array>, String> {
    match vec {
        Vector::Boolean(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(BooleanArray::from_iter(iter)))
        }
        Vector::Int8(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Int8Array::from_iter(iter)))
        }
        Vector::Int16(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Int16Array::from_iter(iter)))
        }
        Vector::Int32(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Int32Array::from_iter(iter)))
        }
        Vector::Int64(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Int64Array::from_iter(iter)))
        }
        Vector::Float32(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Float32Array::from_iter(iter)))
        }
        Vector::Float64(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Float64Array::from_iter(iter)))
        }
        Vector::String(v) => {
            // StringVector::get(i) returns Option<&str>
            let iter = (0..v.len()).map(|i| v.get(i).map(|s| s.to_string()));
            Ok(Arc::new(StringArray::from_iter(iter)))
        }
        Vector::Date(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(Date32Array::from_iter(iter)))
        }
        Vector::DateTime(v) => {
            let iter = (0..v.len()).map(|i| v.get(i));
            Ok(Arc::new(TimestampSecondArray::from_iter(iter)))
        }
        Vector::Null(_) => {
            let len = vec.len();
            Ok(Arc::new(NullArray::new(len)))
        }
        _ => Err(format!("Unsupported vector type for Arrow conversion: {:?}", vec)),
    }
}

// ---------------------------------------------------------------------------
// RecordBatch → Block  (STUB — implement when needed)
// ---------------------------------------------------------------------------

pub fn record_batch_to_block(_rb: &RecordBatch) -> Result<Block, String> {
    Err("record_batch_to_block not yet implemented".to_string())
}

// ---------------------------------------------------------------------------
// Arrow Array → Vector  (STUB — implement when needed)
// ---------------------------------------------------------------------------

pub fn array_to_vector(_array: &Arc<dyn arrow_array::Array>) -> Result<Vector, String> {
    // TODO: implement downcast logic when needed
    Err("array_to_vector not yet implemented".to_string())
}
