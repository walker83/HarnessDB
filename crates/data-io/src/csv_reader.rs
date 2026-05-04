use types::{Block, Schema, DataType, Field, Vector, vector::*};
use std::io::{Read, BufRead};
use crate::schema_inference::infer_type;

pub struct CsvReader<R: Read> {
    reader: R,
    delimiter: u8,
    has_header: bool,
    schema: Option<Schema>,
    batch_size: usize,
    null_strings: Vec<String>,
    headers: Vec<String>,
    eof: bool,
}

impl<R: Read + BufRead> CsvReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            delimiter: b',',
            has_header: false,
            schema: None,
            batch_size: 1024,
            null_strings: vec!["\\N".to_string(), "NULL".to_string(), "null".to_string()],
            headers: Vec::new(),
            eof: false,
        }
    }

    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_header(mut self) -> Self {
        self.has_header = true;
        self
    }

    pub fn with_delimiter(mut self, d: u8) -> Self {
        self.delimiter = d;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn with_null_strings(mut self, nulls: Vec<String>) -> Self {
        self.null_strings = nulls;
        self
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    fn is_null_value(&self, value: &str) -> bool {
        self.null_strings.iter().any(|n| value.is_empty() || value == *n || value.eq_ignore_ascii_case(n))
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut in_quotes = false;
        let mut field = String::new();
        let delim = self.delimiter as char;

        for ch in line.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                }
                _ if ch == delim && !in_quotes => {
                    fields.push(field.trim().to_string());
                    field = String::new();
                }
                _ => {
                    field.push(ch);
                }
            }
        }
        fields.push(field.trim().to_string());
        fields
    }

    fn parse_quoted_field(&self, field: &str) -> String {
        let mut result = String::new();
        let mut chars = field.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '"' {
                if let Some(&next) = chars.peek()
                    && next == '"' {
                        result.push('"');
                        chars.next();
                    }
            } else {
                result.push(ch);
            }
        }
        result
    }

    pub fn next_batch(&mut self) -> common::Result<Option<Block>> {
        if self.eof {
            return Ok(None);
        }

        let mut line = String::new();

        // Read header if needed
        if self.has_header && self.headers.is_empty() {
            line.clear();
            match self.reader.read_line(&mut line)? {
                0 => {
                    self.eof = true;
                    return Ok(None);
                }
                _ => {
                    let fields = self.parse_line(line.trim());
                    self.headers = fields.clone();
                }
            }
        }

        // Build schema if not provided
        let schema = match &self.schema {
            Some(s) => s.clone(),
            None => {
                // Read first data row to infer schema
                line.clear();
                match self.reader.read_line(&mut line)? {
                    0 => {
                        self.eof = true;
                        return Ok(None);
                    }
                    _ => {
                        let fields = self.parse_line(line.trim());
                        let num_cols = fields.len();

                        // Infer types from first row
                        let inferred_fields: Vec<Field> = (0..num_cols).map(|i| {
                            let name = if self.has_header {
                                self.headers.get(i).cloned().unwrap_or_else(|| format!("col_{}", i))
                            } else {
                                format!("col_{}", i)
                            };
                            let value = fields.get(i).map(|s| s.as_str()).unwrap_or("");
                            let data_type = if self.is_null_value(value) {
                                DataType::String
                            } else {
                                infer_type(value)
                            };
                            Field {
                                name,
                                data_type,
                                nullable: true,
                            }
                        }).collect();

                        Schema::new(inferred_fields)
                    }
                }
            }
        };

        // Read rows and build columns
        let mut all_rows: Vec<Vec<String>> = Vec::new();

        // If schema was just inferred, we already read one line above
        if self.schema.is_none() && !self.has_header {
            // Use the line we already read
            all_rows.push(self.parse_line(line.trim()));
        }

        // Read remaining lines
        line.clear();
        while all_rows.len() < self.batch_size {
            match self.reader.read_line(&mut line)? {
                0 => break,
                _ => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        all_rows.push(self.parse_line(trimmed));
                    }
                }
            }
            line.clear();
        }

        if all_rows.is_empty() {
            self.eof = true;
            return Ok(None);
        }

        self.eof = true; // For simplicity, mark EOF after one batch

        // Build columns
        let num_cols = schema.num_fields();
        let mut columns: Vec<Vector> = Vec::with_capacity(num_cols);

        for col_idx in 0..num_cols {
            let data_type = schema.field(col_idx).map(|f| f.data_type.clone()).unwrap_or(DataType::String);

            let values: Vec<String> = all_rows.iter().map(|row| {
                row.get(col_idx).cloned().unwrap_or_default()
            }).collect();

            let vector = build_vector_from_strings(&values, &data_type, |v: &str| {
                // Check null strings
                v.is_empty() || v == "\\N" || v.eq_ignore_ascii_case("\\N")
                    || v == "NULL" || v.eq_ignore_ascii_case("null")
            });
            columns.push(vector);
        }

        Ok(Some(Block::new(schema, columns)))
    }
}

fn build_vector_from_strings<F>(
    values: &[String],
    data_type: &DataType,
    is_null: F,
) -> Vector
where
    F: Fn(&str) -> bool,
{
    match data_type {
        DataType::Int64 => {
            let mut vec = Int64Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0)));
                }
            }
            Vector::Int64(vec)
        }
        DataType::Int32 => {
            let mut vec = Int32Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0)));
                }
            }
            Vector::Int32(vec)
        }
        DataType::Int16 => {
            let mut vec = Int16Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0)));
                }
            }
            Vector::Int16(vec)
        }
        DataType::Int8 => {
            let mut vec = Int8Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0)));
                }
            }
            Vector::Int8(vec)
        }
        DataType::Float64 => {
            let mut vec = Float64Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0.0)));
                }
            }
            Vector::Float64(vec)
        }
        DataType::Float32 => {
            let mut vec = Float32Vector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.parse().unwrap_or(0.0)));
                }
            }
            Vector::Float32(vec)
        }
        DataType::Date => {
            let mut vec = DateVector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(parse_date_value(v)));
                }
            }
            Vector::Date(vec)
        }
        DataType::DateTime => {
            let mut vec = DateTimeVector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(parse_datetime_value(v)));
                }
            }
            Vector::DateTime(vec)
        }
        _ => {
            let mut vec = StringVector::new();
            for v in values {
                if is_null(v) {
                    vec.push(None);
                } else {
                    vec.push(Some(v.as_str()));
                }
            }
            Vector::String(vec)
        }
    }
}

fn parse_date_value(value: &str) -> i32 {
    use chrono::{NaiveDate, Datelike};

    let formats = [
        "%Y-%m-%d",
        "%Y/%m/%d",
        "%d-%m-%Y",
        "%d/%m/%Y",
        "%m-%d-%Y",
        "%m/%d/%Y",
        "%Y%m%d",
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            return date.ordinal().try_into().unwrap_or(0); // Day of year
        }
    }
    0
}

fn parse_datetime_value(value: &str) -> i64 {
    use chrono::NaiveDateTime;

    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M",
        "%Y/%m/%d %H:%M",
        "%Y%m%d %H:%M:%S",
        "%Y%m%d%H%M%S",
    ];

    for format in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return dt.and_utc().timestamp_millis();
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let reader = CsvReader::new("a,b,c".as_bytes());
        let fields = reader.parse_line("a,b,c");
        assert_eq!(fields, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_quoted() {
        let reader = CsvReader::new("a,b,c".as_bytes());
        let fields = reader.parse_line("\"a,b\",c");
        // Quoted field should have quotes stripped
        assert_eq!(fields, vec!["a,b", "c"]);
    }
}