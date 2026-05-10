pub mod writer;
pub mod reader;

#[cfg(feature = "parquet-storage")]
pub mod parquet_writer;
#[cfg(feature = "parquet-storage")]
pub mod parquet_reader;

pub use writer::SegmentWriter;
pub use reader::SegmentReader;

#[cfg(feature = "parquet-storage")]
pub use parquet_writer::{write_parquet_segment, ParquetWriterConfig, ParquetSegmentMeta, ColumnStats};
#[cfg(feature = "parquet-storage")]
pub use parquet_reader::{read_parquet_segment, read_parquet_meta, is_parquet_file, ParquetReadOptions, ReadPredicate, ScalarValue};
