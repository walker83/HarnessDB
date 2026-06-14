// All integration test suites live in this directory as submodules. They are
// gathered here and re-exported via tests/main.rs so that Cargo compiles the
// entire integration test crate as a SINGLE test binary instead of one binary
// per file (19 binaries -> 1). Each `#[tokio::test]` function is still
// discovered and run independently by the test harness.

pub mod builtin_functions_test;
pub mod ddl_lifecycle_test;
pub mod e2e_aggregate_tests;
pub mod e2e_datetime_tests;
pub mod e2e_ddl_tests;
pub mod e2e_dml_tests;
pub mod e2e_doris_compat_test;
pub mod e2e_doris_syntax_tests;
pub mod e2e_edge_case_tests;
pub mod e2e_join_tests;
pub mod e2e_math_tests;
pub mod e2e_null_type_tests;
pub mod e2e_select_basic_tests;
pub mod e2e_string_tests;
pub mod e2e_subquery_tests;
pub mod e2e_window_tests;
pub mod mysql_protocol_test;
pub mod sql_query_test;
pub mod sql_test;
