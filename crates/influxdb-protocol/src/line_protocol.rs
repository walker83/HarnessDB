//! InfluxDB line protocol parser

use std::collections::HashMap;

/// Parsed line protocol point
#[derive(Debug, Clone)]
pub struct Point {
    pub measurement: String,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, FieldValue>,
    pub timestamp: Option<i64>,
}

/// Field value types
#[derive(Debug, Clone)]
pub enum FieldValue {
    Float(f64),
    Integer(i64),
    String(String),
    Boolean(bool),
}

/// Line protocol parser
pub struct LineProtocolParser;

impl LineProtocolParser {
    /// Parse line protocol string into points
    /// Format: measurement,tag1=value1,tag2=value2 field1=value1,field2=value2 timestamp
    pub fn parse(line: &str) -> Option<Point> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        // Split by first space (separates measurement+tags from fields+timestamp)
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            return None;
        }

        let measurement_tags = parts[0];
        let fields_timestamp = parts[1];

        // Parse measurement and tags
        let (measurement, tags) = Self::parse_measurement_tags(measurement_tags)?;

        // Parse fields and timestamp
        let (fields, timestamp) = Self::parse_fields_timestamp(fields_timestamp)?;

        Some(Point {
            measurement,
            tags,
            fields,
            timestamp,
        })
    }

    fn parse_measurement_tags(s: &str) -> Option<(String, HashMap<String, String>)> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.is_empty() {
            return None;
        }

        let measurement = parts[0].to_string();
        let mut tags = HashMap::new();

        for part in &parts[1..] {
            if let Some((key, value)) = part.split_once('=') {
                tags.insert(key.to_string(), value.to_string());
            }
        }

        Some((measurement, tags))
    }

    fn parse_fields_timestamp(s: &str) -> Option<(HashMap<String, FieldValue>, Option<i64>)> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let fields_str = parts[0];
        let timestamp = parts.get(1).and_then(|ts| ts.parse().ok());

        let mut fields = HashMap::new();
        for field in fields_str.split(',') {
            if let Some((key, value)) = field.split_once('=') {
                let field_value = Self::parse_field_value(value)?;
                fields.insert(key.to_string(), field_value);
            }
        }

        Some((fields, timestamp))
    }

    fn parse_field_value(s: &str) -> Option<FieldValue> {
        if s.is_empty() {
            return None;
        }

        // Boolean
        if s == "t" || s == "T" || s == "true" || s == "True" {
            return Some(FieldValue::Boolean(true));
        }
        if s == "f" || s == "F" || s == "false" || s == "False" {
            return Some(FieldValue::Boolean(false));
        }

        // String (quoted)
        if s.starts_with('"') && s.ends_with('"') {
            return Some(FieldValue::String(s[1..s.len()-1].to_string()));
        }

        // Integer (ends with 'i')
        if s.ends_with('i') {
            if let Ok(n) = s[..s.len()-1].parse::<i64>() {
                return Some(FieldValue::Integer(n));
            }
        }

        // Float
        if let Ok(f) = s.parse::<f64>() {
            return Some(FieldValue::Float(f));
        }

        None
    }
}

/// Format points to line protocol
pub fn format_line_protocol(points: &[Point]) -> String {
    let mut lines = Vec::new();

    for point in points {
        let mut line = point.measurement.clone();

        // Add tags
        if !point.tags.is_empty() {
            let tags: Vec<String> = point.tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            line.push(',');
            line.push_str(&tags.join(","));
        }

        line.push(' ');

        // Add fields
        let fields: Vec<String> = point.fields
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    FieldValue::Float(f) => f.to_string(),
                    FieldValue::Integer(i) => format!("{}i", i),
                    FieldValue::String(s) => format!("\"{}\"", s),
                    FieldValue::Boolean(b) => if *b { "t" } else { "f" }.to_string(),
                };
                format!("{}={}", k, value)
            })
            .collect();
        line.push_str(&fields.join(","));

        // Add timestamp
        if let Some(ts) = point.timestamp {
            line.push(' ');
            line.push_str(&ts.to_string());
        }

        lines.push(line);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let line = "cpu,host=server01 value=0.64 1434055562000000000";
        let point = LineProtocolParser::parse(line).unwrap();
        assert_eq!(point.measurement, "cpu");
        assert_eq!(point.tags.get("host").unwrap(), "server01");
        assert!(matches!(point.fields.get("value"), Some(FieldValue::Float(_))));
        assert_eq!(point.timestamp, Some(1434055562000000000));
    }

    #[test]
    fn test_parse_multiple_fields() {
        let line = "weather,location=us temperature=82,humidity=71";
        let point = LineProtocolParser::parse(line).unwrap();
        assert_eq!(point.measurement, "weather");
        assert_eq!(point.fields.len(), 2);
    }

    #[test]
    fn test_parse_integer() {
        let line = "disk free=123456i";
        let point = LineProtocolParser::parse(line).unwrap();
        assert!(matches!(point.fields.get("free"), Some(FieldValue::Integer(123456))));
    }
}
