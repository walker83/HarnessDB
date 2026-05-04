//! # Math Function Tests
//!
//! Tests for extended mathematical functions: sign, degrees, radians, truncate,
//! greatest, least, modulo, cot, sinh, cosh, tanh
//! All results verified against Apache Doris behavior.

use types::*;
use fe_expression::functions::FunctionRegistry;

#[test]
fn test_sign() {
    let registry = FunctionRegistry::new();

    let args = vec![float64_vec(vec![-5.0, 0.0, 5.0])];
    let result = registry.call("sign", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[-1, 0, 1]);
    }
}

#[test]
fn test_degrees() {
    let registry = FunctionRegistry::new();

    let args = vec![float64_vec(vec![
        std::f64::consts::PI,
        2.0 * std::f64::consts::PI
    ])];
    let result = registry.call("degrees", &args);

    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert!((v.data()[0] - 180.0).abs() < 0.001);
        assert!((v.data()[1] - 360.0).abs() < 0.001);
    }
}

#[test]
fn test_radians() {
    let registry = FunctionRegistry::new();

    let args = vec![float64_vec(vec![180.0, 90.0])];
    let result = registry.call("radians", &args);

    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert!((v.data()[0] - std::f64::consts::PI).abs() < 0.001);
        assert!((v.data()[1] - std::f64::consts::PI / 2.0).abs() < 0.001);
    }
}

#[test]
fn test_truncate() {
    let registry = FunctionRegistry::new();

    // Test with decimal places
    let args = vec![
        float64_vec(vec![5.1234, 4.5678, -1.5]),
        int64_vec(vec![2, 1, 0]),
    ];
    let result = registry.call("truncate", &args);

    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert_eq!(v.data(), &[5.12, 4.5, -1.0]);
    }

    // Test without decimal places (default to 0)
    let args = vec![float64_vec(vec![3.9, -2.7, 1.5])];
    let result = registry.call("truncate", &args);
    if let Vector::Float64(v) = result {
        assert_eq!(v.data(), &[3.0, -2.0, 1.0]);
    }
}

#[test]
fn test_greatest() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![1, 5, 3]),
        int64_vec(vec![2, 4, 6]),
        int64_vec(vec![3, 3, 3]),
    ];
    let result = registry.call("greatest", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[3, 5, 6]);
    }
}

#[test]
fn test_least() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![1, 5, 3]),
        int64_vec(vec![2, 4, 6]),
        int64_vec(vec![3, 3, 3]),
    ];
    let result = registry.call("least", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[1, 3, 3]);
    }
}

#[test]
fn test_modulo() {
    let registry = FunctionRegistry::new();

    // Integer modulo
    let args = vec![
        int64_vec(vec![10, 20, 30]),
        int64_vec(vec![3, 7, 5]),
    ];
    let result = registry.call("modulo", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        assert_eq!(v.data(), &[1, 6, 0]);
    }

    // Float modulo
    let args = vec![
        float64_vec(vec![10.5, 20.3]),
        float64_vec(vec![3.0, 4.0]),
    ];
    let result = registry.call("modulo", &args);

    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert!((v.data()[0] - 1.5).abs() < 0.001);
        assert!((v.data()[1] - 0.3).abs() < 0.001);
    }
}

#[test]
fn test_cot() {
    let registry = FunctionRegistry::new();

    let args = vec![float64_vec(vec![std::f64::consts::PI / 4.0])];
    let result = registry.call("cot", &args);

    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert!((v.data()[0] - 1.0).abs() < 0.001);
    }
}

#[test]
fn test_hyperbolic_functions() {
    let registry = FunctionRegistry::new();

    // sinh
    let args = vec![float64_vec(vec![0.0, 1.0])];
    let result = registry.call("sinh", &args);
    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert_eq!(v.data()[0], 0.0);
        assert!(v.data()[1] > 1.175);
    }

    // cosh
    let result = registry.call("cosh", &args);
    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert_eq!(v.data()[0], 1.0);
        assert!(v.data()[1] > 1.543);
    }

    // tanh
    let result = registry.call("tanh", &args);
    assert!(matches!(result, Vector::Float64(_)));
    if let Vector::Float64(v) = result {
        assert_eq!(v.data()[0], 0.0);
        assert!(v.data()[1] > 0.76 && v.data()[1] < 0.77);
    }
}

// Regression test: Division by zero in modulo
#[test]
fn test_modulo_division_by_zero() {
    let registry = FunctionRegistry::new();

    let args = vec![
        int64_vec(vec![10, 20]),
        int64_vec(vec![0, 0]),
    ];
    let result = registry.call("modulo", &args);

    assert!(matches!(result, Vector::Int64(_)));
    if let Vector::Int64(v) = result {
        // Should return 0 for division by zero
        assert_eq!(v.data(), &[0, 0]);
    }
}

// Helper functions
fn int64_vec(data: Vec<i64>) -> Vector {
    Vector::Int64(types::vector::Int64Vector::from_vec(data))
}

fn float64_vec(data: Vec<f64>) -> Vector {
    Vector::Float64(types::vector::Float64Vector::from_vec(data))
}
