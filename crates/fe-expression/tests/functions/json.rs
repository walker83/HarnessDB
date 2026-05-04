//! # JSON Function Tests
//!
//! Tests for JSON functions: json_parse, json_query, json_get, json_contains,
//! json_array, json_object, json_length, json_keys, json_valid
//! All results verified against Apache Doris behavior.

use types::*;
use fe_expression::functions::FunctionRegistry;

// Test strings defined as variables to avoid syntax issues
const JSON_NUMBER: &str = "123";
const JSON_STRING: &str = r#""hello""#;
const JSON_BOOL: &str = "true";
const JSON_NULL: &str = "null";
const JSON_ARRAY: &str = "[1,2,3]";
const JSON_OBJECT: &str = r#"{"key":"value"}"#;
const JSON_OBJ_NESTED: &str = r#"{"name":"John","age":30}"#;
const JSON_INVALID: &str = "{invalid";
const JSON_INCOMPLETE: &str = "undefined";
const JSON_NOTJSON: &str = "notjson";
const JSON_ARR5: &str = r#"[1,2,3,4,5]"#;
const JSON_EMPTY: &str = r#"[]"#;
const JSON_SINGLE: &str = r#"[1]"#;
const JSON_OBJ_3KEYS: &str = r#"{"a":1,"b":2,"c":3}"#;
const JSON_EMPTY_OBJ: &str = r#"{}"#;
const JSON_ARR_3: &str = "[1,2,3]";
const JSON_OBJ_NAME: &str = r#"{"name":"John"}"#;
const JSON_STR_HELLO_WORLD: &str = r#""hello world""#;

#[test]
fn test_json_parse() {
    let registry = FunctionRegistry::new();

    let test_cases = vec![
        (JSON_NUMBER, "number"),
        (JSON_STRING, "string"),
        (JSON_BOOL, "boolean"),
        (JSON_NULL, "null"),
        (JSON_ARRAY, "array"),
        (JSON_OBJECT, "object"),
    ];

    for (input, desc) in test_cases {
        let args = vec![string_vec(&[input])];
        let result = registry.call("json_parse", &args);

        assert!(matches!(result, Vector::Json(_)), "Failed to parse: {} ({})", input, desc);
    }
}

#[test]
fn test_json_valid() {
    let registry = FunctionRegistry::new();

    let valid_json = vec![JSON_NUMBER, r#""test""#, JSON_BOOL, JSON_NULL, JSON_EMPTY, JSON_EMPTY_OBJ];
    let invalid_json = vec![JSON_INVALID, JSON_INCOMPLETE, r#"[1,2"#, "test"];

    // Test valid JSON
    let args = vec![string_vec(&valid_json)];
    let result = registry.call("json_valid", &args);
    assert!(matches!(result, Vector::Boolean(_)));
    if let Vector::Boolean(v) = result {
        for i in 0..valid_json.len() {
            assert!(v.get(i).unwrap(), "Should be valid: {}", valid_json[i]);
        }
    }

    // Test invalid JSON
    let args = vec![string_vec(&invalid_json)];
    let result = registry.call("json_valid", &args);
    assert!(matches!(result, Vector::Boolean(_)));
    if let Vector::Boolean(v) = result {
        for i in 0..invalid_json.len() {
            assert!(!v.get(i).unwrap(), "Should be invalid: {}", invalid_json[i]);
        }
    }
}

#[test]
fn test_json_query() {
    let registry = FunctionRegistry::new();

    let json_str = string_vec(&[JSON_OBJ_NESTED, JSON_OBJ_NESTED]);
    let path = string_vec(&["$.name", "$.age"]);

    let args = vec![json_str, path];
    let result = registry.call("json_query", &args);

    assert!(matches!(result, Vector::Json(_)));
    if let Vector::Json(v) = result {
        if let Some(ScalarValue::Json(JsonValue::String(s))) = v.get(0) {
            assert_eq!(s, "John");
        }
        if let Some(ScalarValue::Json(JsonValue::Number(n))) = v.get(1) {
            assert_eq!(n, 30.0);
        }
    }
}

#[test]
fn test_json_array() {
    let registry = FunctionRegistry::new();

    let args = vec![
        string_vec(&["a", "b"]),
        int64_vec(vec![1, 2]),
        float64_vec(vec![1.5, 2.5]),
    ];
    let result = registry.call("json_array", &args);

    assert!(matches!(result, Vector::Json(_)));
    if let Vector::Json(v) = result
        && let Some(ScalarValue::Json(JsonValue::Array(items))) = v.get(0) {
            assert_eq!(items.len(), 6);
        }
}

#[test]
fn test_json_object() {
    let registry = FunctionRegistry::new();

    let args = vec![
        Vector::String(types::vector::StringVector::from_vec(vec!["name", "age"])),
        Vector::String(types::vector::StringVector::from_vec(vec!["John", "30"])),
    ];
    let result = registry.call("json_object", &args);

    assert!(matches!(result, Vector::Json(_)));
}

#[test]
fn test_json_length() {
    let registry = FunctionRegistry::new();

    // Array length
    let args = vec![string_vec(&[JSON_ARR5, JSON_EMPTY, JSON_SINGLE])];
    let result = registry.call("json_length", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 5);
        assert_eq!(v.data()[1], 0);
        assert_eq!(v.data()[2], 1);
    }

    // Object length
    let args = vec![string_vec(&[JSON_OBJ_3KEYS, JSON_EMPTY_OBJ])];
    let result = registry.call("json_length", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 3);
        assert_eq!(v.data()[1], 0);
    }
}

#[test]
fn test_json_contains() {
    let registry = FunctionRegistry::new();

    let json = string_vec(&[JSON_OBJ_NAME, JSON_ARR_3, JSON_STR_HELLO_WORLD]);
    let target = string_vec(&["John", "2", "world"]);

    let args = vec![json, target];
    let result = registry.call("json_contains", &args);

    assert!(matches!(result, Vector::Boolean(_)));
    if let Vector::Boolean(v) = result {
        assert!(v.get(0).unwrap());
        assert!(v.get(1).unwrap());
        assert!(v.get(2).unwrap());
    }
}

// Regression test: Invalid JSON handling
#[test]
fn test_json_parse_invalid() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&[JSON_INVALID, JSON_INCOMPLETE, JSON_NOTJSON])];
    let result = registry.call("json_parse", &args);

    // Should return Json vectors (possibly with null for invalid JSON)
    assert!(matches!(result, Vector::Json(_)));
}

#[test]
fn test_json_keys() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&[JSON_OBJ_3KEYS, JSON_EMPTY_OBJ, JSON_ARR_3])];
    let result = registry.call("json_keys", &args);

    assert!(matches!(result, Vector::Json(_)));
    if let Vector::Json(v) = result {
        // First object has 3 keys
        if let Some(ScalarValue::Json(JsonValue::Array(keys))) = v.get(0) {
            assert_eq!(keys.len(), 3);
        }
        // Second object has 0 keys
        if let Some(ScalarValue::Json(JsonValue::Array(keys))) = v.get(1) {
            assert_eq!(keys.len(), 0);
        }
    }
}

// Helper functions
fn string_vec(data: &[&str]) -> Vector {
    Vector::String(types::vector::StringVector::from_vec(data.to_vec()))
}

fn int64_vec(data: Vec<i64>) -> Vector {
    Vector::Int64(types::vector::Int64Vector::from_vec(data))
}

fn float64_vec(data: Vec<f64>) -> Vector {
    Vector::Float64(types::vector::Float64Vector::from_vec(data))
}
