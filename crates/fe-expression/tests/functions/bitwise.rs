//! # Bitwise Function Tests
//!
//! Tests for bitwise operations: bitand, bitor, bitxor, bitnot, bitshiftleft, bitshiftright
//! All results verified against Apache Doris behavior.

use types::*;
use fe_expression::functions::FunctionRegistry;

#[test]
fn test_bitand() {
    let registry = FunctionRegistry::new();

    // Basic test: 5 & 3 = 1 (101 & 011 = 001)
    let args = vec![
        int64_vec(vec![5, 10, 15]),  // 101, 1010, 1111
        int64_vec(vec![3, 6, 7]),    // 011, 0110, 0111
    ];
    let result = registry.call("bitand", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[1, 2, 7]);
    }
}

#[test]
fn test_bitor() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![5, 10, 15]),
        int64_vec(vec![3, 6, 7]),
    ];
    let result = registry.call("bitor", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[7, 14, 15]);
    }
}

#[test]
fn test_bitxor() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![5, 10, 15]),
        int64_vec(vec![3, 6, 7]),
    ];
    let result = registry.call("bitxor", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[6, 12, 8]);
    }
}

#[test]
fn test_bitnot() {
    let registry = FunctionRegistry::new();

    let args = vec![int64_vec(vec![0, -1, 5])];
    let result = registry.call("bitnot", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[-1, 0, -6]);
    }
}

#[test]
fn test_bitshiftleft() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![1, 2, 3]),
        int64_vec(vec![1, 2, 3]),
    ];
    let result = registry.call("bitshiftleft", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[2, 8, 24]);
    }
}

#[test]
fn test_bitshiftright() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![8, 16, 24]),
        int64_vec(vec![1, 2, 3]),
    ];
    let result = registry.call("bitshiftright", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[4, 4, 3]);
    }
}

#[test]
fn test_bitwise_with_different_types() {
    let registry = FunctionRegistry::new();

    // Test with Int32
    let args = vec![
        Vector::Int32(types::vector::Int32Vector::from_vec(vec![5, 10])),
        Vector::Int32(types::vector::Int32Vector::from_vec(vec![3, 6])),
    ];
    let result = registry.call("bitand", &args);
    assert!(matches!(result, Vector::Int32(_)));

    // Test with Int16
    let args = vec![
        Vector::Int16(types::vector::Int16Vector::from_vec(vec![5, 10])),
        Vector::Int16(types::vector::Int16Vector::from_vec(vec![3, 6])),
    ];
    let result = registry.call("bitand", &args);
    assert!(matches!(result, Vector::Int16(_)));

    // Test with Int8
    let args = vec![
        Vector::Int8(types::vector::Int8Vector::from_vec(vec![5, 10])),
        Vector::Int8(types::vector::Int8Vector::from_vec(vec![3, 6])),
    ];
    let result = registry.call("bitand", &args);
    assert!(matches!(result, Vector::Int8(_)));
}

// Regression test: Ensure bitwise operations with edge cases
#[test]
fn test_bitwise_edge_cases() {
    let registry = FunctionRegistry::new();

    // Test with 0
    let args = vec![
        int64_vec(vec![0, 255, -1]),
        int64_vec(vec![255, 0, -1]),
    ];
    let result = registry.call("bitand", &args);
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[0, 0, -1]);
    }

    // Test shift by 0
    let args = vec![
        int64_vec(vec![5, 10]),
        int64_vec(vec![0, 0]),
    ];
    let result = registry.call("bitshiftleft", &args);
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[5, 10]);
    }
}

// Helper functions
fn int64_vec(data: Vec<i64>) -> Vector {
    Vector::Int64(types::vector::Int64Vector::from_vec(data))
}
