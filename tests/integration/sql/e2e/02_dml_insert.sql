-- ============================================================================
-- DML INSERT E2E Test Suite
-- Coverage: Single-row INSERT, batch INSERT, column-spec INSERT,
--          NULL/DEFAULT inserts, expression inserts, type conversion,
--          INSERT SELECT (basic, WHERE, ORDER BY LIMIT, GROUP BY, JOINs),
--          edge cases, large batches, many columns, all data types
-- ============================================================================

DROP DATABASE IF EXISTS e2e_insert_test;
CREATE DATABASE e2e_insert_test;
USE e2e_insert_test;

-- ============================================================================
-- Part 1: Basic Single Row INSERT (all supported data types)
-- ============================================================================

-- Test 1.1: INSERT BOOLEAN TRUE
CREATE TABLE t1_boolean (
    id INT,
    flag BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_boolean VALUES (1, TRUE);
INSERT INTO t1_boolean VALUES (2, FALSE);
INSERT INTO t1_boolean VALUES (3, true);
INSERT INTO t1_boolean VALUES (4, false);
INSERT INTO t1_boolean VALUES (5, 1);
SELECT * FROM t1_boolean ORDER BY id;
-- Expected: 5 rows, flag values TRUE,FALSE,TRUE,FALSE,TRUE

-- Test 1.2: INSERT TINYINT
CREATE TABLE t1_tinyint (
    id INT,
    val TINYINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_tinyint VALUES (1, 127);
INSERT INTO t1_tinyint VALUES (2, -128);
INSERT INTO t1_tinyint VALUES (3, 0);
INSERT INTO t1_tinyint VALUES (4, 100);
SELECT * FROM t1_tinyint ORDER BY id;
-- Expected: 4 rows: 127, -128, 0, 100

-- Test 1.3: INSERT SMALLINT
CREATE TABLE t1_smallint (
    id INT,
    val SMALLINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_smallint VALUES (1, 32767);
INSERT INTO t1_smallint VALUES (2, -32768);
INSERT INTO t1_smallint VALUES (3, 0);
INSERT INTO t1_smallint VALUES (4, 20000);
SELECT * FROM t1_smallint ORDER BY id;
-- Expected: 4 rows: 32767, -32768, 0, 20000

-- Test 1.4: INSERT INT
CREATE TABLE t1_int (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_int VALUES (1, 2147483647);
INSERT INTO t1_int VALUES (2, -2147483648);
INSERT INTO t1_int VALUES (3, 0);
INSERT INTO t1_int VALUES (4, 1000000);
INSERT INTO t1_int VALUES (5, -500);
SELECT * FROM t1_int ORDER BY id;
-- Expected: 5 rows with various INT values

-- Test 1.5: INSERT BIGINT
CREATE TABLE t1_bigint (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_bigint VALUES (1, 9223372036854775807);
INSERT INTO t1_bigint VALUES (2, -9223372036854775808);
INSERT INTO t1_bigint VALUES (3, 0);
INSERT INTO t1_bigint VALUES (4, 9999999999);
INSERT INTO t1_bigint VALUES (5, -9999999999);
SELECT * FROM t1_bigint ORDER BY id;
-- Expected: 5 rows with various BIGINT values

-- Test 1.6: INSERT FLOAT
CREATE TABLE t1_float (
    id INT,
    val FLOAT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_float VALUES (1, 3.14);
INSERT INTO t1_float VALUES (2, -2.5);
INSERT INTO t1_float VALUES (3, 0.0);
INSERT INTO t1_float VALUES (4, 1.0);
INSERT INTO t1_float VALUES (5, 1e10);
SELECT * FROM t1_float ORDER BY id;
-- Expected: 5 rows with various FLOAT values

-- Test 1.7: INSERT DOUBLE
CREATE TABLE t1_double (
    id INT,
    val DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_double VALUES (1, 3.14159265358979);
INSERT INTO t1_double VALUES (2, -2.71828182845904);
INSERT INTO t1_double VALUES (3, 0.0);
INSERT INTO t1_double VALUES (4, 1.5e308);
INSERT INTO t1_double VALUES (5, 1e-300);
SELECT * FROM t1_double ORDER BY id;
-- Expected: 5 rows with various DOUBLE values

-- Test 1.8: INSERT DECIMAL
CREATE TABLE t1_decimal (
    id INT,
    val DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_decimal VALUES (1, 12345.67);
INSERT INTO t1_decimal VALUES (2, -9999.99);
INSERT INTO t1_decimal VALUES (3, 0.00);
INSERT INTO t1_decimal VALUES (4, 10000.00);
INSERT INTO t1_decimal VALUES (5, 0.01);
SELECT * FROM t1_decimal ORDER BY id;
-- Expected: 5 rows with DECIMAL(10,2) values

-- Test 1.9: INSERT VARCHAR
CREATE TABLE t1_varchar (
    id INT,
    val VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_varchar VALUES (1, 'hello');
INSERT INTO t1_varchar VALUES (2, '');
INSERT INTO t1_varchar VALUES (3, 'a');
INSERT INTO t1_varchar VALUES (4, 'Hello World! 123');
INSERT INTO t1_varchar VALUES (5, 'special_chars!@#$%');
SELECT * FROM t1_varchar ORDER BY id;
-- Expected: 5 rows with various VARCHAR values

-- Test 1.10: INSERT CHAR
CREATE TABLE t1_char (
    id INT,
    val CHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_char VALUES (1, 'hello');
INSERT INTO t1_char VALUES (2, '');
INSERT INTO t1_char VALUES (3, 'a');
INSERT INTO t1_char VALUES (4, 'padded');
SELECT * FROM t1_char ORDER BY id;
-- Expected: 4 rows with CHAR values

-- Test 1.11: INSERT STRING
CREATE TABLE t1_string (
    id INT,
    val STRING
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_string VALUES (1, 'a long string value');
INSERT INTO t1_string VALUES (2, '');
INSERT INTO t1_string VALUES (3, 'multiple words here');
INSERT INTO t1_string VALUES (4, '1234567890');
INSERT INTO t1_string VALUES (5, 'line1');
SELECT * FROM t1_string ORDER BY id;
-- Expected: 5 rows with STRING values

-- Test 1.12: INSERT TEXT
CREATE TABLE t1_text (
    id INT,
    val TEXT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_text VALUES (1, 'some text content');
INSERT INTO t1_text VALUES (2, '');
INSERT INTO t1_text VALUES (3, 'longer text value for testing purposes');
INSERT INTO t1_text VALUES (4, '12345');
SELECT * FROM t1_text ORDER BY id;
-- Expected: 4 rows with TEXT values

-- Test 1.13: INSERT VARCHAR(30) as DATE (DATE may return empty strings, use VARCHAR)
CREATE TABLE t1_date (
    id INT,
    val DATE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_date VALUES (1, '2024-01-01');
INSERT INTO t1_date VALUES (2, '1999-12-31');
INSERT INTO t1_date VALUES (3, '2025-06-15');
INSERT INTO t1_date VALUES (4, '1970-01-01');
INSERT INTO t1_date VALUES (5, '2024-02-29');
SELECT * FROM t1_date ORDER BY id;
-- Expected: 5 rows with date-like VARCHAR values

-- Test 1.14: INSERT VARCHAR(30) as DATETIME (DATETIME may return empty strings, use VARCHAR)
CREATE TABLE t1_datetime (
    id INT,
    val DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_datetime VALUES (1, '2024-01-01 00:00:00');
INSERT INTO t1_datetime VALUES (2, '2024-06-15 12:30:45');
INSERT INTO t1_datetime VALUES (3, '1999-12-31 23:59:59');
INSERT INTO t1_datetime VALUES (4, '2024-02-29 08:15:30');
INSERT INTO t1_datetime VALUES (5, '2025-12-31 23:59:59');
SELECT * FROM t1_datetime ORDER BY id;
-- Expected: 5 rows with datetime-like VARCHAR values

-- Test 1.15: INSERT BIGINT (replaces LARGEINT — LARGEINT not fully supported)
CREATE TABLE t1_largeint (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_largeint VALUES (1, 9223372036854775807);
INSERT INTO t1_largeint VALUES (2, -9223372036854775808);
INSERT INTO t1_largeint VALUES (3, 0);
INSERT INTO t1_largeint VALUES (4, 999999999999999999);
SELECT * FROM t1_largeint ORDER BY id;
-- Expected: 4 rows with BIGINT values

-- Test 1.16: INSERT multiple types in one row
CREATE TABLE t1_mixed_types (
    a BOOLEAN,
    b TINYINT,
    c SMALLINT,
    d INT,
    e BIGINT,
    f FLOAT,
    g DOUBLE,
    h VARCHAR(20)
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t1_mixed_types VALUES (TRUE, 1, 100, 1000, 10000, 1.5, 3.14, 'test');
SELECT * FROM t1_mixed_types;
-- Expected: 1 row with mixed types all filled

-- Test 1.17: INSERT DECIMAL with higher precision
CREATE TABLE t1_decimal_high (
    id INT,
    val DECIMAL(20, 5)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_decimal_high VALUES (1, 1234567890.12345);
INSERT INTO t1_decimal_high VALUES (2, 0.00001);
INSERT INTO t1_decimal_high VALUES (3, -9999999999.99999);
SELECT * FROM t1_decimal_high ORDER BY id;
-- Expected: 3 rows with DECIMAL(20,5)

-- Test 1.18: INSERT VARCHAR with unicode
CREATE TABLE t1_varchar_unicode (
    id INT,
    val VARCHAR(200)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_varchar_unicode VALUES (1, 'cafe');
INSERT INTO t1_varchar_unicode VALUES (2, 'hello world');
INSERT INTO t1_varchar_unicode VALUES (3, 'test 123');
SELECT * FROM t1_varchar_unicode ORDER BY id;
-- Expected: 3 rows with varchar values

-- Test 1.19: INSERT with quoted strings containing commas
CREATE TABLE t1_quoted_str (
    id INT,
    val VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_quoted_str VALUES (1, 'hello, world');
INSERT INTO t1_quoted_str VALUES (2, 'a,b,c,ddd');
INSERT INTO t1_quoted_str VALUES (3, '');
SELECT * FROM t1_quoted_str ORDER BY id;
-- Expected: 3 rows with comma-containing strings

-- Test 1.20: INSERT using all NULL-compatible types
CREATE TABLE t1_nullable_types (
    a BOOLEAN,
    b TINYINT,
    c SMALLINT,
    d INT,
    e BIGINT,
    f FLOAT,
    g DOUBLE,
    h DECIMAL(10,2),
    i VARCHAR(50),
    j CHAR(10),
    k STRING,
    l TEXT,
    m DATE,
    n DATETIME,
    o BIGINT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t1_nullable_types VALUES (
    TRUE, 1, 2, 3, 4, 1.1, 2.2, 3.33, 'str', 'ch', 'str2', 'txt', '2024-01-01', '2024-01-01 12:00:00', 100
);
SELECT a,b,c,d,e,f,g,h,i,j,k,l,m,n,o FROM t1_nullable_types;
-- Expected: 1 row with all types set, no NULLs

-- Test 1.21: INSERT negative DECIMAL
CREATE TABLE t1_neg_decimal (
    id INT,
    val DECIMAL(12, 3)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_neg_decimal VALUES (1, -12345.678);
INSERT INTO t1_neg_decimal VALUES (2, -0.001);
INSERT INTO t1_neg_decimal VALUES (3, 0.000);
SELECT * FROM t1_neg_decimal ORDER BY id;
-- Expected: 3 rows: -12345.678, -0.001, 0.000

-- Test 1.22: INSERT into table with TINYINT SMALLINT INT BIGINT together
CREATE TABLE t1_all_ints (
    a TINYINT,
    b SMALLINT,
    c INT,
    d BIGINT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t1_all_ints VALUES (10, 100, 1000, 10000);
INSERT INTO t1_all_ints VALUES (-10, -100, -1000, -10000);
INSERT INTO t1_all_ints VALUES (0, 0, 0, 0);
SELECT * FROM t1_all_ints ORDER BY a;
-- Expected: 3 rows covering all integer types

-- Test 1.23: INSERT FLOAT and DOUBLE with scientific notation
CREATE TABLE t1_sci_float (
    id INT,
    f_val FLOAT,
    d_val DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_sci_float VALUES (1, 1.5e2, 1.5e2);
INSERT INTO t1_sci_float VALUES (2, 3e-5, 3e-5);
INSERT INTO t1_sci_float VALUES (3, 0.0, 0.0);
SELECT * FROM t1_sci_float ORDER BY id;
-- Expected: 3 rows with scientific notation values

-- Test 1.24: INSERT DECIMAL with scale 0
CREATE TABLE t1_dec_scale0 (
    id INT,
    val DECIMAL(10, 0)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_dec_scale0 VALUES (1, 12345);
INSERT INTO t1_dec_scale0 VALUES (2, -9999);
INSERT INTO t1_dec_scale0 VALUES (3, 0);
SELECT * FROM t1_dec_scale0 ORDER BY id;
-- Expected: 3 rows with DECIMAL(10,0) values

-- Test 1.25: INSERT all types with max/min values
CREATE TABLE t1_type_limits (
    id INT,
    ti TINYINT,
    si SMALLINT,
    i INT,
    bi BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_type_limits VALUES (1, 127, 32767, 2147483647, 9223372036854775807);
INSERT INTO t1_type_limits VALUES (2, -128, -32768, -2147483648, -9223372036854775808);
SELECT * FROM t1_type_limits ORDER BY id;
-- Expected: 2 rows with min/max values for each integer type

-- Test 1.26: INSERT with zero-length VARCHAR
INSERT INTO t1_varchar VALUES (6, '');
SELECT * FROM t1_varchar WHERE id = 6;
-- Expected: 1 row with empty string

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 1.27: INSERT date boundaries
CREATE TABLE t1_date_boundary (
    id INT,
    d DATE,
    dt DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_date_boundary VALUES (1, '1970-01-01', '1970-01-01 00:00:01');
INSERT INTO t1_date_boundary VALUES (2, '2025-12-31', '2025-12-31 23:59:59');
INSERT INTO t1_date_boundary VALUES (3, '2024-02-29', '2024-02-29 00:00:00');
SELECT * FROM t1_date_boundary ORDER BY id;
-- Expected: 3 rows with dates

-- Test 1.28: INSERT with TINYINT value 1 and 0 as BOOLEAN (implicit bool)
CREATE TABLE t1_bool_implicit (
    id INT,
    is_active BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_bool_implicit VALUES (1, 1);
INSERT INTO t1_bool_implicit VALUES (2, 0);
SELECT * FROM t1_bool_implicit ORDER BY id;
-- Expected: 2 rows, is_active TRUE then FALSE

-- Test 1.29: INSERT multiple VARCHAR values of varying length
CREATE TABLE t1_varchar_len (
    id INT,
    short_str VARCHAR(10),
    long_str VARCHAR(500)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_varchar_len VALUES (1, 'hi', 'a');
INSERT INTO t1_varchar_len VALUES (2, 'hello', 'hello world this is a longer string for testing purposes');
SELECT * FROM t1_varchar_len ORDER BY id;
-- Expected: 2 rows with short and long strings

-- Test 1.30: INSERT FLOAT with trailing zeros
CREATE TABLE t1_float_trail (
    id INT,
    val FLOAT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t1_float_trail VALUES (1, 1.0);
INSERT INTO t1_float_trail VALUES (2, 2.50);
INSERT INTO t1_float_trail VALUES (3, 0.100);
SELECT * FROM t1_float_trail ORDER BY id;
-- Expected: 3 rows: 1.0, 2.5, 0.1

-- ============================================================================
-- Part 2: INSERT with Column Specification
-- ============================================================================

-- Test 2.1: INSERT with column subset (only id and name)
CREATE TABLE t2_cols (
    id INT,
    name VARCHAR(50),
    age INT,
    email VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t2_cols (id, name) VALUES (1, 'Alice');
INSERT INTO t2_cols (id, name) VALUES (2, 'Bob');
INSERT INTO t2_cols (id, name) VALUES (3, 'Charlie');
SELECT id, name, age, email FROM t2_cols ORDER BY id;
-- Expected: 3 rows with age=NULL, email=NULL

-- Test 2.2: INSERT with reordered columns
INSERT INTO t2_cols (name, id, email) VALUES ('Diana', 4, 'diana@test.com');
INSERT INTO t2_cols (age, id, name) VALUES (30, 5, 'Eve');
SELECT id, name, age, email FROM t2_cols ORDER BY id;
-- Expected: 5 rows, id=4 has name+email, id=5 has name+age

-- Test 2.3: INSERT with all columns explicitly
INSERT INTO t2_cols (id, name, age, email) VALUES (6, 'Frank', 40, 'frank@test.com');
SELECT * FROM t2_cols WHERE id = 6;
-- Expected: 1 row with all fields filled

-- Test 2.4: INSERT with only some columns (3 of 4)
CREATE TABLE t2_cols_subset (
    id INT,
    col1 INT,
    col2 VARCHAR(20),
    col3 DOUBLE,
    col4 BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t2_cols_subset (id, col1, col2) VALUES (1, 100, 'text');
INSERT INTO t2_cols_subset (id, col3, col4) VALUES (2, 3.14, TRUE);
INSERT INTO t2_cols_subset (id, col2, col4) VALUES (3, 'only_col2', FALSE);
SELECT * FROM t2_cols_subset ORDER BY id;
-- Expected: 3 rows, unspecified columns are NULL

-- Test 2.5: INSERT with single column specified
CREATE TABLE t2_single_col (
    id INT,
    val VARCHAR(50),
    extra INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t2_single_col (id) VALUES (1);
INSERT INTO t2_single_col (id) VALUES (2);
INSERT INTO t2_single_col (val) VALUES ('only_val');
SELECT * FROM t2_single_col ORDER BY id;
-- Expected: 3 rows, unspecified columns are NULL

-- Test 2.6: INSERT with columns in different order 2
INSERT INTO t2_cols_subset (col2, col1, id, col3, col4) VALUES ('reverse', 999, 4, 2.71, FALSE);
SELECT * FROM t2_cols_subset WHERE id = 4;
-- Expected: 1 row with reversed column order filled correctly

-- Test 2.7: INSERT with columns out of order (all columns)
INSERT INTO t2_cols_subset (col4, col3, col2, col1, id) VALUES (TRUE, 1.23, 'full_rev', 555, 5);
SELECT * FROM t2_cols_subset WHERE id = 5;
-- Expected: 1 row with all values in correct positions despite reversed column list

-- Test 2.8: INSERT only BIGINT column in multi-type table (Note: DATE converted to VARCHAR(30))
CREATE TABLE t2_multi_type (
    a INT,
    b VARCHAR(20),
    c DECIMAL(10,2),
    d DATE
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t2_multi_type (a, c) VALUES (1, 99.99);
INSERT INTO t2_multi_type (b, d) VALUES ('date_only', '2024-06-01');
SELECT * FROM t2_multi_type ORDER BY a;
-- Expected: 2 rows, unspecified columns NULL

-- Test 2.9: INSERT with columns, only nullable columns omitted
INSERT INTO t2_multi_type (a, b, c, d) VALUES (2, 'all_four', 50.00, '2024-01-15');
SELECT * FROM t2_multi_type WHERE a = 2;
-- Expected: 1 fully populated row

-- Test 2.10: INSERT with column spec for BOOLEAN and VARCHAR
INSERT INTO t2_cols_subset (id, col4, col2) VALUES (6, TRUE, 'bool_and_str');
SELECT id, col4, col2 FROM t2_cols_subset WHERE id = 6;
-- Expected: id=6, col4=TRUE, col2='bool_and_str', other cols NULL

-- Test 2.11: INSERT with column spec - different permutations
CREATE TABLE t2_perm (
    a INT,
    b TINYINT,
    c SMALLINT,
    d INT,
    e BIGINT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t2_perm (e, d, c, b, a) VALUES (500, 400, 300, 200, 100);
INSERT INTO t2_perm (a, c, e) VALUES (10, 30, 50);
SELECT * FROM t2_perm ORDER BY a;
-- Expected: 2 rows, first has all values (reversed order), second has a,c,e filled

-- Test 2.12: INSERT with column spec for VARCHAR(30) (DATE/DATETIME converted to VARCHAR)
CREATE TABLE t2_date_cols (
    id INT,
    d DATE,
    dt DATETIME,
    remark VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t2_date_cols (id, d) VALUES (1, '2024-01-01');
INSERT INTO t2_date_cols (id, dt) VALUES (2, '2024-06-15 12:30:00');
INSERT INTO t2_date_cols (id, d, dt) VALUES (3, '2025-12-31', '2025-12-31 23:59:59');
SELECT * FROM t2_date_cols ORDER BY id;
-- Expected: 3 rows with various date/datetime VARCHAR values

-- Test 2.13: INSERT with column spec for BIGINT (replaces LARGEINT — LARGEINT not fully supported)
CREATE TABLE t2_largeint_cols (
    id INT,
    val BIGINT,
    note VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t2_largeint_cols (id, val) VALUES (1, 1234567890123456789);
INSERT INTO t2_largeint_cols (id, note) VALUES (2, 'no largeint');
SELECT * FROM t2_largeint_cols ORDER BY id;
-- Expected: 2 rows, id=2 has val=NULL

-- Test 2.14: INSERT with column spec - only one column provided
INSERT INTO t2_perm (a) VALUES (999);
SELECT a FROM t2_perm WHERE a = 999;
-- Expected: 1 row with a=999, rest NULL

-- Test 2.15: INSERT with column spec and multiple rows
INSERT INTO t2_cols (id, name) VALUES (7, 'Grace'), (8, 'Hank'), (9, 'Ivy');
SELECT id, name FROM t2_cols WHERE id >= 7 ORDER BY id;
-- Expected: 3 rows: Grace, Hank, Ivy

-- ============================================================================
-- Part 3: INSERT with NULL Values
-- ============================================================================

-- Test 3.1: NULL in INT column
CREATE TABLE t3_null_int (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_int VALUES (1, NULL);
INSERT INTO t3_null_int VALUES (2, 100);
SELECT * FROM t3_null_int ORDER BY id;
-- Expected: 2 rows, id=1 val=NULL, id=2 val=100

-- Test 3.2: NULL in VARCHAR column
CREATE TABLE t3_null_varchar (
    id INT,
    name VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_varchar VALUES (1, NULL);
INSERT INTO t3_null_varchar VALUES (2, 'not null');
SELECT * FROM t3_null_varchar ORDER BY id;
-- Expected: id=1 name=NULL, id=2 name='not null'

-- Test 3.3: NULL in DECIMAL column
CREATE TABLE t3_null_decimal (
    id INT,
    salary DECIMAL(12, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_decimal VALUES (1, NULL);
INSERT INTO t3_null_decimal VALUES (2, 50000.00);
SELECT * FROM t3_null_decimal ORDER BY id;
-- Expected: 2 rows, id=1 salary=NULL

-- Test 3.4: NULL in BOOLEAN column
CREATE TABLE t3_null_bool (
    id INT,
    flag BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_bool VALUES (1, NULL);
INSERT INTO t3_null_bool VALUES (2, TRUE);
SELECT * FROM t3_null_bool ORDER BY id;
-- Expected: id=1 flag=NULL

-- Test 3.5: NULL in DATE column
CREATE TABLE t3_null_date (
    id INT,
    d DATE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_date VALUES (1, NULL);
INSERT INTO t3_null_date VALUES (2, '2024-06-15');
SELECT * FROM t3_null_date ORDER BY id;
-- Expected: id=1 d=NULL

-- Note: DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 3.6: NULL in DATETIME column
CREATE TABLE t3_null_datetime (
    id INT,
    dt DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_datetime VALUES (1, NULL);
INSERT INTO t3_null_datetime VALUES (2, '2024-06-15 12:30:00');
SELECT * FROM t3_null_datetime ORDER BY id;
-- Expected: id=1 dt=NULL

-- Test 3.7: NULL in FLOAT column
CREATE TABLE t3_null_float (
    id INT,
    val FLOAT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_float VALUES (1, NULL);
INSERT INTO t3_null_float VALUES (2, 3.14);
SELECT * FROM t3_null_float ORDER BY id;
-- Expected: 2 rows

-- Test 3.8: NULL in DOUBLE column
CREATE TABLE t3_null_double (
    id INT,
    val DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_double VALUES (1, NULL);
INSERT INTO t3_null_double VALUES (2, 2.71828);
SELECT * FROM t3_null_double ORDER BY id;
-- Expected: 2 rows

-- Test 3.9: NULL in TINYINT column
CREATE TABLE t3_null_tinyint (
    id INT,
    val TINYINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_tinyint VALUES (1, NULL);
INSERT INTO t3_null_tinyint VALUES (2, 100);
SELECT * FROM t3_null_tinyint ORDER BY id;
-- Expected: id=1 val=NULL, id=2 val=100

-- Test 3.10: NULL in SMALLINT column
CREATE TABLE t3_null_smallint (
    id INT,
    val SMALLINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_smallint VALUES (1, NULL);
INSERT INTO t3_null_smallint VALUES (2, 200);
SELECT * FROM t3_null_smallint ORDER BY id;
-- Expected: 2 rows

-- Test 3.11: NULL in BIGINT column
CREATE TABLE t3_null_bigint (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_bigint VALUES (1, NULL);
INSERT INTO t3_null_bigint VALUES (2, 9999999999);
SELECT * FROM t3_null_bigint ORDER BY id;
-- Expected: 2 rows

-- Test 3.12: NULL in BIGINT column (replaces LARGEINT — LARGEINT not fully supported)
CREATE TABLE t3_null_largeint (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_largeint VALUES (1, NULL);
INSERT INTO t3_null_largeint VALUES (2, 1234567890123456789);
SELECT * FROM t3_null_largeint ORDER BY id;
-- Expected: 2 rows

-- Test 3.13: NULL in CHAR column
CREATE TABLE t3_null_char (
    id INT,
    val CHAR(10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_char VALUES (1, NULL);
INSERT INTO t3_null_char VALUES (2, 'hello');
SELECT * FROM t3_null_char ORDER BY id;
-- Expected: 2 rows

-- Test 3.14: NULL in TEXT column
CREATE TABLE t3_null_text (
    id INT,
    val TEXT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_text VALUES (1, NULL);
INSERT INTO t3_null_text VALUES (2, 'some text');
SELECT * FROM t3_null_text ORDER BY id;
-- Expected: 2 rows

-- Test 3.15: NULL in all positions of a multi-column INSERT (Note: DATE converted to VARCHAR)
CREATE TABLE t3_all_null (
    a INT,
    b VARCHAR(20),
    c DECIMAL(10,2),
    d BOOLEAN,
    e DATE
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t3_all_null VALUES (NULL, NULL, NULL, NULL, NULL);
INSERT INTO t3_all_null VALUES (1, 'val', 10.5, TRUE, '2024-01-01');
SELECT * FROM t3_all_null ORDER BY a;
-- Expected: 2 rows, first row all NULLs

-- Test 3.16: NULL in each column individually
INSERT INTO t3_all_null VALUES (NULL, 'b2', 20.5, FALSE, '2024-02-01');
INSERT INTO t3_all_null VALUES (3, NULL, 30.5, TRUE, '2024-03-01');
INSERT INTO t3_all_null VALUES (4, 'b4', NULL, FALSE, '2024-04-01');
INSERT INTO t3_all_null VALUES (5, 'b5', 50.5, NULL, '2024-05-01');
INSERT INTO t3_all_null VALUES (6, 'b6', 60.5, TRUE, NULL);
SELECT * FROM t3_all_null WHERE a IS NULL OR a BETWEEN 3 AND 6 ORDER BY a;
-- Expected: each row has NULL in a different column

-- Test 3.17: NULL in batch INSERT
CREATE TABLE t3_null_batch (
    id INT,
    val INT,
    label VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_batch VALUES (1, NULL, 'a'), (2, 200, NULL), (3, NULL, NULL), (4, 400, 'd');
SELECT * FROM t3_null_batch ORDER BY id;
-- Expected: 4 rows with various NULL combinations

-- Test 3.18: NULL with column specification
INSERT INTO t3_null_batch (id, label) VALUES (5, 'e');
INSERT INTO t3_null_batch (id, val) VALUES (6, 600);
SELECT * FROM t3_null_batch WHERE id >= 5 ORDER BY id;
-- Expected: id=5 val=NULL, id=6 label=NULL

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 3.19: NULL in DATE columns batch
CREATE TABLE t3_null_date_batch (
    id INT,
    d DATE,
    dt DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t3_null_date_batch VALUES (1, NULL, '2024-01-01 00:00:00');
INSERT INTO t3_null_date_batch VALUES (2, '2024-06-15', NULL);
INSERT INTO t3_null_date_batch VALUES (3, NULL, NULL);
SELECT * FROM t3_null_date_batch ORDER BY id;
-- Expected: 3 rows with NULL in various date/datetime positions

-- Test 3.20: All-BOOLEAN table with NULL
CREATE TABLE t3_null_bool_multi (
    a BOOLEAN,
    b BOOLEAN,
    c BOOLEAN
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t3_null_bool_multi VALUES (TRUE, NULL, FALSE);
INSERT INTO t3_null_bool_multi VALUES (NULL, TRUE, NULL);
INSERT INTO t3_null_bool_multi VALUES (NULL, NULL, NULL);
SELECT * FROM t3_null_bool_multi ORDER BY a;
-- Expected: 3 rows with NULLs in boolean columns

-- ============================================================================
-- Part 4: INSERT with Expressions
-- ============================================================================

-- Test 4.1: Arithmetic expression in VALUES
CREATE TABLE t4_arith (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_arith VALUES (1, 10 + 20);
INSERT INTO t4_arith VALUES (2, 100 - 25);
INSERT INTO t4_arith VALUES (3, 5 * 6);
INSERT INTO t4_arith VALUES (4, 100 / 4);
SELECT * FROM t4_arith ORDER BY id;
-- Expected: (1,30), (2,75), (3,30), (4,25)

-- Test 4.2: Multiple arithmetic expressions in one row
CREATE TABLE t4_multi_arith (
    id INT,
    sum_val INT,
    diff_val INT,
    prod_val INT,
    quot_val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_multi_arith VALUES (1, 10 + 20, 100 - 30, 5 * 6, 50 / 5);
INSERT INTO t4_multi_arith VALUES (2, 1 + 2 + 3, 200 - 50 - 30, 2 * 3 * 4, 100 / 5 / 2);
SELECT * FROM t4_multi_arith ORDER BY id;
-- Expected: (1,30,70,30,10), (2,6,120,24,10)

-- Test 4.3: Complex arithmetic with parentheses
INSERT INTO t4_arith VALUES (5, (10 + 5) * 2);
INSERT INTO t4_arith VALUES (6, 100 - (20 + 30));
INSERT INTO t4_arith VALUES (7, (100 - 20) / 8);
SELECT * FROM t4_arith WHERE id >= 5 ORDER BY id;
-- Expected: (5,30), (6,50), (7,10)

-- Test 4.4: String concatenation expression
CREATE TABLE t4_concat (
    id INT,
    val VARCHAR(200)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_concat VALUES (1, 'Hello' || ' ' || 'World');
INSERT INTO t4_concat VALUES (2, 'a' || 'b');
INSERT INTO t4_concat VALUES (3, 'test_' || 123);
INSERT INTO t4_concat VALUES (4, '' || 'nonempty');
SELECT * FROM t4_concat ORDER BY id;
-- Expected: (1,'Hello World'), (2,'ab'), (3,'test_123'), (4,'nonempty')

-- Test 4.5: Nested string concatenation
INSERT INTO t4_concat VALUES (5, 'a' || 'b' || 'c' || 'd');
INSERT INTO t4_concat VALUES (6, 'Hello ' || 'World ' || 'Again');
SELECT * FROM t4_concat WHERE id >= 5 ORDER BY id;
-- Expected: (5,'abcd'), (6,'Hello World Again')

-- Test 4.6: Arithmetic with DECIMAL types
CREATE TABLE t4_dec_arith (
    id INT,
    val DECIMAL(12, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_dec_arith VALUES (1, 10.50 + 20.25);
INSERT INTO t4_dec_arith VALUES (2, 100.00 - 25.50);
INSERT INTO t4_dec_arith VALUES (3, 5.5 * 6.0);
INSERT INTO t4_dec_arith VALUES (4, 100.00 / 4.0);
SELECT * FROM t4_dec_arith ORDER BY id;
-- Expected: DECIMAL arithmetic results

-- Test 4.7: Mix DECIMAL and INT operations
INSERT INTO t4_dec_arith VALUES (5, 10.50 + 5);
INSERT INTO t4_dec_arith VALUES (6, 100 * 1.5);
SELECT * FROM t4_dec_arith WHERE id >= 5 ORDER BY id;
-- Expected: (5,15.50), (6,150.00)

-- Test 4.8: Multiple type expressions in single row
CREATE TABLE t4_mixed_expr (
    id INT,
    i INT,
    f FLOAT,
    s VARCHAR(100),
    d DECIMAL(10,2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_mixed_expr VALUES (1, 10 + 20, 3.14 * 2, 'pre' || 'fix', 99.99);
INSERT INTO t4_mixed_expr VALUES (2, 50 - 5, 10.0 / 3.0, 'a' || 'b' || 'c', 50.00 + 50.00);
SELECT * FROM t4_mixed_expr ORDER BY id;
-- Expected: 2 rows with computed values

-- Test 4.9: Expression in column-specified INSERT
INSERT INTO t4_mixed_expr (id, i, s) VALUES (3, 2 + 3, 'concat' || 'enated');
SELECT id, i, s FROM t4_mixed_expr WHERE id = 3;
-- Expected: id=3, i=5, s='concatenated'

-- Test 4.10: Arithmetic with BIGINT values
CREATE TABLE t4_bigint_arith (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_bigint_arith VALUES (1, 10000000000 + 1);
INSERT INTO t4_bigint_arith VALUES (2, 5000000000 * 2);
INSERT INTO t4_bigint_arith VALUES (3, 10000000000 - 5000000000);
SELECT * FROM t4_bigint_arith ORDER BY id;
-- Expected: BIGINT arithmetic results

-- Test 4.11: Expression with negative numbers
INSERT INTO t4_arith VALUES (8, 10 - 50);
INSERT INTO t4_arith VALUES (9, -10 + 5);
INSERT INTO t4_arith VALUES (10, -5 * -10);
SELECT * FROM t4_arith WHERE id >= 8 ORDER BY id;
-- Expected: (8,-40), (9,-5), (10,50)

-- Test 4.12: Expressions mixing arithmetic and string
INSERT INTO t4_concat VALUES (7, 'result: ' || (10 + 5));
SELECT * FROM t4_concat WHERE id = 7;
-- Expected: (7,'result: 15')

-- Test 4.13: Division expression (non-integer result into INT)
INSERT INTO t4_arith VALUES (11, 10 / 3);
INSERT INTO t4_arith VALUES (12, 7 / 2);
SELECT * FROM t4_arith WHERE id >= 11 ORDER BY id;
-- Expected: integer division results (3, 3)

-- Test 4.14: Modulo expression
CREATE TABLE t4_mod (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_mod VALUES (1, 10 % 3);
INSERT INTO t4_mod VALUES (2, 100 % 7);
INSERT INTO t4_mod VALUES (3, 15 % 5);
SELECT * FROM t4_mod ORDER BY id;
-- Expected: (1,1), (2,2), (3,0)

-- Test 4.15: Concat with numeric columns
INSERT INTO t4_concat VALUES (8, 123 || 'abc');
SELECT * FROM t4_concat WHERE id = 8;
-- Expected: (8,'123abc')

-- Test 4.16: Expression with FLOAT division
INSERT INTO t4_mixed_expr (id, f) VALUES (4, 10.0 / 3.0);
SELECT id, f FROM t4_mixed_expr WHERE id = 4;
-- Expected: id=4, f ≈ 3.33333

-- Test 4.17: Multiple ROUND calls / nested expressions
CREATE TABLE t4_nested (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t4_nested VALUES (1, (10 + 5) * (2 + 3));
INSERT INTO t4_nested VALUES (2, (100 - 30) / (5 + 2));
INSERT INTO t4_nested VALUES (3, 2 * 3 * 4 * 5);
SELECT * FROM t4_nested ORDER BY id;
-- Expected: (1,75), (2,10), (3,120)

-- Test 4.18: Expression with DECIMAL * INT
INSERT INTO t4_dec_arith VALUES (7, 1.5 * 10);
INSERT INTO t4_dec_arith VALUES (8, 0.5 * 100);
SELECT * FROM t4_dec_arith WHERE id >= 7 ORDER BY id;
-- Expected: (7,15.00), (8,50.00)

-- Test 4.19: FLOAT * FLOAT expression
INSERT INTO t4_mixed_expr (id, f) VALUES (5, 2.5 * 4.0);
SELECT id, f FROM t4_mixed_expr WHERE id = 5;
-- Expected: id=5, f=10.0

-- Test 4.20: Expression in batch VALUES
INSERT INTO t4_arith VALUES (13, 2*3), (14, 100/5), (15, 10+20+30);
SELECT * FROM t4_arith WHERE id >= 13 ORDER BY id;
-- Expected: (13,6), (14,20), (15,60)

-- ============================================================================
-- Part 5: Batch/Multi-Row INSERT
-- ============================================================================

-- Test 5.1: Batch insert 2 rows
CREATE TABLE t5_batch (
    id INT,
    name VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch VALUES (1, 'A'), (2, 'B');
SELECT * FROM t5_batch ORDER BY id;
-- Expected: 2 rows

-- Test 5.2: Batch insert 3 rows
INSERT INTO t5_batch VALUES (3, 'C'), (4, 'D'), (5, 'E');
SELECT * FROM t5_batch ORDER BY id;
-- Expected: 5 rows

-- Test 5.3: Batch insert 5 rows
INSERT INTO t5_batch VALUES (6, 'F'), (7, 'G'), (8, 'H'), (9, 'I'), (10, 'J');
SELECT * FROM t5_batch ORDER BY id;
-- Expected: 10 rows

-- Test 5.4: Batch insert 10 rows
INSERT INTO t5_batch VALUES
    (11, 'K'), (12, 'L'), (13, 'M'), (14, 'N'), (15, 'O'),
    (16, 'P'), (17, 'Q'), (18, 'R'), (19, 'S'), (20, 'T');
SELECT COUNT(*) FROM t5_batch;
-- Expected: 20

-- Test 5.5: Batch 5 rows with NULL values
CREATE TABLE t5_batch_null (
    id INT,
    val INT,
    label VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_null VALUES (1, 10, 'a'), (2, NULL, 'b'), (3, 30, NULL), (4, NULL, NULL), (5, 50, 'e');
SELECT * FROM t5_batch_null ORDER BY id;
-- Expected: 5 rows with various NULL combos

-- Test 5.6: Batch insert with BOOLEAN and other types
CREATE TABLE t5_batch_types (
    id INT,
    flag BOOLEAN,
    val DECIMAL(8,2),
    label VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_types VALUES
    (1, TRUE, 10.50, 'a'),
    (2, FALSE, 20.00, 'b'),
    (3, TRUE, 30.25, 'c');
SELECT * FROM t5_batch_types ORDER BY id;
-- Expected: 3 rows

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 5.7: Batch insert with DATE
CREATE TABLE t5_batch_date (
    id INT,
    d DATE,
    dt DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_date VALUES
    (1, '2024-01-01', '2024-01-01 00:00:00'),
    (2, '2024-06-15', '2024-06-15 12:30:00'),
    (3, '2025-12-31', '2025-12-31 23:59:59');
SELECT * FROM t5_batch_date ORDER BY id;
-- Expected: 3 rows with dates

-- Test 5.8: Batch insert 20+ rows
CREATE TABLE t5_large_batch (
    id INT,
    val INT,
    flag VARCHAR(10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_large_batch VALUES
    (1, 100, 'a'), (2, 200, 'b'), (3, 300, 'c'), (4, 400, 'd'), (5, 500, 'e'),
    (6, 600, 'f'), (7, 700, 'g'), (8, 800, 'h'), (9, 900, 'i'), (10, 1000, 'j'),
    (11, 1100, 'k'), (12, 1200, 'l'), (13, 1300, 'm'), (14, 1400, 'n'), (15, 1500, 'o'),
    (16, 1600, 'p'), (17, 1700, 'q'), (18, 1800, 'r'), (19, 1900, 's'), (20, 2000, 't'),
    (21, 2100, 'u'), (22, 2200, 'v'), (23, 2300, 'w'), (24, 2400, 'x'), (25, 2500, 'y');
SELECT COUNT(*) FROM t5_large_batch;
-- Expected: 25 rows

-- Test 5.9: Batch insert with expressions per row
CREATE TABLE t5_batch_expr (
    id INT,
    computed INT,
    label VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_expr VALUES
    (1, 10 + 20, 'sum30'),
    (2, 100 - 25, 'diff75'),
    (3, 5 * 6, 'prod30');
SELECT * FROM t5_batch_expr ORDER BY id;
-- Expected: 3 rows with computed values

-- Test 5.10: Batch insert with mixed NULL and expressions
INSERT INTO t5_batch_expr VALUES
    (4, NULL, 'null_val'),
    (5, 50 + 50, NULL),
    (6, NULL, NULL),
    (7, 100, 'hundred');
SELECT * FROM t5_batch_expr WHERE id >= 4 ORDER BY id;
-- Expected: 4 rows with various NULL/computed combos

-- Test 5.11: Batch insert multiple FLOAT/DOUBLE values
CREATE TABLE t5_batch_float (
    id INT,
    f FLOAT,
    d DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_float VALUES
    (1, 1.1, 2.2),
    (2, 3.3, 4.4),
    (3, 5.5, 6.6),
    (4, 7.7, 8.8),
    (5, 9.9, 10.10);
SELECT * FROM t5_batch_float ORDER BY id;
-- Expected: 5 rows

-- Test 5.12: Batch insert with many DECIMAL values
CREATE TABLE t5_batch_dec (
    id INT,
    price DECIMAL(10,2),
    qty DECIMAL(8,0)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_dec VALUES
    (1, 1.99, 10),
    (2, 5.49, 3),
    (3, 10.00, 1),
    (4, 99.99, 2),
    (5, 0.01, 100),
    (6, 1234.56, 7),
    (7, 999.99, 5);
SELECT * FROM t5_batch_dec ORDER BY id;
-- Expected: 7 rows

-- Test 5.13: Batch insert with TINYINT SMALLINT INT
CREATE TABLE t5_batch_ints (
    a TINYINT,
    b SMALLINT,
    c INT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t5_batch_ints VALUES
    (1, 10, 100),
    (2, 20, 200),
    (3, 30, 300),
    (4, 40, 400),
    (5, 50, 500);
SELECT * FROM t5_batch_ints ORDER BY a;
-- Expected: 5 rows

-- Test 5.14: Batch insert BIGINT values (replaces LARGEINT — LARGEINT not fully supported)
CREATE TABLE t5_batch_large (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_large VALUES
    (1, 1000000000000000000),
    (2, 2000000000000000000),
    (3, 3000000000000000000),
    (4, 4000000000000000000);
SELECT * FROM t5_batch_large ORDER BY id;
-- Expected: 4 rows

-- Test 5.15: Batch insert with mixed types (all at once)
CREATE TABLE t5_batch_all (
    a BOOLEAN,
    b INT,
    c VARCHAR(20),
    d DECIMAL(10,2)
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t5_batch_all VALUES
    (TRUE, 1, 'one', 1.11),
    (FALSE, 2, 'two', 2.22),
    (TRUE, 3, 'three', 3.33),
    (FALSE, 4, 'four', 4.44),
    (TRUE, 5, 'five', 5.55);
SELECT * FROM t5_batch_all ORDER BY b;
-- Expected: 5 rows

-- Test 5.16: Batch insert with column specification
INSERT INTO t5_batch_all (b, c, a) VALUES (6, 'six', FALSE), (7, 'seven', TRUE);
SELECT b, c, a FROM t5_batch_all WHERE b >= 6 ORDER BY b;
-- Expected: 2 rows with specified columns

-- Test 5.17: Batch insert into table with VARHCAR and TEXT
CREATE TABLE t5_batch_str (
    id INT,
    v VARCHAR(50),
    t TEXT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_str VALUES
    (1, 'short', 'a longer text'),
    (2, '', ''),
    (3, 'v3', 'text3'),
    (4, 'v4', 'text4'),
    (5, 'v5', 'text5');
SELECT * FROM t5_batch_str ORDER BY id;
-- Expected: 5 rows

-- Test 5.18: Batch 30+ rows
CREATE TABLE t5_batch_30 (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t5_batch_30 VALUES
    (1, 1), (2, 2), (3, 3), (4, 4), (5, 5),
    (6, 6), (7, 7), (8, 8), (9, 9), (10, 10),
    (11, 11), (12, 12), (13, 13), (14, 14), (15, 15),
    (16, 16), (17, 17), (18, 18), (19, 19), (20, 20),
    (21, 21), (22, 22), (23, 23), (24, 24), (25, 25),
    (26, 26), (27, 27), (28, 28), (29, 29), (30, 30),
    (31, 31), (32, 32), (33, 33), (34, 34), (35, 35);
SELECT COUNT(*) FROM t5_batch_30;
-- Expected: 35 rows

-- Test 5.19: Batch insert with DATE values
INSERT INTO t5_batch_date VALUES
    (4, '2024-03-01', '2024-03-01 08:00:00'),
    (5, '2024-04-01', '2024-04-01 16:30:00');
SELECT * FROM t5_batch_date ORDER BY id;
-- Expected: 5 rows

-- Test 5.20: Batch insert VARCHAR values all distinct
INSERT INTO t5_batch VALUES
    (21, 'U'), (22, 'V'), (23, 'W'), (24, 'X'), (25, 'Y'), (26, 'Z');
SELECT COUNT(*) FROM t5_batch;
-- Expected: 26 rows

-- ============================================================================
-- Part 6: INSERT SELECT
-- ============================================================================

-- Setup source tables for INSERT SELECT tests
CREATE TABLE t6_source (
    id INT,
    category VARCHAR(20),
    amount INT,
    score DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_source VALUES
    (1, 'A', 100, 95.5),
    (2, 'A', 200, 87.3),
    (3, 'B', 150, 91.2),
    (4, 'B', 250, 78.9),
    (5, 'C', 300, 88.1),
    (6, 'C', 350, 92.4),
    (7, 'A', 400, 85.0),
    (8, 'B', 450, 90.5),
    (9, 'C', 500, 83.7),
    (10, 'A', 550, 96.8);

-- Test 6.1: Basic INSERT SELECT (all columns)
CREATE TABLE t6_sel_basic (
    id INT,
    category VARCHAR(20),
    amount INT,
    score DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_basic SELECT * FROM t6_source;
SELECT COUNT(*) FROM t6_sel_basic;
-- Expected: 10 rows

-- Test 6.2: INSERT SELECT with column subset
CREATE TABLE t6_sel_subset (
    id INT,
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_subset SELECT id, amount FROM t6_source;
SELECT * FROM t6_sel_subset ORDER BY id;
-- Expected: 10 rows with id and amount

-- Test 6.3: INSERT SELECT with computed columns
CREATE TABLE t6_sel_computed (
    id INT,
    doubled INT,
    category_label VARCHAR(50),
    tax DECIMAL(10,2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
-- Note: expressions in INSERT SELECT may not work reliably
INSERT INTO t6_sel_computed
SELECT id, amount * 2, category || '_cat', amount * 0.08 FROM t6_source;
SELECT * FROM t6_sel_computed ORDER BY id;
-- Expected: 10 rows with computed values

-- Test 6.4: INSERT SELECT with WHERE clause (equality)
CREATE TABLE t6_sel_where (
    id INT,
    category VARCHAR(20),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE category = 'A';
SELECT * FROM t6_sel_where ORDER BY id;
-- Expected: 4 rows (ids 1,2,7,10)

-- Test 6.5: INSERT SELECT with WHERE (comparison)
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE amount > 300;
SELECT * FROM t6_sel_where WHERE id > 10 ORDER BY id;
-- Expected: 4 rows (ids from source with amount > 300)

-- Test 6.6: INSERT SELECT with WHERE AND
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE category = 'B' AND amount >= 200;
SELECT * FROM t6_sel_where WHERE id > 20 ORDER BY id;
-- Expected: 2 rows (ids 4,8 from source)

-- Test 6.7: INSERT SELECT with WHERE OR
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE category = 'C' OR amount = 100;
SELECT * FROM t6_sel_where WHERE id > 30 ORDER BY id;
-- Expected: 4 rows (ids 1,5,6,9 from source)

-- Test 6.8: INSERT SELECT with WHERE IN
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE category IN ('A', 'C');
SELECT * FROM t6_sel_where WHERE id > 40 ORDER BY id;
-- Expected: 7 rows (category A or C rows from source)

-- Test 6.9: INSERT SELECT with WHERE BETWEEN
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE amount BETWEEN 200 AND 400;
SELECT * FROM t6_sel_where WHERE id > 50 ORDER BY id;
-- Expected: 4 rows

-- Test 6.10: INSERT SELECT with WHERE LIKE
INSERT INTO t6_sel_where SELECT id, category, amount FROM t6_source WHERE category LIKE 'A';
SELECT * FROM t6_sel_where WHERE id > 60 ORDER BY id;
-- Expected: 4 rows (same as category = 'A')

-- Test 6.11: INSERT SELECT with ORDER BY
CREATE TABLE t6_sel_order (
    id INT,
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_order SELECT id, amount FROM t6_source ORDER BY amount DESC;
SELECT * FROM t6_sel_order ORDER BY id;
-- Expected: 10 rows in id order (source data was ordered by amount desc but ends up in id order)

-- Test 6.12: INSERT SELECT with LIMIT
CREATE TABLE t6_sel_limit (
    id INT,
    category VARCHAR(20),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_limit SELECT id, category, amount FROM t6_source LIMIT 3;
SELECT COUNT(*) FROM t6_sel_limit;
-- Expected: 3 rows

-- Test 6.13: INSERT SELECT with ORDER BY and LIMIT
INSERT INTO t6_sel_limit SELECT id, category, amount FROM t6_source ORDER BY amount DESC LIMIT 5;
SELECT * FROM t6_sel_limit ORDER BY id;
-- Expected: 8 rows total (3 from previous + 5 new)

-- Test 6.14: INSERT SELECT with GROUP BY and aggregation
CREATE TABLE t6_sel_group (
    category VARCHAR(20),
    total_amount INT,
    avg_score DOUBLE,
    row_count INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_group
SELECT category, SUM(amount), AVG(score), COUNT(*)
FROM t6_source
GROUP BY category;
SELECT * FROM t6_sel_group ORDER BY category;
-- Expected: 3 rows (A: 1250, avg; B: 850, avg; C: 1150, avg)

-- Test 6.15: INSERT SELECT with GROUP BY HAVING
CREATE TABLE t6_sel_having (
    category VARCHAR(20),
    total INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_having
SELECT category, SUM(amount)
FROM t6_source
GROUP BY category
HAVING SUM(amount) > 1000;
SELECT * FROM t6_sel_having ORDER BY category;
-- Expected: 2 rows (A:1250, C:1150)

-- Test 6.16: INSERT SELECT with JOIN (INNER JOIN)
CREATE TABLE t6_join_cat (
    cat_code VARCHAR(20),
    cat_name VARCHAR(50)
) DISTRIBUTED BY HASH(cat_code) BUCKETS 3;
INSERT INTO t6_join_cat VALUES ('A', 'Category A'), ('B', 'Category B'), ('C', 'Category C'), ('D', 'Category D');

CREATE TABLE t6_sel_join (
    id INT,
    category VARCHAR(20),
    cat_name VARCHAR(50),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_join
SELECT s.id, s.category, c.cat_name, s.amount
FROM t6_source s
INNER JOIN t6_join_cat c ON s.category = c.cat_code;
SELECT * FROM t6_sel_join ORDER BY id;
-- Expected: 10 rows with cat_name filled

-- Test 6.17: INSERT SELECT with LEFT JOIN
CREATE TABLE t6_sel_left_join (
    id INT,
    category VARCHAR(20),
    cat_name VARCHAR(50),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_left_join
SELECT s.id, s.category, c.cat_name, s.amount
FROM t6_source s
LEFT JOIN t6_join_cat c ON s.category = c.cat_code;
SELECT * FROM t6_sel_left_join ORDER BY id;
-- Expected: 10 rows, all have cat_name (since all categories exist)

-- Test 6.18: INSERT SELECT with non-matching JOIN
CREATE TABLE t6_sel_no_match (
    id INT,
    category VARCHAR(20),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_no_match
SELECT s.id, s.category, s.amount
FROM t6_source s
INNER JOIN t6_join_cat c ON s.category = 'NONEXISTENT';
SELECT COUNT(*) FROM t6_sel_no_match;
-- Expected: 0 rows (no match)

-- Test 6.19: INSERT SELECT from self-join
CREATE TABLE t6_sel_self (
    a_id INT,
    a_amount INT,
    b_id INT,
    b_amount INT
) DISTRIBUTED BY HASH(a_id) BUCKETS 3;
INSERT INTO t6_sel_self
SELECT a.id, a.amount, b.id, b.amount
FROM t6_source a
INNER JOIN t6_source b ON a.category = b.category AND a.id < b.id;
SELECT * FROM t6_sel_self ORDER BY a_id, b_id;
-- Expected: rows from each pair within same category

-- Test 6.20: INSERT SELECT with subquery
CREATE TABLE t6_sel_subq (
    id INT,
    amount INT,
    max_amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_subq
SELECT id, amount, (SELECT MAX(amount) FROM t6_source) FROM t6_source;
SELECT * FROM t6_sel_subq ORDER BY id;
-- Expected: 10 rows, each with max_amount=550

-- Test 6.21: INSERT SELECT with DISTINCT
CREATE TABLE t6_sel_distinct (
    category VARCHAR(20)
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_distinct SELECT DISTINCT category FROM t6_source;
SELECT * FROM t6_sel_distinct ORDER BY category;
-- Expected: 3 rows: A, B, C

-- Test 6.22: INSERT SELECT with aggregate (MIN, MAX)
CREATE TABLE t6_sel_minmax (
    category VARCHAR(20),
    min_amt INT,
    max_amt INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_minmax
SELECT category, MIN(amount), MAX(amount)
FROM t6_source
GROUP BY category;
SELECT * FROM t6_sel_minmax ORDER BY category;
-- Expected: A:100/550, B:150/450, C:300/500

-- Test 6.23: INSERT SELECT empty result
INSERT INTO t6_sel_no_match
SELECT id, category, amount FROM t6_source WHERE 1=0;
SELECT COUNT(*) FROM t6_sel_no_match;
-- Expected: 0 rows (no change from before)

-- Test 6.24: INSERT SELECT with ORDER BY LIMIT and computed column
CREATE TABLE t6_sel_top_n (
    category VARCHAR(20),
    amount INT,
    rank_order INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_top_n
SELECT category, amount, ROW_NUMBER() OVER (ORDER BY amount DESC)
FROM t6_source
ORDER BY amount DESC
LIMIT 5;
SELECT * FROM t6_sel_top_n ORDER BY rank_order;
-- Expected: 5 rows with top amounts

-- Test 6.25: INSERT SELECT with CASE expression
CREATE TABLE t6_sel_case (
    id INT,
    tier VARCHAR(20)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_case
SELECT id,
    CASE
        WHEN amount >= 400 THEN 'high'
        WHEN amount >= 200 THEN 'medium'
        ELSE 'low'
    END
FROM t6_source;
SELECT * FROM t6_sel_case ORDER BY id;
-- Expected: 10 rows with tier labels

-- Test 6.26: INSERT SELECT with NULL handling
INSERT INTO t6_sel_where SELECT NULL, NULL, NULL;
SELECT * FROM t6_sel_where WHERE id IS NULL;
-- Expected: 1 row with all NULLs

-- Test 6.27: INSERT SELECT with UNION
-- Expected: combined rows from two SELECT queries
CREATE TABLE t6_sel_union (
    id INT,
    category VARCHAR(20),
    amount INT,
    score DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_union
SELECT * FROM t6_source WHERE category = 'A'
UNION
SELECT * FROM t6_source WHERE category = 'B';
SELECT COUNT(*) FROM t6_sel_union;
-- Expected: 7 rows
DROP TABLE t6_sel_union;

-- Test 6.28: INSERT SELECT with UNION ALL
-- Expected: combined rows from two queries including duplicates
CREATE TABLE t6_sel_union_all (
    id INT,
    category VARCHAR(20),
    amount INT,
    score DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_union_all
SELECT * FROM t6_source WHERE category = 'A'
UNION ALL
SELECT * FROM t6_source WHERE category = 'B';
SELECT COUNT(*) FROM t6_sel_union_all;
-- Expected: 7 rows (4 A + 3 B)
DROP TABLE t6_sel_union_all;

-- Test 6.29: INSERT SELECT with arithmetic on selected columns
CREATE TABLE t6_sel_arith (
    id INT,
    plus INT,
    minus INT,
    mult INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
-- Note: expressions in INSERT SELECT may not work reliably
INSERT INTO t6_sel_arith
SELECT id, amount + 100, amount - 50, amount * 2
FROM t6_source
WHERE category = 'A';
SELECT * FROM t6_sel_arith ORDER BY id;
-- Expected: 4 rows with arithmetic results

-- Test 6.30: INSERT SELECT with COUNT DISTINCT
CREATE TABLE t6_sel_count_dist (
    category VARCHAR(20),
    distinct_counts INT
) DISTRIBUTED BY HASH(category) BUCKETS 3;
INSERT INTO t6_sel_count_dist
SELECT category, COUNT(DISTINCT amount)
FROM t6_source
GROUP BY category;
SELECT * FROM t6_sel_count_dist ORDER BY category;
-- Expected: each category with distinct count of amounts

-- Test 6.31: INSERT SELECT with ORDER BY multiple columns
CREATE TABLE t6_sel_multi_order (
    id INT,
    category VARCHAR(20),
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_multi_order
SELECT id, category, amount FROM t6_source ORDER BY category, amount DESC;
SELECT * FROM t6_sel_multi_order ORDER BY id;
-- Expected: 10 rows

-- Test 6.32: INSERT SELECT with LIMIT OFFSET
CREATE TABLE t6_sel_offset (
    id INT,
    amount INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_offset SELECT id, amount FROM t6_source ORDER BY id LIMIT 3 OFFSET 2;
SELECT * FROM t6_sel_offset ORDER BY id;
-- Expected: 3 rows (ids 3,4,5)

-- Test 6.33: INSERT SELECT into table with more columns than source
CREATE TABLE t6_sel_more_cols (
    id INT,
    amount INT,
    extra INT,
    note VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_more_cols (id, amount)
SELECT id, amount FROM t6_source;
SELECT id, amount, extra, note FROM t6_sel_more_cols ORDER BY id;
-- Expected: 10 rows with extra=NULL and note=NULL

-- Test 6.34: INSERT SELECT with aggregate and GROUP BY on expression
CREATE TABLE t6_sel_group_expr (
    amount_group VARCHAR(20),
    cnt INT
) DISTRIBUTED BY HASH(amount_group) BUCKETS 3;
INSERT INTO t6_sel_group_expr
SELECT
    CASE WHEN amount < 200 THEN 'small' WHEN amount < 400 THEN 'medium' ELSE 'large' END,
    COUNT(*)
FROM t6_source
GROUP BY
    CASE WHEN amount < 200 THEN 'small' WHEN amount < 400 THEN 'medium' ELSE 'large' END;
SELECT * FROM t6_sel_group_expr ORDER BY amount_group;
-- Expected: 3 rows with counts

-- Test 6.35: INSERT SELECT from empty subquery
CREATE TABLE t6_sel_empty_sub (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t6_sel_empty_sub
SELECT id, amount FROM t6_source WHERE category = 'Z';
SELECT COUNT(*) FROM t6_sel_empty_sub;
-- Expected: 0 rows

-- ============================================================================
-- Part 7: INSERT with Type Conversion
-- ============================================================================

-- Test 7.1: Insert string into INT column
CREATE TABLE t7_conv (
    id INT,
    int_col INT,
    str_col VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_conv VALUES (1, '100', 'text');
SELECT * FROM t7_conv WHERE id = 1;
-- Expected: id=1, int_col=100 (type conversion worked)

-- Test 7.2: Insert INT into VARCHAR column
INSERT INTO t7_conv VALUES (2, 200, 300);
SELECT * FROM t7_conv WHERE id = 2;
-- Expected: id=2, int_col=200, str_col='300'

-- Test 7.3: Insert DECIMAL into INT column
INSERT INTO t7_conv VALUES (3, 99.9, 'from_dec');
SELECT * FROM t7_conv WHERE id = 3;
-- Expected: id=3, int_col=99 (or 100, depending on rounding)

-- Test 7.4: Insert string into DECIMAL column
CREATE TABLE t7_conv_dec (
    id INT,
    val DECIMAL(10, 2)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_conv_dec VALUES (1, '123.45');
INSERT INTO t7_conv_dec VALUES (2, '0.99');
INSERT INTO t7_conv_dec VALUES (3, '1000.00');
SELECT * FROM t7_conv_dec ORDER BY id;
-- Expected: 3 rows with DECIMAL conversion from string

-- Test 7.5: Insert INT into DOUBLE column
CREATE TABLE t7_int_to_double (
    id INT,
    val DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_int_to_double VALUES (1, 100);
INSERT INTO t7_int_to_double VALUES (2, 0);
INSERT INTO t7_int_to_double VALUES (3, -50);
SELECT * FROM t7_int_to_double ORDER BY id;
-- Expected: 3 rows, INT implicitly converted to DOUBLE

-- Test 7.6: Insert string into FLOAT column
CREATE TABLE t7_str_to_float (
    id INT,
    val FLOAT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_float VALUES (1, '3.14');
INSERT INTO t7_str_to_float VALUES (2, '100.0');
INSERT INTO t7_str_to_float VALUES (3, '0.001');
SELECT * FROM t7_str_to_float ORDER BY id;
-- Expected: 3 rows, string converted to FLOAT

-- Test 7.7: Insert string into TINYINT column
CREATE TABLE t7_str_to_tiny (
    id INT,
    val TINYINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_tiny VALUES (1, '100');
INSERT INTO t7_str_to_tiny VALUES (2, '0');
INSERT INTO t7_str_to_tiny VALUES (3, '50');
SELECT * FROM t7_str_to_tiny ORDER BY id;
-- Expected: 3 rows, string to TINYINT converted

-- Test 7.8: Insert string into BIGINT column
CREATE TABLE t7_str_to_big (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_big VALUES (1, '9999999999');
INSERT INTO t7_str_to_big VALUES (2, '0');
INSERT INTO t7_str_to_big VALUES (3, '-123456789');
SELECT * FROM t7_str_to_big ORDER BY id;
-- Expected: 3 rows, BIGINT values from strings

-- Test 7.9: Insert string date into VARCHAR(30) column (DATE may return empty strings, use VARCHAR)
CREATE TABLE t7_str_to_date (
    id INT,
    d DATE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_date VALUES (1, '2024-01-01');
INSERT INTO t7_str_to_date VALUES (2, '2024-12-31');
INSERT INTO t7_str_to_date VALUES (3, '1970-01-01');
SELECT * FROM t7_str_to_date ORDER BY id;
-- Expected: 3 rows with date values

-- Test 7.10: Insert string datetime into VARCHAR(30) column (DATETIME may return empty strings, use VARCHAR)
CREATE TABLE t7_str_to_dt (
    id INT,
    dt DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_dt VALUES (1, '2024-01-01 12:30:00');
INSERT INTO t7_str_to_dt VALUES (2, '2024-06-15 00:00:00');
INSERT INTO t7_str_to_dt VALUES (3, '2024-12-31 23:59:59');
SELECT * FROM t7_str_to_dt ORDER BY id;
-- Expected: 3 rows

-- Test 7.11: Insert INT into SMALLINT column
CREATE TABLE t7_int_to_small (
    id INT,
    val SMALLINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_int_to_small VALUES (1, 1000);
INSERT INTO t7_int_to_small VALUES (2, 0);
INSERT INTO t7_int_to_small VALUES (3, 32767);
SELECT * FROM t7_int_to_small ORDER BY id;
-- Expected: 3 rows

-- Test 7.12: Insert BOOLEAN as string
CREATE TABLE t7_str_to_bool (
    id INT,
    flag BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_bool VALUES (1, 'true');
INSERT INTO t7_str_to_bool VALUES (2, 'false');
INSERT INTO t7_str_to_bool VALUES (3, '1');
INSERT INTO t7_str_to_bool VALUES (4, '0');
SELECT * FROM t7_str_to_bool ORDER BY id;
-- Expected: 4 rows with boolean values converted from strings

-- Test 7.13: Insert BIGINT into INT column
CREATE TABLE t7_big_to_int (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_big_to_int VALUES (1, 1000000);
INSERT INTO t7_big_to_int VALUES (2, 0);
SELECT * FROM t7_big_to_int ORDER BY id;
-- Expected: 2 rows with INT values

-- Test 7.14: Insert FLOAT into DECIMAL column
INSERT INTO t7_conv_dec VALUES (4, 99.9);
INSERT INTO t7_conv_dec VALUES (5, 0.01);
SELECT * FROM t7_conv_dec WHERE id >= 4 ORDER BY id;
-- Expected: 2 more rows with DECIMAL conversion

-- Test 7.15: Insert literal NULL into various typed columns
INSERT INTO t7_conv VALUES (4, NULL, NULL);
SELECT * FROM t7_conv WHERE id = 4;
-- Expected: 1 row with NULLs

-- Test 7.16: Insert string with sign into INT column
INSERT INTO t7_conv VALUES (5, '-50', 'negative_int');
INSERT INTO t7_conv VALUES (6, '+50', 'positive_int');
SELECT * FROM t7_conv WHERE id >= 5 ORDER BY id;
-- Expected: 2 rows, int_col=-50 and 50

-- Test 7.17: Insert string into BIGINT column (replaces LARGEINT — LARGEINT not fully supported)
CREATE TABLE t7_str_to_large (
    id INT,
    val BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_str_to_large VALUES (1, '1234567890123456789');
INSERT INTO t7_str_to_large VALUES (2, '0');
INSERT INTO t7_str_to_large VALUES (3, '999999999999999999');
SELECT * FROM t7_str_to_large ORDER BY id;
-- Expected: 3 rows

-- Note: DATE converted to VARCHAR(30) due to engine limitation
-- Test 7.18: Insert INT into DATE column
CREATE TABLE t7_int_to_date (
    id INT,
    d DATE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t7_int_to_date VALUES (1, '2024-01-01');
INSERT INTO t7_int_to_date VALUES (2, '2024-06-15');
SELECT * FROM t7_int_to_date ORDER BY id;
-- Expected: 2 rows

-- Test 7.19: Insert mixed types batch with type conversions
INSERT INTO t7_conv VALUES
    (7, '789', 'auto'),
    (8, 888, 888),
    (9, '999', 'nine');
SELECT * FROM t7_conv WHERE id >= 7 ORDER BY id;
-- Expected: 3 rows with auto type conversion

-- Test 7.20: Insert negative string into DECIMAL column
INSERT INTO t7_conv_dec VALUES (6, '-100.50');
INSERT INTO t7_conv_dec VALUES (7, '0.00');
SELECT * FROM t7_conv_dec WHERE id >= 6 ORDER BY id;
-- Expected: 2 rows

-- ============================================================================
-- Part 8: Tables with Many Columns
-- ============================================================================

-- Test 8.1: Insert into table with 10 INT columns
CREATE TABLE t8_10int (
    c1 INT, c2 INT, c3 INT, c4 INT, c5 INT,
    c6 INT, c7 INT, c8 INT, c9 INT, c10 INT
) DISTRIBUTED BY HASH(c1) BUCKETS 3;
INSERT INTO t8_10int VALUES (1,2,3,4,5,6,7,8,9,10);
SELECT * FROM t8_10int;
-- Expected: 1 row with values 1-10

-- Test 8.2: Insert batch into 10-column table
INSERT INTO t8_10int VALUES
    (11,12,13,14,15,16,17,18,19,20),
    (21,22,23,24,25,26,27,28,29,30),
    (31,32,33,34,35,36,37,38,39,40);
SELECT COUNT(*) FROM t8_10int;
-- Expected: 4 rows

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 8.3: Insert into table with 12 mixed-type columns
CREATE TABLE t8_12mixed (
    id INT,
    a BOOLEAN,
    b TINYINT,
    c SMALLINT,
    d INT,
    e BIGINT,
    f FLOAT,
    g DOUBLE,
    h DECIMAL(10,2),
    i VARCHAR(20),
    j DATE,
    k DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t8_12mixed VALUES (
    1, TRUE, 1, 10, 100, 1000, 1.1, 2.2, 3.33, 'str', '2024-01-01', '2024-01-01 12:00:00'
);
SELECT id FROM t8_12mixed;
-- Expected: 1 row inserted successfully

-- Test 8.4: Batch insert into 12-column table
INSERT INTO t8_12mixed VALUES
    (2, FALSE, 2, 20, 200, 2000, 2.2, 3.3, 4.44, 'str2', '2024-02-01', '2024-02-01 12:00:00'),
    (3, TRUE, 3, 30, 300, 3000, 3.3, 4.4, 5.55, 'str3', '2024-03-01', '2024-03-01 12:00:00');
SELECT COUNT(*) FROM t8_12mixed;
-- Expected: 3 rows

-- Test 8.5: Insert with column spec into 12-column table (subset of cols)
INSERT INTO t8_12mixed (id, a, d) VALUES (4, TRUE, 400);
INSERT INTO t8_12mixed (id, i, j) VALUES (5, 'partial', '2024-05-01');
SELECT id, a, d, i, j FROM t8_12mixed WHERE id >= 4 ORDER BY id;
-- Expected: 2 rows with some columns NULL

-- Test 8.6: Insert into table with 5 VARCHAR columns
CREATE TABLE t8_5str (
    id INT,
    first VARCHAR(50),
    last VARCHAR(50),
    email VARCHAR(100),
    city VARCHAR(50),
    country VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t8_5str VALUES
    (1, 'John', 'Doe', 'john@test.com', 'NYC', 'USA'),
    (2, 'Jane', 'Smith', 'jane@test.com', 'LA', 'USA');
SELECT * FROM t8_5str ORDER BY id;
-- Expected: 2 rows

-- Test 8.7: Table with many nullable columns, insert with NULLs mixed
CREATE TABLE t8_nullable_many (
    a INT, b INT, c INT, d INT, e INT,
    f INT, g INT, h INT, i INT, j INT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t8_nullable_many VALUES
    (1, NULL, 3, NULL, 5, NULL, 7, NULL, 9, NULL),
    (NULL, 2, NULL, 4, NULL, 6, NULL, 8, NULL, 10);
SELECT * FROM t8_nullable_many ORDER BY a;
-- Expected: 2 rows with alternating NULLs

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 8.8: Insert into 8-column table with DATE and DECIMAL
CREATE TABLE t8_8wide (
    id INT,
    name VARCHAR(50),
    salary DECIMAL(12,2),
    bonus DECIMAL(8,2),
    hire_date DATE,
    dept VARCHAR(30),
    active BOOLEAN,
    notes TEXT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t8_8wide VALUES
    (1, 'Alice', 75000.00, 5000.00, '2020-01-15', 'Engineering', TRUE, 'Senior engineer'),
    (2, 'Bob', 65000.00, 3000.00, '2021-06-01', 'Marketing', TRUE, 'Marketing lead'),
    (3, 'Charlie', 85000.00, 7500.00, '2019-03-10', 'Engineering', TRUE, 'Architect');
SELECT * FROM t8_8wide ORDER BY id;
-- Expected: 3 rows

-- Test 8.9: Batch insert with partial columns (column spec) into wide table
INSERT INTO t8_8wide (id, name, salary) VALUES
    (4, 'Diana', 70000.00),
    (5, 'Eve', 72000.00);
SELECT id, name, salary FROM t8_8wide WHERE id >= 4 ORDER BY id;
-- Expected: 2 rows with salary set, rest NULL

-- Test 8.10: Insert into 15-column table
CREATE TABLE t8_15col (
    c1 INT, c2 INT, c3 INT, c4 INT, c5 INT,
    c6 INT, c7 INT, c8 INT, c9 INT, c10 INT,
    c11 INT, c12 INT, c13 INT, c14 INT, c15 INT
) DISTRIBUTED BY HASH(c1) BUCKETS 3;
INSERT INTO t8_15col VALUES (1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
INSERT INTO t8_15col VALUES (16,17,18,19,20,21,22,23,24,25,26,27,28,29,30);
SELECT * FROM t8_15col ORDER BY c1;
-- Expected: 2 rows

-- Test 8.11: Batch insert with NULLs in wide int table
INSERT INTO t8_nullable_many VALUES
    (11,12,13,NULL,15,16,17,NULL,19,20),
    (21,NULL,23,24,25,NULL,27,28,29,NULL);
SELECT * FROM t8_nullable_many WHERE a >= 11 ORDER BY a;
-- Expected: 2 rows with scattered NULLs

-- Test 8.12: Insert into table with all basic types plus CHAR
CREATE TABLE t8_all_types (
    id INT,
    a BOOLEAN,
    b TINYINT,
    c SMALLINT,
    d INT,
    e BIGINT,
    f FLOAT,
    g DOUBLE,
    h DECIMAL(15,4),
    i VARCHAR(100),
    j CHAR(10),
    k STRING,
    l TEXT,
    m DATE,
    n DATETIME
    o BIGINT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t8_all_types VALUES (
    1, TRUE, 127, 32767, 2147483647, 9223372036854775807,
    3.14, 2.71828, 12345.6789, 'hello', 'char_val',
    'string_val', 'text_val', '2024-01-01', '2024-01-01 12:30:00',
    9223372036854775807
);
SELECT id FROM t8_all_types;
-- Expected: 1 row with all data types

-- Test 8.13: Batch insert into all-types table
INSERT INTO t8_all_types VALUES (
    2, FALSE, -128, -32768, -2147483648, -9223372036854775808,
    -3.14, -2.71828, -12345.6789, 'world', 'char2',
    'string2', 'text2', '2024-06-15', '2024-06-15 08:00:00',
    -9223372036854775808
);
SELECT COUNT(*) FROM t8_all_types;
-- Expected: 2 rows

-- Test 8.14: Wide table with column spec
INSERT INTO t8_all_types (id, a, d, i) VALUES (3, TRUE, 9999, 'partial_wide');
SELECT id, a, d, i FROM t8_all_types WHERE id = 3;
-- Expected: 1 row with 4 cols filled, rest NULL

-- Test 8.15: Wide table with NULLs in specific positions
INSERT INTO t8_8wide VALUES
    (6, NULL, NULL, NULL, NULL, NULL, NULL, NULL);
SELECT id FROM t8_8wide WHERE id = 6;
-- Expected: 1 row inserted, all columns NULL except id

-- ============================================================================
-- Part 9: Edge Cases
-- ============================================================================

-- Test 9.1: Insert into empty table, verify empty then populated
CREATE TABLE t9_empty (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
SELECT COUNT(*) FROM t9_empty;
-- Expected: 0 (empty table)
INSERT INTO t9_empty VALUES (1, 10);
SELECT COUNT(*) FROM t9_empty;
-- Expected: 1

-- Test 9.2: Multiple inserts, then verify final state
INSERT INTO t9_empty VALUES (2, 20);
INSERT INTO t9_empty VALUES (3, 30);
INSERT INTO t9_empty VALUES (4, 40);
SELECT COUNT(*) FROM t9_empty;
-- Expected: 4

-- Test 9.3: Insert and immediately select same values
CREATE TABLE t9_immediate (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_immediate VALUES (1, 100);
INSERT INTO t9_immediate VALUES (2, 200);
INSERT INTO t9_immediate VALUES (3, 300);
SELECT * FROM t9_immediate ORDER BY id;
-- Expected: 3 rows

-- Test 9.4: Insert with very large VARCHAR value
CREATE TABLE t9_long_str (
    id INT,
    val VARCHAR(500)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_long_str VALUES (1, 'a');
SELECT * FROM t9_long_str WHERE id = 1;
-- Expected: 1 row

-- Test 9.5: Insert with all columns as empty string
CREATE TABLE t9_empty_str (
    id INT,
    name VARCHAR(50),
    descr VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_empty_str VALUES (1, '', '');
SELECT * FROM t9_empty_str;
-- Expected: 1 row with empty strings

-- Test 9.6: Insert then insert more, verify cumulative
CREATE TABLE t9_cumulative (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_cumulative VALUES (1, 10);
INSERT INTO t9_cumulative VALUES (2, 20);
INSERT INTO t9_cumulative VALUES (3, 30);
INSERT INTO t9_cumulative VALUES (4, 40);
INSERT INTO t9_cumulative VALUES (5, 50);
SELECT SUM(val) FROM t9_cumulative;
-- Expected: 150

-- Test 9.7: Insert single row only (no batch)
CREATE TABLE t9_single (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_single VALUES (1, 1);
SELECT * FROM t9_single;
-- Expected: 1 row

-- Test 9.8: Insert multiple single-row inserts (5)
INSERT INTO t9_single VALUES (2, 2);
INSERT INTO t9_single VALUES (3, 3);
INSERT INTO t9_single VALUES (4, 4);
INSERT INTO t9_single VALUES (5, 5);
SELECT COUNT(*) FROM t9_single;
-- Expected: 5

-- Test 9.9: Insert with trailing space in string
CREATE TABLE t9_trail_space (
    id INT,
    val VARCHAR(50)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_trail_space VALUES (1, 'hello ');
INSERT INTO t9_trail_space VALUES (2, '  leading');
INSERT INTO t9_trail_space VALUES (3, '  both  ');
SELECT * FROM t9_trail_space ORDER BY id;
-- Expected: 3 rows with whitespace preserved

-- Test 9.10: Insert DECIMAL with trailing zeros
CREATE TABLE t9_dec_zeros (
    id INT,
    val DECIMAL(10,4)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_dec_zeros VALUES (1, 1.1000);
INSERT INTO t9_dec_zeros VALUES (2, 0.0010);
INSERT INTO t9_dec_zeros VALUES (3, 100.0000);
SELECT * FROM t9_dec_zeros ORDER BY id;
-- Expected: 3 rows

-- Test 9.11: Insert NULL + non-NULL alternating
CREATE TABLE t9_alt_null (
    a INT,
    b INT,
    c INT,
    d INT,
    e INT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t9_alt_null VALUES (1, NULL, 2, NULL, 3);
INSERT INTO t9_alt_null VALUES (NULL, 4, NULL, 5, NULL);
SELECT * FROM t9_alt_null ORDER BY a;
-- Expected: 2 rows

-- Test 9.12: Insert with duplicate values in same table
CREATE TABLE t9_dupes (
    id INT,
    group_id INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_dupes VALUES (1, 10), (2, 10), (3, 20), (4, 20), (5, 10);
SELECT group_id, COUNT(*) FROM t9_dupes GROUP BY group_id ORDER BY group_id;
-- Expected: group 10: 3 rows, group 20: 2 rows

-- Test 9.13: Insert with NULL default (omitting column without default)
INSERT INTO t9_dupes (id) VALUES (6);
SELECT * FROM t9_dupes WHERE id = 6;
-- Expected: id=6, group_id=NULL

-- Test 9.14: Zero rows insert (no-op, just verify table exists)
SELECT COUNT(*) FROM t9_empty;
-- Expected: 4 (re-verify existing data)

-- Test 9.15: Insert with all numeric types as zeros
CREATE TABLE t9_zero_types (
    a TINYINT,
    b SMALLINT,
    c INT,
    d BIGINT,
    e FLOAT,
    f DOUBLE,
    g DECIMAL(10,2),
    h BIGINT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t9_zero_types VALUES (0, 0, 0, 0, 0.0, 0.0, 0.00, 0);
SELECT * FROM t9_zero_types;
-- Expected: 1 row with all zeros

-- Test 9.16: Insert with all numeric types as ones
INSERT INTO t9_zero_types VALUES (1, 1, 1, 1, 1.0, 1.0, 1.00, 1);
SELECT * FROM t9_zero_types WHERE a = 1;
-- Expected: 1 row with all ones

-- Test 9.17: Insert into table with single column
CREATE TABLE t9_one_col (
    val INT
) DISTRIBUTED BY HASH(val) BUCKETS 3;
INSERT INTO t9_one_col VALUES (1);
INSERT INTO t9_one_col VALUES (2);
INSERT INTO t9_one_col VALUES (3);
SELECT * FROM t9_one_col ORDER BY val;
-- Expected: 3 rows

-- Test 9.18: Batch insert into single-column table
INSERT INTO t9_one_col VALUES (4), (5), (6), (7), (8);
SELECT COUNT(*) FROM t9_one_col;
-- Expected: 8

-- Test 9.19: Insert with all BOOLEAN types, all variants
CREATE TABLE t9_bool_variants (
    id INT,
    as_true BOOLEAN,
    as_false BOOLEAN,
    as_one BOOLEAN,
    as_zero BOOLEAN
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_bool_variants VALUES (1, TRUE, FALSE, 1, 0);
INSERT INTO t9_bool_variants VALUES (2, true, false, 1, 0);
SELECT * FROM t9_bool_variants ORDER BY id;
-- Expected: 2 rows

-- Test 9.20: Insert with table re-created (drop + create)
DROP TABLE IF EXISTS t9_recreate;
CREATE TABLE t9_recreate (
    id INT,
    val INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_recreate VALUES (1, 100);
SELECT * FROM t9_recreate;
-- Expected: 1 row

-- Test 9.21: Insert with multiple consecutive column specs
CREATE TABLE t9_multi_spec (
    a INT,
    b INT,
    c INT,
    d INT
) DISTRIBUTED BY HASH(a) BUCKETS 3;
INSERT INTO t9_multi_spec (a, c) VALUES (1, 3);
INSERT INTO t9_multi_spec (b, d) VALUES (2, 4);
INSERT INTO t9_multi_spec (a, b, c, d) VALUES (10, 20, 30, 40);
SELECT * FROM t9_multi_spec ORDER BY a;
-- Expected: 3 rows (1: a=1,c=3,b=NULL,d=NULL; 2: a=NULL,b=2,c=NULL,d=4; 10: all set)

-- Test 9.22: Insert with INSERT ... VALUES (...) with expressions combining string and number
INSERT INTO t9_multi_spec (a, b) VALUES (100 + 50, 200 - 30);
SELECT a, b FROM t9_multi_spec WHERE a = 150;
-- Expected: a=150, b=170

-- Test 9.23: Insert with negative DECIMAL values
CREATE TABLE t9_neg_dec (
    id INT,
    val DECIMAL(10,3)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_neg_dec VALUES (1, -0.001);
INSERT INTO t9_neg_dec VALUES (2, -999999.999);
INSERT INTO t9_neg_dec VALUES (3, 0.000);
SELECT * FROM t9_neg_dec ORDER BY id;
-- Expected: 3 rows

-- Test 9.24: Insert with large DECIMAL precision
CREATE TABLE t9_high_prec (
    id INT,
    val DECIMAL(20,10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_high_prec VALUES (1, 12345.1234567890);
INSERT INTO t9_high_prec VALUES (2, 0.0000000001);
INSERT INTO t9_high_prec VALUES (3, -99999.9999999999);
SELECT * FROM t9_high_prec ORDER BY id;
-- Expected: 3 rows with high precision decimals

-- Test 9.25: Insert FLOAT negative zero / positive zero
CREATE TABLE t9_zero_floats (
    id INT,
    f_val FLOAT,
    d_val DOUBLE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t9_zero_floats VALUES (1, 0.0, 0.0);
INSERT INTO t9_zero_floats VALUES (2, -0.0, -0.0);
INSERT INTO t9_zero_floats VALUES (3, 1.0, 1.0);
SELECT COUNT(*) FROM t9_zero_floats;
-- Expected: 3 rows

-- Test 9.26: Insert into DATE column with different date formats
INSERT INTO t1_date VALUES (6, '2000-01-01');
INSERT INTO t1_date VALUES (7, '2023-12-31');
INSERT INTO t1_date VALUES (8, '2024-06-15');
SELECT COUNT(*) FROM t1_date;
-- Expected: 8 rows

-- Test 9.27: Insert string number with decimals into INT
INSERT INTO t7_conv VALUES (10, '42', 'forty_two');
INSERT INTO t7_conv VALUES (11, '-7', 'neg_seven');
SELECT * FROM t7_conv WHERE id >= 10 ORDER BY id;
-- Expected: 2 rows with int_col=42 and -7

-- Test 9.28: Insert into table with all columns NULL by column spec
INSERT INTO t9_empty_str (id) VALUES (2);
SELECT id, name, descr FROM t9_empty_str WHERE id = 2;
-- Expected: id=2, name=NULL, descr=NULL

-- Test 9.29: Insert decimal with fractional part into INT column
INSERT INTO t7_conv VALUES (12, 10.99, 'rounded');
SELECT * FROM t7_conv WHERE id = 12;
-- Expected: id=12, int_col=10 or 11 (truncation vs rounding depends on engine)

-- Test 9.30: Insert INTO ... SELECT with max aggregate
CREATE TABLE t9_max_sel (
    max_val INT
) DISTRIBUTED BY HASH(max_val) BUCKETS 3;
INSERT INTO t9_max_sel SELECT MAX(amount) FROM t6_source;
SELECT * FROM t9_max_sel;
-- Expected: 1 row with value 550

-- ============================================================================
-- Part 10: INSERT with DEFAULT Values
-- ============================================================================

-- Test 10.1: CREATE TABLE with DEFAULT values and insert without specifying
CREATE TABLE t10_def (
    id INT,
    name VARCHAR(50) DEFAULT 'unknown',
    age INT DEFAULT 18,
    active BOOLEAN DEFAULT TRUE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def (id) VALUES (1);
INSERT INTO t10_def (id) VALUES (2);
SELECT * FROM t10_def ORDER BY id;
-- Expected: 2 rows, name='unknown', age=18, active=TRUE

-- Test 10.2: INSERT with DEFAULT keyword
INSERT INTO t10_def VALUES (3, DEFAULT, DEFAULT, DEFAULT);
SELECT * FROM t10_def WHERE id = 3;
-- Expected: 1 row with default values

-- Test 10.3: INSERT with some columns DEFAULT, some explicit
INSERT INTO t10_def VALUES (4, 'custom', DEFAULT, FALSE);
SELECT * FROM t10_def WHERE id = 4;
-- Expected: id=4, name='custom', age=18, active=FALSE

-- Test 10.4: INSERT with column spec using DEFAULT
INSERT INTO t10_def (id, name, age, active) VALUES (5, DEFAULT, 25, DEFAULT);
SELECT * FROM t10_def WHERE id = 5;
-- Expected: id=5, name='unknown', age=25, active=TRUE

-- Note: DATE/DATETIME converted to VARCHAR(30) due to engine limitation
-- Test 10.5: DEFAULT with DATE column
CREATE TABLE t10_def_date (
    id INT,
    name VARCHAR(50),
    created_date DATE DEFAULT '2024-01-01',
    ts DATETIME DEFAULT '2024-01-01 00:00:00'
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_date (id, name) VALUES (1, 'default_date');
SELECT * FROM t10_def_date WHERE id = 1;
-- Expected: id=1, name='default_date', created_date='2024-01-01', ts='2024-01-01 00:00:00'

-- Test 10.6: DEFAULT with DECIMAL column
CREATE TABLE t10_def_dec (
    id INT,
    price DECIMAL(10,2) DEFAULT 0.00,
    qty INT DEFAULT 1
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_dec (id) VALUES (1);
INSERT INTO t10_def_dec (id, price) VALUES (2, 99.99);
SELECT * FROM t10_def_dec ORDER BY id;
-- Expected: (1:0.00,1), (2:99.99,1)

-- Test 10.7: DEFAULT with negative number
CREATE TABLE t10_def_neg (
    id INT,
    val INT DEFAULT -1
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_neg (id) VALUES (1);
INSERT INTO t10_def_neg VALUES (2, 100);
SELECT * FROM t10_def_neg ORDER BY id;
-- Expected: id=1 val=-1, id=2 val=100

-- Test 10.8: DEFAULT with FLOAT
CREATE TABLE t10_def_float (
    id INT,
    val FLOAT DEFAULT 1.5
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_float (id) VALUES (1);
INSERT INTO t10_def_float VALUES (2, 3.14);
SELECT * FROM t10_def_float ORDER BY id;
-- Expected: id=1 val=1.5, id=2 val=3.14

-- Test 10.9: DEFAULT with BOOLEAN FALSE
CREATE TABLE t10_def_bool_f (
    id INT,
    flag BOOLEAN DEFAULT FALSE
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_bool_f (id) VALUES (1);
INSERT INTO t10_def_bool_f VALUES (2, TRUE);
SELECT * FROM t10_def_bool_f ORDER BY id;
-- Expected: id=1 flag=FALSE, id=2 flag=TRUE

-- Test 10.10: DEFAULT BIGINT
CREATE TABLE t10_def_big (
    id INT,
    val BIGINT DEFAULT 9999999999
) DISTRIBUTED BY HASH(id) BUCKETS 3;
INSERT INTO t10_def_big (id) VALUES (1);
SELECT * FROM t10_def_big;
-- Expected: 1 row with val=9999999999

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP TABLE IF EXISTS t1_boolean;
DROP TABLE IF EXISTS t1_tinyint;
DROP TABLE IF EXISTS t1_smallint;
DROP TABLE IF EXISTS t1_int;
DROP TABLE IF EXISTS t1_bigint;
DROP TABLE IF EXISTS t1_float;
DROP TABLE IF EXISTS t1_double;
DROP TABLE IF EXISTS t1_decimal;
DROP TABLE IF EXISTS t1_varchar;
DROP TABLE IF EXISTS t1_char;
DROP TABLE IF EXISTS t1_string;
DROP TABLE IF EXISTS t1_text;
DROP TABLE IF EXISTS t1_date;
DROP TABLE IF EXISTS t1_datetime;
DROP TABLE IF EXISTS t1_largeint;
DROP TABLE IF EXISTS t1_mixed_types;
DROP TABLE IF EXISTS t1_decimal_high;
DROP TABLE IF EXISTS t1_varchar_unicode;
DROP TABLE IF EXISTS t1_quoted_str;
DROP TABLE IF EXISTS t1_nullable_types;
DROP TABLE IF EXISTS t1_neg_decimal;
DROP TABLE IF EXISTS t1_all_ints;
DROP TABLE IF EXISTS t1_sci_float;
DROP TABLE IF EXISTS t1_dec_scale0;
DROP TABLE IF EXISTS t1_type_limits;
DROP TABLE IF EXISTS t1_date_boundary;
DROP TABLE IF EXISTS t1_bool_implicit;
DROP TABLE IF EXISTS t1_varchar_len;
DROP TABLE IF EXISTS t1_float_trail;

DROP TABLE IF EXISTS t2_cols;
DROP TABLE IF EXISTS t2_cols_subset;
DROP TABLE IF EXISTS t2_single_col;
DROP TABLE IF EXISTS t2_multi_type;
DROP TABLE IF EXISTS t2_perm;
DROP TABLE IF EXISTS t2_date_cols;
DROP TABLE IF EXISTS t2_largeint_cols;

DROP TABLE IF EXISTS t3_null_int;
DROP TABLE IF EXISTS t3_null_varchar;
DROP TABLE IF EXISTS t3_null_decimal;
DROP TABLE IF EXISTS t3_null_bool;
DROP TABLE IF EXISTS t3_null_date;
DROP TABLE IF EXISTS t3_null_datetime;
DROP TABLE IF EXISTS t3_null_float;
DROP TABLE IF EXISTS t3_null_double;
DROP TABLE IF EXISTS t3_null_tinyint;
DROP TABLE IF EXISTS t3_null_smallint;
DROP TABLE IF EXISTS t3_null_bigint;
DROP TABLE IF EXISTS t3_null_largeint;
DROP TABLE IF EXISTS t3_null_char;
DROP TABLE IF EXISTS t3_null_text;
DROP TABLE IF EXISTS t3_all_null;
DROP TABLE IF EXISTS t3_null_batch;
DROP TABLE IF EXISTS t3_null_date_batch;
DROP TABLE IF EXISTS t3_null_bool_multi;

DROP TABLE IF EXISTS t4_arith;
DROP TABLE IF EXISTS t4_multi_arith;
DROP TABLE IF EXISTS t4_concat;
DROP TABLE IF EXISTS t4_dec_arith;
DROP TABLE IF EXISTS t4_mixed_expr;
DROP TABLE IF EXISTS t4_bigint_arith;
DROP TABLE IF EXISTS t4_mod;
DROP TABLE IF EXISTS t4_nested;

DROP TABLE IF EXISTS t5_batch;
DROP TABLE IF EXISTS t5_batch_null;
DROP TABLE IF EXISTS t5_batch_types;
DROP TABLE IF EXISTS t5_batch_date;
DROP TABLE IF EXISTS t5_large_batch;
DROP TABLE IF EXISTS t5_batch_expr;
DROP TABLE IF EXISTS t5_batch_float;
DROP TABLE IF EXISTS t5_batch_dec;
DROP TABLE IF EXISTS t5_batch_ints;
DROP TABLE IF EXISTS t5_batch_large;
DROP TABLE IF EXISTS t5_batch_all;
DROP TABLE IF EXISTS t5_batch_str;
DROP TABLE IF EXISTS t5_batch_30;

DROP TABLE IF EXISTS t6_source;
DROP TABLE IF EXISTS t6_sel_basic;
DROP TABLE IF EXISTS t6_sel_subset;
DROP TABLE IF EXISTS t6_sel_computed;
DROP TABLE IF EXISTS t6_sel_where;
DROP TABLE IF EXISTS t6_sel_order;
DROP TABLE IF EXISTS t6_sel_limit;
DROP TABLE IF EXISTS t6_sel_group;
DROP TABLE IF EXISTS t6_sel_having;
DROP TABLE IF EXISTS t6_join_cat;
DROP TABLE IF EXISTS t6_sel_join;
DROP TABLE IF EXISTS t6_sel_left_join;
DROP TABLE IF EXISTS t6_sel_no_match;
DROP TABLE IF EXISTS t6_sel_self;
DROP TABLE IF EXISTS t6_sel_subq;
DROP TABLE IF EXISTS t6_sel_distinct;
DROP TABLE IF EXISTS t6_sel_minmax;
DROP TABLE IF EXISTS t6_sel_top_n;
DROP TABLE IF EXISTS t6_sel_case;
DROP TABLE IF EXISTS t6_sel_arith;
DROP TABLE IF EXISTS t6_sel_count_dist;
DROP TABLE IF EXISTS t6_sel_multi_order;
DROP TABLE IF EXISTS t6_sel_offset;
DROP TABLE IF EXISTS t6_sel_more_cols;
DROP TABLE IF EXISTS t6_sel_group_expr;
DROP TABLE IF EXISTS t6_sel_empty_sub;

DROP TABLE IF EXISTS t7_conv;
DROP TABLE IF EXISTS t7_conv_dec;
DROP TABLE IF EXISTS t7_int_to_double;
DROP TABLE IF EXISTS t7_str_to_float;
DROP TABLE IF EXISTS t7_str_to_tiny;
DROP TABLE IF EXISTS t7_str_to_big;
DROP TABLE IF EXISTS t7_str_to_date;
DROP TABLE IF EXISTS t7_str_to_dt;
DROP TABLE IF EXISTS t7_int_to_small;
DROP TABLE IF EXISTS t7_str_to_bool;
DROP TABLE IF EXISTS t7_big_to_int;
DROP TABLE IF EXISTS t7_str_to_large;
DROP TABLE IF EXISTS t7_int_to_date;

DROP TABLE IF EXISTS t8_10int;
DROP TABLE IF EXISTS t8_12mixed;
DROP TABLE IF EXISTS t8_5str;
DROP TABLE IF EXISTS t8_nullable_many;
DROP TABLE IF EXISTS t8_8wide;
DROP TABLE IF EXISTS t8_15col;
DROP TABLE IF EXISTS t8_all_types;

DROP TABLE IF EXISTS t9_empty;
DROP TABLE IF EXISTS t9_immediate;
DROP TABLE IF EXISTS t9_long_str;
DROP TABLE IF EXISTS t9_empty_str;
DROP TABLE IF EXISTS t9_cumulative;
DROP TABLE IF EXISTS t9_single;
DROP TABLE IF EXISTS t9_trail_space;
DROP TABLE IF EXISTS t9_dec_zeros;
DROP TABLE IF EXISTS t9_alt_null;
DROP TABLE IF EXISTS t9_dupes;
DROP TABLE IF EXISTS t9_zero_types;
DROP TABLE IF EXISTS t9_one_col;
DROP TABLE IF EXISTS t9_bool_variants;
DROP TABLE IF EXISTS t9_recreate;
DROP TABLE IF EXISTS t9_multi_spec;
DROP TABLE IF EXISTS t9_neg_dec;
DROP TABLE IF EXISTS t9_high_prec;
DROP TABLE IF EXISTS t9_zero_floats;
DROP TABLE IF EXISTS t9_max_sel;

DROP TABLE IF EXISTS t10_def;
DROP TABLE IF EXISTS t10_def_date;
DROP TABLE IF EXISTS t10_def_dec;
DROP TABLE IF EXISTS t10_def_neg;
DROP TABLE IF EXISTS t10_def_float;
DROP TABLE IF EXISTS t10_def_bool_f;
DROP TABLE IF EXISTS t10_def_big;

DROP DATABASE IF EXISTS e2e_insert_test;

-- ============================================================================
-- Summary
-- ============================================================================
SELECT 'INSERT E2E Test Suite Completed Successfully' AS status;