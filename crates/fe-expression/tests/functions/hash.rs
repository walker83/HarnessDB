//! # Hash Function Tests
//!
//! Tests for cryptographic hash functions: md5, sha1
//! All results verified against Apache Doris behavior.

use types::*;
use fe_expression::functions::FunctionRegistry;

#[test]
fn test_md5() {
    let registry = FunctionRegistry::new();

    // Test cases from RFC 1321
    let test_cases = vec![
        ("", "d41d8cd98f00b204e9800998ecf8427e"),
        ("hello", "5d41402abc4b2a76b9719d911017c592"),
        ("world", "7d793037a0760186574b0282f2f435e7"),
        ("test", "098f6bcd4621d373cade4e832627b4f6"),
    ];

    for (input, expected) in test_cases {
        let args = vec![string_vec(&[input])];
        let result = registry.call("md5", &args);

        assert!(matches!(result, Vector::String(_)));
        if let Vector::String(v) = result {
            assert_eq!(v.get(0).unwrap(), expected, "MD5 mismatch for input: {}", input);
        }
    }
}

#[test]
fn test_md5_batch() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["hello", "world", ""])];
    let result = registry.call("md5", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "5d41402abc4b2a76b9719d911017c592");
        assert_eq!(v.get(1).unwrap(), "7d793037a0760186574b0282f2f435e7");
        assert_eq!(v.get(2).unwrap(), "d41d8cd98f00b204e9800998ecf8427e");
    }
}

#[test]
fn test_sha1() {
    let registry = FunctionRegistry::new();

    let test_cases = vec![
        ("", "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
        ("hello", "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"),
        ("world", "7c211433f02071597741e6ff5a8ea34789abbf43"),
        ("test", "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"),
    ];

    for (input, expected) in test_cases {
        let args = vec![string_vec(&[input])];
        let result = registry.call("sha1", &args);

        assert!(matches!(result, Vector::String(_)));
        if let Vector::String(v) = result {
            assert_eq!(v.get(0).unwrap(), expected, "SHA1 mismatch for input: {}", input);
        }
    }
}

#[test]
fn test_sha1_batch() {
    let registry = FunctionRegistry::new();

    let args = vec![string_vec(&["hello", "world", ""])];
    let result = registry.call("sha1", &args);

    assert!(matches!(result, Vector::String(_)));
    if let Vector::String(v) = result {
        assert_eq!(v.get(0).unwrap(), "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
        assert_eq!(v.get(1).unwrap(), "7c211433f02071597741e6ff5a8ea34789abbf43");
        assert_eq!(v.get(2).unwrap(), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }
}

// Regression test: Hash consistency
#[test]
fn test_hash_consistency() {
    let registry = FunctionRegistry::new();

    // Same input should always produce same hash
    let args = vec![string_vec(&["consistent"])];

    let result1 = registry.call("md5", &args);
    let result2 = registry.call("md5", &args);

    if let (Vector::String(v1), Vector::String(v2)) = (result1, result2) {
        assert_eq!(v1.get(0).unwrap(), v2.get(0).unwrap());
    }
}

// Helper functions
fn string_vec(data: &[&str]) -> Vector {
    Vector::String(types::vector::StringVector::from_vec(data.to_vec()))
}
