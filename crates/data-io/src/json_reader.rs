use types::{Block, Schema, Field, DataType, Vector, vector::*};
use serde_json::{Value, Map};
use std::io::{Read, BufRead};

pub struct JsonReader<R: Read> {
    reader: R,
    batch_size: usize,
    schema: Option<Schema>,
    headers: Vec<String>,
    eof: bool,
}

impl<R: Read + BufRead> JsonReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            batch_size: 1024,
            schema: None,
            headers: Vec::new(),
            eof: false,
        }
    }

    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    /// Flattens a JSON object into a map of column names to values
    fn flatten_json(value: &Value) -> Map<String, Value> {
        let mut result = Map::new();
        flatten_json_recursive(value, String::new(), &mut result);
        result
    }
}

fn flatten_json_recursive(value: &Value, prefix: String, result: &mut Map<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_json_recursive(val, new_key, result);
            }
        }
        Value::Array(arr) => {
            result.insert(prefix, Value::String(serde_json::to_string(arr).unwrap_or_default()));
        }
        _ => {
            result.insert(prefix, value.clone());
        }
    }
}

impl<R: Read + BufRead> JsonReader<R> {
    pub fn next_batch(&mut self) -> common::Result<Option<Block>> {
        if self.eof {
            return Ok(None);
        }

        let mut lines: Vec<String> = Vec::with_capacity(self.batch_size);
        let mut eof = false;

        {
            let mut line = String::new();
            while lines.len() < self.batch_size {
                line.clear();
                match self.reader.read_line(&mut line)? {
                    0 => {
                        eof = true;
                        break;
                    }
                    _ => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        lines.push(trimmed.to_string());
                    }
                }
            }
        }

        if lines.is_empty() {
            self.eof = true;
            return Ok(None);
        }

        self.eof = eof;

        // Parse all JSON lines
        let mut json_maps: Vec<Map<String, Value>> = Vec::with_capacity(lines.len());
        for line in &lines {
            match serde_json::from_str::<Value>(line) {
                Ok(Value::Object(map)) => {
                    let flat = Self::flatten_json(&Value::Object(map));
                    json_maps.push(flat);
                }
                Ok(Value::Array(arr)) => {
                    for item in arr {
                        if let Value::Object(_) = item {
                            let flat = Self::flatten_json(&item);
                            json_maps.push(flat);
                        }
                    }
                }
                Ok(_) => {
                    let mut map = Map::new();
                    map.insert("value".to_string(), Value::String(line.clone()));
                    json_maps.push(map);
                }
                Err(_) => {
                    tracing::warn!("Failed to parse JSON line: {}", line);
                    continue;
                }
            }
        }

        if json_maps.is_empty() {
            return Ok(None);
        }

        // Build schema from first JSON object, or use provided schema
        let schema = if let Some(ref schema) = self.schema {
            schema.clone()
        } else {
            let mut all_keys: Vec<String> = json_maps[0].keys().cloned().collect();
            for json_map in &json_maps[1..] {
                for key in json_map.keys() {
                    if !all_keys.contains(key) {
                        all_keys.push(key.clone());
                    }
                }
            }
            all_keys.sort();

            let fields: Vec<Field> = all_keys.iter().map(|name| {
                let mut data_type = DataType::String;
                for json_map in &json_maps {
                    if let Some(value) = json_map.get(name) {
                        if !value.is_null() {
                            data_type = infer_type_from_json_value(value);
                            break;
                        }
                    }
                }
                Field {
                    name: name.clone(),
                    data_type,
                    nullable: true,
                }
            }).collect();
            Schema::new(fields)
        };

        self.headers = schema.names().iter().map(|s| s.to_string()).collect();

        // Build columns
        let num_cols = schema.num_fields();
        let mut columns: Vec<Vector> = Vec::with_capacity(num_cols);

        for col_idx in 0..num_cols {
            let field = schema.field(col_idx).unwrap();
            let mut vector = create_vector_for_type(&field.data_type);

            for json_map in &json_maps {
                let value = json_map.get(&field.name);
                let scalar = json_value_to_scalar(value, &field.data_type);
                push_scalar_to_vector(&mut vector, &scalar);
            }

            columns.push(vector);
        }

        Ok(Some(Block::new(schema, columns)))
    }
}

fn infer_type_from_json_value(value: &Value) -> DataType {
    match value {
        Value::Null => DataType::String,
        Value::Bool(_) => DataType::Boolean,
        Value::Number(n) => {
            if n.is_i64() {
                DataType::Int64
            } else if n.is_f64() {
                DataType::Float64
            } else {
                DataType::String
            }
        }
        Value::String(_) => DataType::String,
        Value::Array(_) | Value::Object(_) => DataType::String,
    }
}

fn json_value_to_scalar(value: Option<&Value>, data_type: &DataType) -> types::ScalarValue {
    use types::{ScalarValue, JsonValue};

    match value {
        None | Some(Value::Null) => ScalarValue::Null,
        Some(Value::Bool(b)) => {
            if matches!(data_type, DataType::Json) {
                ScalarValue::Json(JsonValue::Bool(*b))
            } else {
                ScalarValue::Boolean(*b)
            }
        }
        Some(Value::Number(n)) => {
            match data_type {
                DataType::Int64 | DataType::Int32 | DataType::Int16 | DataType::Int8 => {
                    n.as_i64().map(ScalarValue::Int64).unwrap_or(ScalarValue::Null)
                }
                DataType::Float64 | DataType::Float32 => {
                    n.as_f64().map(ScalarValue::Float64).unwrap_or(ScalarValue::Null)
                }
                DataType::Json => {
                    n.as_f64().map(JsonValue::Number).map(ScalarValue::Json)
                        .unwrap_or(ScalarValue::Null)
                }
                _ => {
                    n.to_string().parse::<i64>()
                        .map(ScalarValue::Int64)
                        .or_else(|_| n.to_string().parse::<f64>().map(ScalarValue::Float64))
                        .unwrap_or(ScalarValue::String(n.to_string()))
                }
            }
        }
        Some(Value::String(s)) => {
            match data_type {
                DataType::Date => parse_date_scalar(s).map(ScalarValue::Date).unwrap_or(ScalarValue::String(s.clone())),
                DataType::DateTime => parse_datetime_scalar(s).map(ScalarValue::DateTime).unwrap_or(ScalarValue::String(s.clone())),
                DataType::Json => ScalarValue::Json(JsonValue::String(s.clone())),
                _ => ScalarValue::String(s.clone()),
            }
        }
        Some(Value::Array(arr)) => {
            if matches!(data_type, DataType::Json) {
                let items: Vec<JsonValue> = arr.iter().map(|v| value_to_json(v)).collect();
                ScalarValue::Json(JsonValue::Array(items))
            } else {
                ScalarValue::String(serde_json::to_string(value.unwrap()).unwrap_or_default())
            }
        }
        Some(Value::Object(obj)) => {
            if matches!(data_type, DataType::Json) {
                let pairs: Vec<(String, JsonValue)> = obj.iter()
                    .map(|(k, v)| (k.clone(), value_to_json(v)))
                    .collect();
                ScalarValue::Json(JsonValue::Object(pairs))
            } else {
                ScalarValue::String(serde_json::to_string(value.unwrap()).unwrap_or_default())
            }
        }
    }
}

fn value_to_json(value: &serde_json::Value) -> types::JsonValue {
    use types::JsonValue;
    match value {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Bool(*b),
        serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsonValue::String(s.clone()),
        serde_json::Value::Array(arr) => JsonValue::Array(arr.iter().map(value_to_json).collect()),
        serde_json::Value::Object(obj) => {
            JsonValue::Object(obj.iter().map(|(k, v)| (k.clone(), value_to_json(v))).collect())
        }
    }
}

fn create_vector_for_type(data_type: &DataType) -> Vector {
    match data_type {
        DataType::Boolean => Vector::Boolean(BooleanVector::new()),
        DataType::Int8 => Vector::Int8(Int8Vector::new()),
        DataType::Int16 => Vector::Int16(Int16Vector::new()),
        DataType::Int32 => Vector::Int32(Int32Vector::new()),
        DataType::Int64 => Vector::Int64(Int64Vector::new()),
        DataType::Int128 => Vector::Int128(Int128Vector::new()),
        DataType::Float32 => Vector::Float32(Float32Vector::new()),
        DataType::Float64 => Vector::Float64(Float64Vector::new()),
        DataType::Date => Vector::Date(DateVector::new()),
        DataType::DateTime => Vector::DateTime(DateTimeVector::new()),
        DataType::Json => Vector::Json(types::JsonVector::new()),
        _ => Vector::String(StringVector::new()),
    }
}

fn push_scalar_to_vector(vector: &mut Vector, scalar: &types::ScalarValue) {
    use types::ScalarValue;

    match scalar {
        ScalarValue::Boolean(b) => {
            if let Vector::Boolean(v) = vector {
                v.push(Some(*b));
            }
        }
        ScalarValue::Int8(n) => {
            if let Vector::Int8(v) = vector {
                v.push(Some(*n));
            }
        }
        ScalarValue::Int16(n) => {
            if let Vector::Int16(v) = vector {
                v.push(Some(*n));
            }
        }
        ScalarValue::Int32(n) => {
            if let Vector::Int32(v) = vector {
                v.push(Some(*n));
            }
        }
        ScalarValue::Int64(n) => {
            if let Vector::Int64(v) = vector {
                v.push(Some(*n));
            }
        }
        ScalarValue::Int128(n) => {
            if let Vector::Int128(v) = vector {
                v.push(Some(*n));
            }
        }
        ScalarValue::Float32(f) => {
            if let Vector::Float32(v) = vector {
                v.push(Some(*f));
            }
        }
        ScalarValue::Float64(f) => {
            if let Vector::Float64(v) = vector {
                v.push(Some(*f));
            }
        }
        ScalarValue::Date(d) => {
            if let Vector::Date(v) = vector {
                v.push(Some(*d));
            }
        }
        ScalarValue::DateTime(d) => {
            if let Vector::DateTime(v) = vector {
                v.push(Some(*d));
            }
        }
        ScalarValue::String(s) => {
            if let Vector::String(v) = vector {
                v.push(Some(s.as_str()));
            }
        }
        ScalarValue::Json(j) => {
            if let Vector::Json(v) = vector {
                v.push(Some(ScalarValue::Json(j.clone())));
            }
        }
        ScalarValue::Null | ScalarValue::Binary(_) | ScalarValue::Array(_) => {
            push_null_to_vector(vector);
        }
    }
}

fn push_null_to_vector(vector: &mut Vector) {
    match vector {
        Vector::Boolean(v) => v.push(None),
        Vector::Int8(v) => v.push(None),
        Vector::Int16(v) => v.push(None),
        Vector::Int32(v) => v.push(None),
        Vector::Int64(v) => v.push(None),
        Vector::Int128(v) => v.push(None),
        Vector::Float32(v) => v.push(None),
        Vector::Float64(v) => v.push(None),
        Vector::Date(v) => v.push(None),
        Vector::DateTime(v) => v.push(None),
        Vector::String(v) => v.push(None),
        Vector::Json(v) => v.push(None),
        Vector::Null(_) => {}
    }
}

fn parse_date_scalar(value: &str) -> Option<i32> {
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
            return Some(date.ordinal().try_into().unwrap_or(0));
        }
    }
    None
}

fn parse_datetime_scalar(value: &str) -> Option<i64> {
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
            return Some(dt.and_utc().timestamp_millis());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_json() {
        let json = serde_json::json!({"a": {"b": 1}});
        let flat = {
            let mut result = serde_json::Map::new();
            fn recurse(v: &serde_json::Value, prefix: String, res: &mut serde_json::Map<String, serde_json::Value>) {
                match v {
                    serde_json::Value::Object(map) => {
                        for (key, val) in map {
                            let new_key = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                            recurse(val, new_key, res);
                        }
                    }
                    serde_json::Value::Array(arr) => { res.insert(prefix, serde_json::Value::String(serde_json::to_string(arr).unwrap_or_default())); }
                    _ => { res.insert(prefix, v.clone()); }
                }
            }
            recurse(&json, String::new(), &mut result);
            result
        };
        assert_eq!(flat.get("a.b").unwrap(), &serde_json::json!(1));
    }

    #[test]
    fn test_infer_type_from_json_value() {
        assert_eq!(infer_type_from_json_value(&serde_json::json!(123)), DataType::Int64);
        assert_eq!(infer_type_from_json_value(&serde_json::json!(3.14)), DataType::Float64);
        assert_eq!(infer_type_from_json_value(&serde_json::json!("hello")), DataType::String);
        assert_eq!(infer_type_from_json_value(&serde_json::json!(true)), DataType::Boolean);
    }
}