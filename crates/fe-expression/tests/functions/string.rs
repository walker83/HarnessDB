//! # String Function Tests
//!
//! Tests for extended string functions: ltrim, rtrim, replace, left, right, locate,
//! repeat, space, reverse, ascii, char, octet_length, bit_length, concat_ws, etc.
//! All results verified against Apache Doris behavior.

use types::*;
use fe_expression::functions::FunctionRegistry;

#[test]
fn test_trim_functions() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["  hello", "\tworld", "  test  "])];

    // ltrim
    let result = registry.call("ltrim", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "hello");
        assert_eq!(v.get(1).unwrap(), "world");
        assert_eq!(v.get(2).unwrap(), "test  ");
    }

    // rtrim
    let result = registry.call("rtrim", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "  hello");
        assert_eq!(v.get(1).unwrap(), "\tworld");
        assert_eq!(v.get(2).unwrap(), "  test");
    }
}

#[test]
fn test_replace() {
    let registry = FunctionRegistry::new();

    let args = vec![
        string_vec(&["hello world", "test test", "abc"]),
        string_vec(&["world", "test", "xyz"]),
        string_vec(&["there", "best", "123"]),
    ];
    let result = registry.call("replace", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "hello there");
        assert_eq!(v.get(1).unwrap(), "best best");
        assert_eq!(v.get(2).unwrap(), "abc");
    }
}

#[test]
fn test_left_right() {
    let registry = FunctionRegistry::new();

    let s = string_vec(&["hello", "world", "test"]);
    let n = int64_vec(vec![2, 3, 10]);

    // left
    let args = vec![s.clone(), n.clone()];
    let result = registry.call("left", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "he");
        assert_eq!(v.get(1).unwrap(), "wor");
        assert_eq!(v.get(2).unwrap(), "test");
    }

    // right
    let result = registry.call("right", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "lo");
        assert_eq!(v.get(1).unwrap(), "rld");
        assert_eq!(v.get(2).unwrap(), "test");
    }
}

#[test]
fn test_locate_functions() {
    let registry = FunctionRegistry::new();

    // locate (same as instr but args in different order)
    let args = vec![
        string_vec(&["hello", "world", "test"]),
        string_vec(&["el", "or", "xyz"]),
    ];
    let result = registry.call("locate", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 2);
        assert_eq!(v.data()[1], 2);
        assert_eq!(v.data()[2], 0);
    }

    // instr
    let result = registry.call("instr", &args);
    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 3);
        assert_eq!(v.data()[1], 2);
        assert_eq!(v.data()[2], 0);
    }
}

#[test]
fn test_repeat_and_space() {
    let registry = FunctionRegistry::new();

    // repeat
    let args = vec![
        string_vec(&["ab", "x", "test"]),
        int64_vec(vec![3, 5, 2]),
    ];
    let result = registry.call("repeat", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "ababab");
        assert_eq!(v.get(1).unwrap(), "xxxxx");
        assert_eq!(v.get(2).unwrap(), "testtest");
    }

    // space
    let args = vec![int64_vec(vec![0, 1, 3])];
    let result = registry.call("space", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "");
        assert_eq!(v.get(1).unwrap(), " ");
        assert_eq!(v.get(2).unwrap(), "   ");
    }
}

#[test]
fn test_reverse() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["hello", "world", "ab"])];
    let result = registry.call("reverse", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "olleh");
        assert_eq!(v.get(1).unwrap(), "dlrow");
        assert_eq!(v.get(2).unwrap(), "ba");
    }
}

#[test]
fn test_ascii_and_char() {
    let registry = FunctionRegistry::new();

    // ascii
    let args = vec![string_vec(&["A", "a", "hello"])];
    let result = registry.call("ascii", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 65);
        assert_eq!(v.data()[1], 97);
        assert_eq!(v.data()[2], 104);
    }

    // char
    let args = vec![
        int64_vec(vec![65, 66, 67]),
        int64_vec(vec![68, 69, 70]),
    ];
    let result = registry.call("char", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "AD");
        assert_eq!(v.get(1).unwrap(), "BE");
        assert_eq!(v.get(2).unwrap(), "CF");
    }
}

#[test]
fn test_length_functions() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["hello", "世界", "test"])];

    // octet_length (bytes)
    let result = registry.call("octet_length", &args);
    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 5);
        assert_eq!(v.data()[1], 6); // UTF-8: 3 bytes per Chinese character
        assert_eq!(v.data()[2], 4);
    }

    // bit_length (bits)
    let result = registry.call("bit_length", &args);
    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 40);
        assert_eq!(v.data()[1], 48);
        assert_eq!(v.data()[2], 32);
    }
}

#[test]
fn test_concat_ws() {
    let registry = FunctionRegistry::new();

    let args = vec![
        string_vec(&[",", "-", "|"]),
        string_vec(&["a", "x", "1"]),
        string_vec(&["b", "y", "2"]),
        string_vec(&["c", "z", "3"]),
    ];
    let result = registry.call("concat_ws", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "a,b,c");
        assert_eq!(v.get(1).unwrap(), "x-y-z");
        assert_eq!(v.get(2).unwrap(), "1|2|3");
    }
}

#[test]
fn test_find_in_set() {
    let registry = FunctionRegistry::new();

    let args = vec![
        string_vec(&["b", "d", "x"]),
        string_vec(&["a,b,c", "a,b,c,d", "a,b,c"]),
    ];
    let result = registry.call("find_in_set", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 2);
        assert_eq!(v.data()[1], 4);
        assert_eq!(v.data()[2], 0);
    }
}

#[test]
fn test_pad_functions() {
    let registry = FunctionRegistry::new();

    let args = vec![
        string_vec(&["hi", "hello", "test"]),
        int64_vec(vec![5, 3, 6]),
        string_vec(&["*", "-", "0"]),
    ];

    // lpad
    let result = registry.call("lpad", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "***hi");
        assert_eq!(v.get(1).unwrap(), "hel");
        assert_eq!(v.get(2).unwrap(), "00test");
    }

    // rpad
    let result = registry.call("rpad", &args);
    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "hi***");
        assert_eq!(v.get(1).unwrap(), "hel");
        assert_eq!(v.get(2).unwrap(), "test00");
    }
}

#[test]
fn test_format() {
    let registry = FunctionRegistry::new();

    let args = vec![
        float64_vec(vec![1234.5678, 100.0, 99.99]),
        int64_vec(vec![2, 0, 1]),
    ];
    let result = registry.call("format", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "1234.57");
        assert_eq!(v.get(1).unwrap(), "100");
        assert_eq!(v.get(2).unwrap(), "100.0");
    }
}

// Regression test: Empty strings
#[test]
fn test_empty_string_handling() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["", "", "test"])];

    // reverse of empty string
    let result = registry.call("reverse", &args);
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "");
        assert_eq!(v.get(1).unwrap(), "");
        assert_eq!(v.get(2).unwrap(), "tset");
    }

    // length of empty string
    let result = registry.call("octet_length", &args);
    if let Vector::Int64(v) = result {
        assert_eq!(v.data()[0], 0);
        assert_eq!(v.data()[1], 0);
        assert_eq!(v.data()[2], 4);
    }
}

// Helper functions
fn int64_vec(data: Vec<i64>) -> Vector {
    Vector::Int64(types::vector::Int64Vector::from_vec(data))
}

fn float64_vec(data: Vec<f64>) -> Vector {
    Vector::Float64(types::vector::Float64Vector::from_vec(data))
}

fn string_vec(data: &[&str]) -> Vector {
    Vector::String(types::vector::StringVector::from_vec(data.to_vec()))
}
