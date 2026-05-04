use types::{Block, Schema, ScalarValue};
use std::io::{Write, Result};

pub struct CsvWriter<W: Write> {
    writer: W,
    delimiter: u8,
    include_header: bool,
    schema: Option<Schema>,
}

impl<W: Write> CsvWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            delimiter: b',',
            include_header: true,
            schema: None,
        }
    }

    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_delimiter(mut self, d: u8) -> Self {
        self.delimiter = d;
        self
    }

    pub fn with_header(mut self, include: bool) -> Self {
        self.include_header = include;
        self
    }

    pub fn write_block(&mut self, block: &Block) -> Result<()> {
        let schema = self.schema.clone().unwrap_or_else(|| block.schema().clone());
        let num_rows = block.num_rows();
        let num_cols = schema.num_fields();

        // Write header if requested
        if self.include_header {
            for (i, field) in schema.fields().iter().enumerate() {
                if i > 0 {
                    write!(self.writer, "{}", self.delimiter as char)?;
                }
                write_csv_field(&mut self.writer, &field.name, self.delimiter)?;
            }
            writeln!(self.writer)?;
        }

        // Write data rows
        for row_idx in 0..num_rows {
            for col_idx in 0..num_cols {
                if col_idx > 0 {
                    write!(self.writer, "{}", self.delimiter as char)?;
                }

                let vector = block.column(col_idx).unwrap();
                let value = vector.scalar_at(row_idx);
                let field_value = scalar_value_to_string(&value);
                write_csv_field(&mut self.writer, &field_value, self.delimiter)?;
            }
            writeln!(self.writer)?;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> Drop for CsvWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Writes a CSV field, escaping delimiters, quotes, and newlines
fn write_csv_field<W: Write>(writer: &mut W, field: &str, delimiter: u8) -> Result<()> {
    let needs_quotes = field.contains(delimiter as char)
        || field.contains('"')
        || field.contains('\n')
        || field.contains('\r');

    if needs_quotes {
        write!(writer, "\"")?;
        for ch in field.chars() {
            match ch {
                '"' => write!(writer, "\"\"")?,
                _ => write!(writer, "{}", ch)?,
            }
        }
        write!(writer, "\"")?;
    } else {
        write!(writer, "{}", field)?;
    }

    Ok(())
}

/// Converts a ScalarValue to a string representation
fn scalar_value_to_string(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Null => String::new(),
        ScalarValue::Boolean(b) => b.to_string(),
        ScalarValue::Int8(n) => n.to_string(),
        ScalarValue::Int16(n) => n.to_string(),
        ScalarValue::Int32(n) => n.to_string(),
        ScalarValue::Int64(n) => n.to_string(),
        ScalarValue::Int128(n) => n.to_string(),
        ScalarValue::Float32(f) => f.to_string(),
        ScalarValue::Float64(f) => f.to_string(),
        ScalarValue::Date(_days) => {
            // Convert ordinal day to date string (simplified)
            format!("date:{}", _days)
        }
        ScalarValue::DateTime(ms) => {
            // Convert milliseconds since epoch to datetime string
            if let Some(dt) = chrono::DateTime::from_timestamp_millis(*ms) {
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                ms.to_string()
            }
        }
        ScalarValue::String(s) => s.clone(),
        ScalarValue::Binary(b) => {
            // Convert binary to hex string
            b.iter().map(|byte| format!("{:02x}", byte)).collect()
        }
        ScalarValue::Array(arr) => {
            // Convert array to JSON-like string
            let items: Vec<String> = arr.iter().map(scalar_value_to_string).collect();
            format!("[{}]", items.join(","))
        }
        ScalarValue::Json(j) => {
            serde_json::to_string(j).unwrap_or_else(|_| "null".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_write_csv_field_no_escape() {
        let mut output = Vec::new();
        write_csv_field(&mut output, "hello", b',').unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "hello");
    }

    #[test]
    fn test_write_csv_field_with_delimiter() {
        let mut output = Vec::new();
        write_csv_field(&mut output, "hello,world", b',').unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\"hello,world\"");
    }

    #[test]
    fn test_write_csv_field_with_quotes() {
        let mut output = Vec::new();
        write_csv_field(&mut output, "say \"hello\"", b',').unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_write_csv_field_with_newline() {
        let mut output = Vec::new();
        write_csv_field(&mut output, "line1\nline2", b',').unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\"line1\nline2\"");
    }

    #[test]
    fn test_scalar_value_to_string() {
        assert_eq!(scalar_value_to_string(&ScalarValue::Int64(42)), "42");
        assert_eq!(scalar_value_to_string(&ScalarValue::Float64(3.14)), "3.14");
        assert_eq!(scalar_value_to_string(&ScalarValue::String("hello".to_string())), "hello");
        assert_eq!(scalar_value_to_string(&ScalarValue::Null), "");
    }
}