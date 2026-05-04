use integration_tests::common;
use types::ScalarValue;

// ===========================================================================
// 3.1 String functions
// ===========================================================================

#[test]
fn test_string_upper_lower() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT UPPER(name), LOWER(name) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_length() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT name, LENGTH(name) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_concat() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT CONCAT(name, ' works in ', department) AS info FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_substring() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT SUBSTRING(name, 1, 3) AS short_name FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_trim_ltrim_rtrim() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT TRIM(name), LTRIM(name), RTRIM(name) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_replace() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT REPLACE(department, 'Engineering', 'Eng') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_left_right() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT LEFT(name, 2), RIGHT(name, 2) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_lpad_rpad() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT LPAD(name, 10, '*'), RPAD(name, 10, '-') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_reverse_repeat() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT REVERSE(name), REPEAT(name, 2) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_string_locate_instr() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT LOCATE('li', name), INSTR(name, 'a') FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// 3.2 Math functions
// ===========================================================================

#[test]
fn test_math_abs_ceil_floor_round() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT ABS(-5), CEIL(3.2), FLOOR(3.8), ROUND(3.14159, 2) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_trunc() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT TRUNC(3.14159, 2) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_trigonometric() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT SIN(0), COS(0), TAN(0), ASIN(0), ACOS(1), ATAN(0) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_log_exp_sqrt_pow() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT LOG(2.718), LOG10(100), EXP(1), SQRT(16), POW(2, 10) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_mod_sign() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT MOD(10, 3), SIGN(-42) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_greatest_least() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT GREATEST(salary, 100000), LEAST(salary, 80000) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_math_rand() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT RAND(), RANDOM() FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// 3.3 Date functions
// ===========================================================================

#[test]
fn test_date_extract_components() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT YEAR('2024-01-15'), MONTH('2024-06-15'), DAY('2024-01-15') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_hour_minute_second() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT HOUR('2024-01-15 14:30:45'), MINUTE('2024-01-15 14:30:45'), SECOND('2024-01-15 14:30:45') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_datediff() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT DATEDIFF('2024-12-31', '2024-01-01') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_add_sub() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT DATE_ADD('2024-01-15', INTERVAL 30 DAY), DATE_SUB('2024-01-15', INTERVAL 1 MONTH) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_format() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT DATE_FORMAT('2024-01-15', '%Y-%m') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_now_curdate() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT NOW(), CURDATE(), CURRENT_TIMESTAMP, CURRENT_DATE FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_trunc() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT DATE_TRUNC('2024-01-15', MONTH) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_date_week_quarter() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT WEEK('2024-01-15'), QUARTER('2024-06-15'), MONTHNAME('2024-01-15'), DAYNAME('2024-01-15') FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// 3.4 JSON functions
// ===========================================================================

#[test]
fn test_json_parse() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_PARSE('{\"a\": 1}') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_json_extract() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_EXTRACT('{\"a\": 1, \"b\": 2}', '$.a') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_json_contains() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_CONTAINS('{\"a\": 1, \"b\": 2}', '1', '$.a') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_json_array_object() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_ARRAY(1, 2, 3), JSON_OBJECT('key', 'value') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_json_length_keys() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_LENGTH('{\"a\": 1, \"b\": 2}'), JSON_KEYS('{\"a\": 1, \"b\": 2}') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_json_valid() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT JSON_VALID('{\"a\": 1}'), JSON_VALID('not json') FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// 3.5 Conditional functions
// ===========================================================================

#[test]
fn test_case_when_simple() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql(
        "SELECT name, CASE WHEN salary > 100000 THEN 'high' WHEN salary > 75000 THEN 'medium' ELSE 'low' END AS salary_band FROM employees"
    );
    assert!(result.is_ok());
}

#[test]
fn test_case_when_with_expressions() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql(
        "SELECT department, CASE department WHEN 'Engineering' THEN 'Tech' WHEN 'Marketing' THEN 'Business' ELSE 'Other' END AS dept_type FROM employees"
    );
    assert!(result.is_ok());
}

#[test]
fn test_coalesce() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT COALESCE(department, 'Unassigned') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_ifnull_nullif() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT IFNULL(department, 'N/A'), NULLIF(department, 'Sales') FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_if_function() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT IF(salary > 80000, 'high', 'low') FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// 3.6 Bitwise functions
// ===========================================================================

#[test]
fn test_bitwise_and_or_xor() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql("SELECT id & 3, id | 4, id ^ 1 FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_bitwise_not() {
    let catalog = common::create_test_catalog();
    // BITNOT function form
    let result = fe_sql_parser::parse_sql("SELECT BITNOT(id) FROM employees");
    assert!(result.is_ok());
}

#[test]
fn test_bitwise_shift() {
    let catalog = common::create_test_catalog();
    // BITSHIFTL/BITSHIFTR function form
    let result = fe_sql_parser::parse_sql("SELECT BITSHIFTL(id, 2), BITSHIFTR(id, 1) FROM employees");
    assert!(result.is_ok());
}

// ===========================================================================
// Function combination tests
// ===========================================================================

#[test]
fn test_nested_functions() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql(
        "SELECT UPPER(SUBSTRING(name, 1, 3)), ROUND(AVG(salary), 2) FROM employees GROUP BY name"
    );
    assert!(result.is_ok());
}

#[test]
fn test_functions_in_where() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql(
        "SELECT * FROM employees WHERE UPPER(department) = 'ENGINEERING' AND YEAR(NOW()) = 2024"
    );
    assert!(result.is_ok());
}

#[test]
fn test_functions_in_having() {
    let catalog = common::create_test_catalog();
    let result = fe_sql_parser::parse_sql(
        "SELECT department, SUM(salary) AS total FROM employees GROUP BY department HAVING SUM(salary) > 100000"
    );
    assert!(result.is_ok());
}

// ===========================================================================
// Block-level expression evaluation
// ===========================================================================

#[test]
fn test_block_string_operations() {
    let block = common::create_employees_block();
    let name_col = block.column_by_name("name").unwrap().1;

    // Simulate UPPER on first value
    if let ScalarValue::String(name) = name_col.scalar_at(0) {
        assert_eq!(name.to_uppercase(), "ALICE");
    }

    // Simulate LENGTH
    if let ScalarValue::String(name) = name_col.scalar_at(0) {
        assert_eq!(name.len(), 5);
    }
}

#[test]
fn test_block_math_operations() {
    let block = common::create_employees_block();
    let salary_col = block.column_by_name("salary").unwrap().1;

    // Test ABS, CEIL, FLOOR on salary values
    for i in 0..block.num_rows() {
        if let ScalarValue::Float64(v) = salary_col.scalar_at(i) {
            assert!(v > 0.0); // ABS is identity for positive
            assert_eq!(v.ceil(), v); // All are integers
            assert_eq!(v.floor(), v);
        }
    }
}
