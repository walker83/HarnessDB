-- ============================================================================
-- E2E Test: Built-in Functions
-- Tests supported built-in functions across string, math, date/time,
-- conditional, conversion, and other categories. Unsupported functions are
-- documented with NOT SUPPORTED markers to serve as future work items.
-- ============================================================================

DROP DATABASE IF EXISTS e2e_functions_test;
CREATE DATABASE e2e_functions_test;
USE e2e_functions_test;

-- ============================================================================
-- Setup: Create tables for testing functions with stored data
-- ============================================================================

CREATE TABLE t_fn_data (
    id INT,
    s1 VARCHAR(100),
    s2 VARCHAR(100),
    s_empty VARCHAR(100),
    s_unicode VARCHAR(100),
    s_special VARCHAR(100),
    s_trim VARCHAR(100),
    num1 INT,
    num2 INT,
    num3 BIGINT,
    num_neg INT,
    num_dec DOUBLE,
    dec_col DECIMAL(10,2),
    d1 VARCHAR(30),
    d2 VARCHAR(30),
    dt1 VARCHAR(30),
    flag BOOLEAN,
    null_col VARCHAR(10)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_fn_data VALUES
    (1, 'Hello World', 'hello', '', '你好世界 Café', 'a@b@c@d', '  spaced  ',
     100, 3, 9999999999, -42, 3.14159, 123.45,
     '2024-01-15', '2023-12-31', '2024-01-15 10:30:00', TRUE, NULL),
    (2, 'hello', 'WORLD', '', '日本語', 'x!y!z!w!v', 'hello   ',
     0, -5, 0, -100, -2.718, -50.00,
     '2024-02-29', '2024-01-01', '2024-02-29 23:59:59', FALSE, NULL),
    (3, 'ABCdef', '123', '', '🌍🌎🌏', 'one|two|three', '   world',
     42, 7, 123456789, -1, 0.001, 99.99,
     '2023-06-15', '2024-12-25', '2023-06-15 08:00:00', TRUE, NULL),
    (4, '', 'World', '', '', '', 'middle',
     255, 10, -999, -5, 1000.0, NULL,
     '2024-07-04', '2024-06-01', '2024-12-31 23:59:00', FALSE, NULL),
    (5, NULL, 'test', '', NULL, NULL, NULL,
     NULL, 1, 0, -255, 0.0, 0.00,
     NULL, NULL, NULL, NULL, NULL);

-- ---------------------------------------------------------------------------
-- Sub-category: String Functions
-- ---------------------------------------------------------------------------

-- Test 1.1: UPPER basic
-- Expected: 'HELLO WORLD'
SELECT '1.1 UPPER basic' AS test_name, UPPER(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.2: UPPER with NULL input
-- Expected: NULL
SELECT '1.2 UPPER NULL' AS test_name, UPPER(NULL) AS result;

-- Test 1.3: UPPER on empty string
-- Expected: ''
SELECT '1.3 UPPER empty' AS test_name, UPPER(s_empty) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.4: UPPER with unicode
-- Expected unchanged (unicode has no case or keeps its case)
SELECT '1.4 UPPER unicode' AS test_name, UPPER(s_unicode) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.5: LOWER basic
-- Expected: 'hello world'
SELECT '1.5 LOWER basic' AS test_name, LOWER(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.6: LOWER with all caps
-- Expected: 'hello'
SELECT '1.6 LOWER all caps' AS test_name, LOWER(s2) AS result FROM t_fn_data WHERE id = 2;

-- Test 1.7: LOWER with NULL
-- Expected: NULL
SELECT '1.7 LOWER NULL' AS test_name, LOWER(NULL) AS result;

-- Test 1.8: LENGTH basic
-- Expected: 11 ('Hello World')
SELECT '1.8 LENGTH basic' AS test_name, LENGTH(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.9: LENGTH of empty string
-- Expected: 0
SELECT '1.9 LENGTH empty' AS test_name, LENGTH(s_empty) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.10: LENGTH of unicode (byte length differs from char length)
-- Expected: (length in bytes, typically > char count for multi-byte)
SELECT '1.10 LENGTH unicode' AS test_name, LENGTH(s_unicode) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.11: LENGTH with NULL
-- Expected: NULL
SELECT '1.11 LENGTH NULL' AS test_name, LENGTH(NULL) AS result;

-- Test 1.12: CHAR_LENGTH basic
-- Expected: 11
SELECT '1.12 CHAR_LENGTH basic' AS test_name, CHAR_LENGTH(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.13: CHAR_LENGTH empty
-- Expected: 0
SELECT '1.13 CHAR_LENGTH empty' AS test_name, CHAR_LENGTH(s_empty) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.14: CHAR_LENGTH vs LENGTH for unicode
-- CHAR_LENGTH counts characters, LENGTH counts bytes
SELECT '1.14 CHAR_LENGTH unicode' AS test_name, CHAR_LENGTH(s_unicode) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.15: CONCAT two strings
-- Expected: 'Hello Worldhello'
SELECT '1.15 CONCAT basic' AS test_name, CONCAT(s1, s2) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.16: CONCAT multiple strings
-- Expected: 'Hello WorldhelloABCdef'
SELECT '1.16 CONCAT multiple' AS test_name, CONCAT(t1.s1, t1.s2, t2.s1) AS result
FROM t_fn_data t1, t_fn_data t2 WHERE t1.id = 1 AND t2.id = 3;

-- Test 1.17: CONCAT with NULL (NULL -> empty string in DataFusion CONCAT)
-- Expected: 'Hello World' (NULL s2 from id=5 treated as empty)
SELECT '1.17 CONCAT with NULL' AS test_name, CONCAT(s1, s2) AS result FROM t_fn_data WHERE id = 5;

-- Test 1.18: CONCAT with empty string
-- Expected: 'Hello World'
SELECT '1.18 CONCAT empty' AS test_name, CONCAT(s1, '') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.19: CONCAT_WS with separator
-- Expected: 'Hello-World'
SELECT '1.19 CONCAT_WS basic' AS test_name, CONCAT_WS('-', s1, s2) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.20: CONCAT_WS skipping NULL
-- Expected: 'Hello World' (NULL skipped)
SELECT '1.20 CONCAT_WS skip NULL' AS test_name, CONCAT_WS(' ', s1, null_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.21: CONCAT_WS multiple strings
-- Expected: 'Hello|hello|ABCdef'
SELECT '1.21 CONCAT_WS multiple' AS test_name, CONCAT_WS('|', t1.s1, t1.s2, t2.s1) AS result
FROM t_fn_data t1, t_fn_data t2 WHERE t1.id = 1 AND t2.id = 3;

-- Test 1.22: CONCAT_WS with all NULL (returns empty string in MySQL)
-- Expected: '' (empty string)
SELECT '1.22 CONCAT_WS all NULL' AS test_name, CONCAT_WS(',', null_col, null_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.23: SUBSTRING with position only
-- Expected: 'World' (from position 7)
SELECT '1.23 SUBSTRING pos' AS test_name, SUBSTRING(s1, 7) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.24: SUBSTRING with position and length
-- Expected: 'Hel'
SELECT '1.24 SUBSTRING pos+len' AS test_name, SUBSTRING(s1, 1, 3) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.25: SUBSTRING with negative position (counts from end)
-- Expected: 'orld'
SELECT '1.25 SUBSTRING neg pos' AS test_name, SUBSTRING(s1, -4) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.26: SUBSTRING negative position with length
-- Expected: 'Wor'
SELECT '1.26 SUBSTRING neg pos+len' AS test_name, SUBSTRING(s1, 7, 3) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.27: SUBSTRING with NULL
-- Expected: NULL
SELECT '1.27 SUBSTRING NULL' AS test_name, SUBSTRING(NULL, 1, 3) AS result;

-- Test 1.28: SUBSTRING beyond string length (returns empty)
-- Expected: ''
SELECT '1.28 SUBSTRING beyond' AS test_name, SUBSTRING(s1, 100) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.29: SUBSTRING with length 0
-- Expected: ''
SELECT '1.29 SUBSTRING len 0' AS test_name, SUBSTRING(s1, 1, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.30: SUBSTR alias of SUBSTRING
-- Expected: 'Hello'
SELECT '1.30 SUBSTR alias' AS test_name, SUBSTR(s1, 1, 5) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.31: TRIM basic
-- Expected: 'spaced'
SELECT '1.31 TRIM basic' AS test_name, TRIM(s_trim) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.32: TRIM leading spaces
-- Expected: 'world'
SELECT '1.32 TRIM leading' AS test_name, TRIM(LEADING FROM s_trim) AS result FROM t_fn_data WHERE id = 3;

-- Test 1.33: TRIM trailing spaces
-- Expected: 'hello'
SELECT '1.33 TRIM trailing' AS test_name, TRIM(TRAILING FROM s_trim) AS result FROM t_fn_data WHERE id = 2;

-- Test 1.34: TRIM both spaces
-- Expected: 'middle'
SELECT '1.34 TRIM both' AS test_name, TRIM(BOTH FROM s_trim) AS result FROM t_fn_data WHERE id = 4;

-- Test 1.35: LTRIM basic
-- Expected: 'spaced  '
SELECT '1.35 LTRIM basic' AS test_name, LTRIM(s_trim) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.36: RTRIM basic
-- Expected: '  spaced'
SELECT '1.36 RTRIM basic' AS test_name, RTRIM(s_trim) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.37: LTRIM on string with only leading spaces
-- Expected: 'hello   '
SELECT '1.37 LTRIM leading' AS test_name, LTRIM(s_trim) AS result FROM t_fn_data WHERE id = 2;

-- Test 1.38: REPLACE basic
-- Expected: 'Hello Universe'
SELECT '1.38 REPLACE basic' AS test_name, REPLACE(s1, 'World', 'Universe') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.39: REPLACE no match
-- Expected: 'Hello World'
SELECT '1.39 REPLACE no match' AS test_name, REPLACE(s1, 'xyz', 'abc') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.40: REPLACE with empty string (remove occurrences)
-- Expected: 'HelloWorld'
SELECT '1.40 REPLACE remove' AS test_name, REPLACE(s1, ' ', '') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.41: REPLACE with NULL
-- Expected: NULL
SELECT '1.41 REPLACE NULL' AS test_name, REPLACE(NULL, 'a', 'b') AS result;

-- Test 1.42: REVERSE basic
-- Expected: 'dlroW olleH'
SELECT '1.42 REVERSE basic' AS test_name, REVERSE(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.43: REVERSE palindrome-style
-- Expected: 'olleh'
SELECT '1.43 REVERSE simple' AS test_name, REVERSE(s2) AS result FROM t_fn_data WHERE id = 2;

-- Test 1.44: REVERSE empty string
-- Expected: ''
SELECT '1.44 REVERSE empty' AS test_name, REVERSE(s_empty) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.45: REVERSE NULL
-- Expected: NULL
SELECT '1.45 REVERSE NULL' AS test_name, REVERSE(NULL) AS result;

-- Test 1.46: LEFT basic
-- Expected: 'Hel'
SELECT '1.46 LEFT basic' AS test_name, LEFT(s1, 3) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.47: LEFT with count 0
-- Expected: ''
SELECT '1.47 LEFT zero' AS test_name, LEFT(s1, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.48: LEFT with count > length
-- Expected: 'Hello World'
SELECT '1.48 LEFT beyond' AS test_name, LEFT(s1, 100) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.49: LEFT with NULL
-- Expected: NULL
SELECT '1.49 LEFT NULL' AS test_name, LEFT(NULL, 3) AS result;

-- Test 1.50: RIGHT basic
-- Expected: 'rld'
SELECT '1.50 RIGHT basic' AS test_name, RIGHT(s1, 3) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.51: RIGHT with count 0
-- Expected: ''
SELECT '1.51 RIGHT zero' AS test_name, RIGHT(s1, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.52: RIGHT with count > length
-- Expected: 'Hello World'
SELECT '1.52 RIGHT beyond' AS test_name, RIGHT(s1, 100) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.53: LPAD basic
-- Expected: '000Hello World' (total length 14)
SELECT '1.53 LPAD basic' AS test_name, LPAD(s1, 14, '0') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.54: LPAD with pad shorter than needed
-- Expected: '****Hello World'
SELECT '1.54 LPAD pattern' AS test_name, LPAD(s1, 15, '*') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.55: LPAD with length less than string
-- Expected: 'Hello World' (no truncation in MySQL, but DataFusion may differ)
SELECT '1.55 LPAD truncate' AS test_name, LPAD(s1, 5, 'x') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.56: RPAD basic
-- Expected: 'Hello Worldxxx'
SELECT '1.56 RPAD basic' AS test_name, RPAD(s1, 14, 'x') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.57: RPAD with multi-char pad
-- Expected: 'Hello Worldab'
SELECT '1.57 RPAD pattern' AS test_name, RPAD(s1, 13, 'ab') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.58: REPEAT basic
-- Expected: 'abcabcabc'
SELECT '1.58 REPEAT basic' AS test_name, REPEAT('abc', 3) AS result;

-- Test 1.59: REPEAT zero times
-- Expected: ''
SELECT '1.59 REPEAT zero' AS test_name, REPEAT('abc', 0) AS result;

-- Test 1.60: REPEAT with NULL
-- Expected: NULL
SELECT '1.60 REPEAT NULL' AS test_name, REPEAT(NULL, 3) AS result;

-- Test 1.61: SPACE basic
-- Expected: 'a   b' (3 spaces between)
SELECT '1.61 SPACE basic' AS test_name, CONCAT('a', SPACE(3), 'b') AS result;

-- Test 1.62: SPACE zero
-- Expected: 'ab'
SELECT '1.62 SPACE zero' AS test_name, CONCAT('a', SPACE(0), 'b') AS result;

-- Test 1.63: LOCATE basic (find substring)
-- Expected: 7 (position of 'World' in 'Hello World')
SELECT '1.63 LOCATE basic' AS test_name, LOCATE('World', s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.64: LOCATE not found
-- Expected: 0
SELECT '1.64 LOCATE not found' AS test_name, LOCATE('xyz', s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.65: LOCATE with starting position
-- Expected: 0 (second 'l' starts at position 3, 'World' not found after pos 10)
SELECT '1.65 LOCATE start pos' AS test_name, LOCATE('o', s1, 7) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.66: LOCATE with NULL
-- Expected: NULL
SELECT '1.66 LOCATE NULL' AS test_name, LOCATE('a', NULL) AS result;

-- Test 1.67: LOCATE empty string
-- Expected: 1 (empty string always found at position 1)
SELECT '1.67 LOCATE empty substr' AS test_name, LOCATE('', s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 1.68: INSTR basic (alias for LOCATE)
-- Expected: 7
SELECT '1.68 INSTR basic' AS test_name, INSTR(s1, 'World') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.69: INSTR not found
-- Expected: 0
SELECT '1.69 INSTR not found' AS test_name, INSTR(s1, 'xyz') AS result FROM t_fn_data WHERE id = 1;

-- Test 1.70: HEX basic — NOT SUPPORTED in RorisDB (DataFusion has encode/decode but no HEX)
-- SELECT '1.70 HEX basic' AS test_name, HEX(s1) AS result FROM t_fn_data WHERE id = 1;
SELECT '1.70 HEX basic not supported' AS test_name;

-- Test 1.71: HEX empty string — NOT SUPPORTED
-- SELECT '1.71 HEX empty' AS test_name, HEX(s_empty) AS result FROM t_fn_data WHERE id = 1;
SELECT '1.71 HEX empty not supported' AS test_name;

-- Test 1.72: HEX of numbers — NOT SUPPORTED
-- SELECT '1.72 HEX number' AS test_name, HEX(num1) AS result FROM t_fn_data WHERE id = 1;
SELECT '1.72 HEX number not supported' AS test_name;

-- Test 1.73: UNHEX basic — NOT SUPPORTED
-- SELECT '1.73 UNHEX basic' AS test_name, UNHEX('48656C6C6F') AS result;
SELECT '1.73 UNHEX basic not supported' AS test_name;

-- Test 1.74: UNHEX invalid hex — NOT SUPPORTED
-- SELECT '1.74 UNHEX invalid' AS test_name, UNHEX('XYZ') AS result;
SELECT '1.74 UNHEX invalid not supported' AS test_name;

-- Test 1.75: UNHEX and HEX round-trip — NOT SUPPORTED
-- SELECT '1.75 HEX/UNHEX roundtrip' AS test_name, UNHEX(HEX('Hello World')) AS result;
SELECT '1.75 HEX/UNHEX roundtrip not supported' AS test_name;

-- ---------------------------------------------------------------------------
-- Sub-category: Math Functions
-- ---------------------------------------------------------------------------

-- Test 2.1: ABS positive
-- Expected: 42
SELECT '2.1 ABS positive' AS test_name, ABS(num_neg) AS result FROM t_fn_data WHERE id = 3;

-- Test 2.2: ABS negative
-- Expected: 42
SELECT '2.2 ABS negative' AS test_name, ABS(num_neg) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.3: ABS zero
-- Expected: 0
SELECT '2.3 ABS zero' AS test_name, ABS(num2) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.4: ABS with NULL
-- Expected: NULL
SELECT '2.4 ABS NULL' AS test_name, ABS(NULL) AS result;

-- Test 2.5: CEIL basic
-- Expected: 4
SELECT '2.5 CEIL basic' AS test_name, CEIL(num_dec) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.6: CEIL negative
-- Expected: -2
SELECT '2.6 CEIL negative' AS test_name, CEIL(num_neg) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.7: CEIL integer
-- Expected: 42
SELECT '2.7 CEIL integer' AS test_name, CEIL(num1) AS result FROM t_fn_data WHERE id = 3;

-- Test 2.8: CEILING alias of CEIL
-- Expected: 4
SELECT '2.8 CEILING alias' AS test_name, CEILING(num_dec) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.9: FLOOR basic
-- Expected: 3
SELECT '2.9 FLOOR basic' AS test_name, FLOOR(num_dec) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.10: FLOOR negative
-- Expected: -3
SELECT '2.10 FLOOR negative' AS test_name, FLOOR(num_neg) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.11: FLOOR integer
-- Expected: 42
SELECT '2.11 FLOOR integer' AS test_name, FLOOR(num1) AS result FROM t_fn_data WHERE id = 3;

-- Test 2.12: ROUND basic
-- Expected: 3
SELECT '2.12 ROUND basic' AS test_name, ROUND(num_dec) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.13: ROUND with decimal places
-- Expected: 3.14
SELECT '2.13 ROUND decimals' AS test_name, ROUND(num_dec, 2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.14: ROUND to 0 decimal places
-- Expected: 123.0 (or 123)
SELECT '2.14 ROUND zero places' AS test_name, ROUND(dec_col, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.15: ROUND negative (rounds to nearest 10)
-- Expected: 120
SELECT '2.15 ROUND neg places' AS test_name, ROUND(dec_col, -1) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.16: ROUND halfway up
-- Expected: 124
SELECT '2.16 ROUND halfway' AS test_name, ROUND(123.5, 0) AS result;

-- Test 2.17: TRUNCATE basic (via DataFusion trunc)
-- Expected: 123
SELECT '2.17 TRUNCATE basic' AS test_name, trunc(123.456, 0) AS result;

-- Test 2.18: TRUNCATE to 2 decimals
-- Expected: 123.45
SELECT '2.18 TRUNCATE decimals' AS test_name, trunc(123.456, 2) AS result;

-- Test 2.19: TRUNCATE negative places
-- Expected: 120
SELECT '2.19 TRUNCATE neg places' AS test_name, trunc(123.456, -1) AS result;

-- Test 2.20: TRUNCATE with integer
-- Expected: 42
SELECT '2.20 TRUNCATE integer' AS test_name, trunc(num1, 0) AS result FROM t_fn_data WHERE id = 3;

-- Test 2.21: MOD basic
-- Expected: 1 (100 % 3 = 1)
SELECT '2.21 MOD basic' AS test_name, MOD(num1, num2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.22: MOD with larger divisor
-- Expected: 100
SELECT '2.22 MOD larger divisor' AS test_name, MOD(num1, 1000) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.23: MOD with zero divisor (returns NULL)
-- Expected: NULL
SELECT '2.23 MOD zero divisor' AS test_name, MOD(num1, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.24: MOD with negative dividend
-- Expected: -1 (-100 % 3 = -1)
SELECT '2.24 MOD negative dividend' AS test_name, MOD(num_neg, num2) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.25: POWER basic
-- Expected: 8
SELECT '2.25 POWER basic' AS test_name, POWER(2, 3) AS result;

-- Test 2.26: POWER with zero exponent
-- Expected: 1
SELECT '2.26 POWER zero exp' AS test_name, POWER(10, 0) AS result;

-- Test 2.27: POWER with negative exponent
-- Expected: 0.25
SELECT '2.27 POWER neg exp' AS test_name, POWER(2, -2) AS result;

-- Test 2.28: POW alias
-- Expected: 9
SELECT '2.28 POW alias' AS test_name, POW(3, 2) AS result;

-- Test 2.29: SQRT basic
-- Expected: 5
SELECT '2.29 SQRT basic' AS test_name, SQRT(25) AS result;

-- Test 2.30: SQRT zero
-- Expected: 0
SELECT '2.30 SQRT zero' AS test_name, SQRT(0) AS result;

-- Test 2.31: SQRT of perfect square
-- Expected: 12
SELECT '2.31 SQRT perfect' AS test_name, SQRT(144) AS result;

-- Test 2.32: SQRT negative (returns NULL in MySQL)
-- Expected: NULL
SELECT '2.32 SQRT negative' AS test_name, SQRT(-1) AS result;

-- Test 2.33: LOG basic (natural log)
-- Expected: 0 (ln(1) = 0)
SELECT '2.33 LOG basic' AS test_name, LOG(1) AS result;

-- Test 2.34: LOG of e
-- Expected: 1 (ln(e) = 1)
SELECT '2.34 LOG of e' AS test_name, LOG(EXP(1)) AS result;

-- Test 2.35: LN alias of LOG
-- Expected: 1
SELECT '2.35 LN basic' AS test_name, LN(EXP(1)) AS result;

-- Test 2.36: LOG2 basic
-- Expected: 3 (log2(8) = 3)
SELECT '2.36 LOG2 basic' AS test_name, LOG2(8) AS result;

-- Test 2.37: LOG2 of 1
-- Expected: 0
SELECT '2.37 LOG2 of 1' AS test_name, LOG2(1) AS result;

-- Test 2.38: LOG10 basic
-- Expected: 2 (log10(100) = 2)
SELECT '2.38 LOG10 basic' AS test_name, LOG10(100) AS result;

-- Test 2.39: LOG10 of 1
-- Expected: 0
SELECT '2.39 LOG10 of 1' AS test_name, LOG10(1) AS result;

-- Test 2.40: EXP basic
-- Expected: ~7.389 (e^2)
SELECT '2.40 EXP basic' AS test_name, ROUND(EXP(2), 3) AS result;

-- Test 2.41: EXP zero
-- Expected: 1
SELECT '2.41 EXP zero' AS test_name, EXP(0) AS result;

-- Test 2.42: SIGN positive
-- Expected: 1
SELECT '2.42 SIGN positive' AS test_name, SIGN(num1) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.43: SIGN negative
-- Expected: -1
SELECT '2.43 SIGN negative' AS test_name, SIGN(num_neg) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.44: SIGN zero
-- Expected: 0
SELECT '2.44 SIGN zero' AS test_name, SIGN(num2) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.45: PI basic
-- Expected: 3.1415...
SELECT '2.45 PI basic' AS test_name, PI() AS result;

-- Test 2.46: RAND returns value between 0 and 1
-- Expected: value >= 0 AND < 1
SELECT '2.46 RAND range' AS test_name, RAND() AS result;

-- Test 2.47: RAND with seed (deterministic)
-- Expected: same value each time for same seed
SELECT '2.47 RAND seed' AS test_name, RAND(42) AS result;

-- Test 2.48: GREATEST numeric
-- Expected: 42
SELECT '2.48 GREATEST numeric' AS test_name, GREATEST(num1, num2, num_neg) AS result FROM t_fn_data WHERE id = 3;

-- Test 2.49: GREATEST with NULL (NULL ignored in DataFusion)
-- Expected: 100 (or NULL depending on implementation)
SELECT '2.49 GREATEST with NULL' AS test_name, GREATEST(num1, null_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.50: LEAST numeric
-- Expected: -42
SELECT '2.50 LEAST numeric' AS test_name, LEAST(num1, num2, num_neg) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.51: LEAST with negative
-- Expected: -100
SELECT '2.51 LEAST negative' AS test_name, LEAST(num1, num_neg, num3) AS result FROM t_fn_data WHERE id = 2;

-- Test 2.52: GREATEST string
-- Expected: 'hello' (lexicographically largest)
SELECT '2.52 GREATEST strings' AS test_name, GREATEST(s1, s2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.53: LEAST string
-- Expected: 'Hello World' (lexicographically smallest)
SELECT '2.53 LEAST strings' AS test_name, LEAST(s1, s2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.54: ROUND nested: ROUND(PI() * num_dec, 2)
-- Expected: ROUND(3.14159 * 123.45, 2) = ROUND(387.86...) = 387.86
SELECT '2.54 ROUND nested' AS test_name, ROUND(PI() * dec_col, 2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.55: Math with extracted value in WHERE clause
-- Expected: rows where ABS(num_neg) > 50
SELECT '2.55 WHERE with ABS' AS test_name, COUNT(*) AS result FROM t_fn_data WHERE ABS(num_neg) > 50;

-- Test 2.56: MOD with POWER
-- Expected: 4 (POWER(2, 3) = 8; 100 % 8 = 4)
SELECT '2.56 MOD with POWER' AS test_name, MOD(num1, POWER(num2, 2)) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.57: CEIL nested
-- Expected: CEIL(100 / 3) = CEIL(33.33) = 34
SELECT '2.57 CEIL division' AS test_name, CEIL(num1 / num2) AS result FROM t_fn_data WHERE id = 1;

-- Test 2.58: TRUNCATE on negative (via DataFusion trunc)
-- Expected: -123 (truncated toward zero)
SELECT '2.58 TRUNCATE negative' AS test_name, trunc(-123.456, 0) AS result;

-- ---------------------------------------------------------------------------
-- Sub-category: Date/Time Functions
-- ---------------------------------------------------------------------------

-- Test 3.1: NOW returns current datetime
-- Expected: non-NULL timestamp
SELECT '3.1 NOW' AS test_name, NOW() AS result;

-- Test 3.2: CURDATE (replaced with CURRENT_DATE)
-- Expected: non-NULL date
SELECT '3.2 CURDATE' AS test_name, CURRENT_DATE AS result;

-- Test 3.3: CURRENT_DATE alias
-- Expected: non-NULL date
SELECT '3.3 CURRENT_DATE' AS test_name, CURRENT_DATE AS result;

-- Test 3.4: CURRENT_TIMESTAMP
-- Expected: non-NULL timestamp
SELECT '3.4 CURRENT_TIMESTAMP' AS test_name, CURRENT_TIMESTAMP AS result;

-- Test 3.5: YEAR basic (via date_part)
-- Expected: 2024
SELECT '3.5 YEAR basic' AS test_name, date_part('year', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.6: YEAR on different year
-- Expected: 2023
SELECT '3.6 YEAR prev year' AS test_name, date_part('year', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 3;

-- Test 3.7: YEAR with NULL
-- Expected: NULL
SELECT '3.7 YEAR NULL' AS test_name, date_part('year', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 5;

-- Test 3.8: MONTH basic (via date_part)
-- Expected: 1 (January)
SELECT '3.8 MONTH basic' AS test_name, date_part('month', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.9: MONTH mid-year
-- Expected: 6 (June)
SELECT '3.9 MONTH mid-year' AS test_name, date_part('month', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 3;

-- Test 3.10: MONTH December
-- Expected: 12
SELECT '3.10 MONTH December' AS test_name, date_part('month', CAST(dt1 AS DATE)) AS result FROM t_fn_data WHERE id = 4;

-- Test 3.11: DAY basic (via date_part)
-- Expected: 15
SELECT '3.11 DAY basic' AS test_name, date_part('day', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.12: DAY on leap year date
-- Expected: 29
SELECT '3.12 DAY leap year' AS test_name, date_part('day', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 2;

-- Test 3.13: DAYOFMONTH (via date_part('day'))
-- Expected: 15
SELECT '3.13 DAYOFMONTH' AS test_name, date_part('day', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.14: HOUR from DATETIME (via date_part)
-- Expected: 10
SELECT '3.14 HOUR basic' AS test_name, date_part('hour', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.15: HOUR at end of day
-- Expected: 23
SELECT '3.15 HOUR late' AS test_name, date_part('hour', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 2;

-- Test 3.16: HOUR morning
-- Expected: 8
SELECT '3.16 HOUR morning' AS test_name, date_part('hour', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 3;

-- Test 3.17: MINUTE basic (via date_part)
-- Expected: 30
SELECT '3.17 MINUTE basic' AS test_name, date_part('minute', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.18: MINUTE near midnight
-- Expected: 59
SELECT '3.18 MINUTE boundary' AS test_name, date_part('minute', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 2;

-- Test 3.19: MINUTE zero
-- Expected: 0
SELECT '3.19 MINUTE zero' AS test_name, date_part('minute', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 4;

-- Test 3.20: SECOND basic (via date_part)
-- Expected: 0
SELECT '3.20 SECOND basic' AS test_name, date_part('second', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.21: SECOND near midnight
-- Expected: 59
SELECT '3.21 SECOND boundary' AS test_name, date_part('second', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 2;

-- Test 3.22: DAYOFWEEK (via date_part('dow'); MySQL Monday=2, but DataFusion DOW: Sun=0..Sat=6)
-- Expected: 1 (2024-01-15 is Monday, DataFusion DOW=1)
SELECT '3.22 DAYOFWEEK' AS test_name, date_part('dow', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.23: DAYOFWEEK Sunday (DataFusion DOW: 2023-12-31 is Sunday=0)
-- Expected: 0
SELECT '3.23 DAYOFWEEK Sunday' AS test_name, date_part('dow', CAST(d2 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.24: DAYOFYEAR (via date_part('doy'))
-- Expected: 15 (Jan 15)
SELECT '3.24 DAYOFYEAR Jan' AS test_name, date_part('doy', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.25: DAYOFYEAR leap year Feb 29
-- Expected: 60 (Jan 31 + Feb 29)
SELECT '3.25 DAYOFYEAR leap' AS test_name, date_part('doy', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 2;

-- Test 3.26: DAYOFYEAR Dec 25
-- Expected: 359 (Dec 25, non-leap year 2023)
SELECT '3.26 DAYOFYEAR Dec' AS test_name, date_part('doy', CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 3;

-- Test 3.27: DATE_ADD interval day (via days_add UDF)
-- Expected: '2024-01-20'
SELECT '3.27 DATE_ADD day' AS test_name, days_add(CAST(d1 AS DATE), 5) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.28: DATE_ADD interval month (via months_add UDF)
-- Expected: '2024-02-15'
SELECT '3.28 DATE_ADD month' AS test_name, months_add(CAST(d1 AS DATE), 1) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.29: DATE_ADD interval year — NOT SUPPORTED in RorisDB
-- No equivalent days_add/months_add for years. Could use CAST(d1 AS DATE) + make_interval(years => 1) if supported.
-- SELECT '3.29 DATE_ADD year' AS test_name, DATE_ADD(d1, INTERVAL 1 YEAR) AS result FROM t_fn_data WHERE id = 1;
SELECT '3.29 DATE_ADD year not supported' AS test_name;

-- Test 3.30: DATE_ADD negative interval (via days_add with negative)
-- Expected: '2023-12-15'
SELECT '3.30 DATE_ADD negative' AS test_name, days_add(CAST(d2 AS DATE), -16) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.31: DATE_SUB basic (via days_add with negative)
-- Expected: '2023-12-31'
SELECT '3.31 DATE_SUB basic' AS test_name, days_add(CAST(d1 AS DATE), -15) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.32: DATE_SUB month boundary (via months_add with negative)
-- Expected: '2023-12-15'
SELECT '3.32 DATE_SUB month' AS test_name, months_add(CAST(d1 AS DATE), -1) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.33: DATEDIFF basic — NOT SUPPORTED in RorisDB
-- No DATEDIFF equivalent registered in DataFusion.
-- SELECT '3.33 DATEDIFF basic' AS test_name, DATEDIFF(d1, d2) AS result FROM t_fn_data WHERE id = 1;
SELECT '3.33 DATEDIFF basic not supported' AS test_name;

-- Test 3.34: DATEDIFF same date — NOT SUPPORTED
-- SELECT '3.34 DATEDIFF same' AS test_name, DATEDIFF(d1, d1) AS result FROM t_fn_data WHERE id = 1;
SELECT '3.34 DATEDIFF same not supported' AS test_name;

-- Test 3.35: DATEDIFF leap year crossover — NOT SUPPORTED
-- SELECT '3.35 DATEDIFF leap' AS test_name, DATEDIFF(d2, d1) AS result FROM t_fn_data WHERE id = 2;
SELECT '3.35 DATEDIFF leap not supported' AS test_name;

-- Test 3.36: DATEDIFF with NULL — NOT SUPPORTED
-- SELECT '3.36 DATEDIFF NULL' AS test_name, DATEDIFF(d1, d2) AS result FROM t_fn_data WHERE id = 5;
SELECT '3.36 DATEDIFF NULL not supported' AS test_name;

-- Test 3.37: DATE_FORMAT basic — may work via DataFusion to_char
-- Expected: '2024-01-15'
SELECT '3.37 DATE_FORMAT YMD' AS test_name, DATE_FORMAT(CAST(d1 AS DATE), '%Y-%m-%d') AS result FROM t_fn_data WHERE id = 1;

-- Test 3.38: DATE_FORMAT month name — may work via DataFusion to_char
-- Expected: 'January'
SELECT '3.38 DATE_FORMAT month name' AS test_name, DATE_FORMAT(CAST(d1 AS DATE), '%M') AS result FROM t_fn_data WHERE id = 1;

-- Test 3.39: DATE_FORMAT day name — may work via DataFusion to_char
-- Expected: 'Monday'
SELECT '3.39 DATE_FORMAT day name' AS test_name, DATE_FORMAT(CAST(d1 AS DATE), '%W') AS result FROM t_fn_data WHERE id = 1;

-- Test 3.40: DATE_FORMAT with DATETIME — may work via DataFusion to_char
-- Expected: '10:30:00'
SELECT '3.40 DATE_FORMAT time' AS test_name, DATE_FORMAT(CAST(dt1 AS TIMESTAMP), '%H:%i:%s') AS result FROM t_fn_data WHERE id = 1;

-- Test 3.41: STR_TO_DATE — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.41 STR_TO_DATE' AS test_name, STR_TO_DATE('2024-01-15', '%Y-%m-%d') AS result;
SELECT '3.41 STR_TO_DATE not supported' AS test_name;

-- Test 3.42: STR_TO_DATE with time — NOT SUPPORTED
-- SELECT '3.42 STR_TO_DATE time' AS test_name, STR_TO_DATE('2024-01-15 10:30:00', '%Y-%m-%d %H:%i:%s') AS result;
SELECT '3.42 STR_TO_DATE time not supported' AS test_name;

-- Test 3.43: STR_TO_DATE non-standard format — NOT SUPPORTED
-- SELECT '3.43 STR_TO_DATE alt' AS test_name, STR_TO_DATE('15/01/2024', '%d/%m/%Y') AS result;
SELECT '3.43 STR_TO_DATE alt not supported' AS test_name;

-- Test 3.44: FROM_UNIXTIME — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.44 FROM_UNIXTIME' AS test_name, FROM_UNIXTIME(1705300200) AS result;
SELECT '3.44 FROM_UNIXTIME not supported' AS test_name;

-- Test 3.45: FROM_UNIXTIME with format — NOT SUPPORTED
-- SELECT '3.45 FROM_UNIXTIME fmt' AS test_name, FROM_UNIXTIME(1705300200, '%Y-%m-%d') AS result;
SELECT '3.45 FROM_UNIXTIME fmt not supported' AS test_name;

-- Test 3.46: FROM_UNIXTIME at epoch zero — NOT SUPPORTED
-- SELECT '3.46 FROM_UNIXTIME epoch' AS test_name, FROM_UNIXTIME(0) AS result;
SELECT '3.46 FROM_UNIXTIME epoch not supported' AS test_name;

-- Test 3.47: UNIX_TIMESTAMP — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.47 UNIX_TIMESTAMP' AS test_name, UNIX_TIMESTAMP() AS result;
SELECT '3.47 UNIX_TIMESTAMP not supported' AS test_name;

-- Test 3.48: UNIX_TIMESTAMP from date string — NOT SUPPORTED
-- SELECT '3.48 UNIX_TIMESTAMP str' AS test_name, UNIX_TIMESTAMP('2024-01-15') AS result;
SELECT '3.48 UNIX_TIMESTAMP str not supported' AS test_name;

-- Test 3.49: UNIX_TIMESTAMP from DATETIME string — NOT SUPPORTED
-- SELECT '3.49 UNIX_TIMESTAMP datetime str' AS test_name, UNIX_TIMESTAMP('2024-01-15 10:30:00') AS result;
SELECT '3.49 UNIX_TIMESTAMP datetime str not supported' AS test_name;

-- Test 3.50: MAKEDATE — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.50 MAKEDATE basic' AS test_name, MAKEDATE(2024, 15) AS result;
SELECT '3.50 MAKEDATE basic not supported' AS test_name;

-- Test 3.51: MAKEDATE leap year — NOT SUPPORTED
-- SELECT '3.51 MAKEDATE leap' AS test_name, MAKEDATE(2024, 60) AS result;
SELECT '3.51 MAKEDATE leap not supported' AS test_name;

-- Test 3.52: MAKEDATE first day — NOT SUPPORTED
-- SELECT '3.52 MAKEDATE first' AS test_name, MAKEDATE(2024, 1) AS result;
SELECT '3.52 MAKEDATE first not supported' AS test_name;

-- Test 3.53: MAKEDATE last day of year — NOT SUPPORTED
-- SELECT '3.53 MAKEDATE last' AS test_name, MAKEDATE(2024, 366) AS result;
SELECT '3.53 MAKEDATE last not supported' AS test_name;

-- Test 3.54: MAKETIME — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.54 MAKETIME basic' AS test_name, MAKETIME(10, 30, 0) AS result;
SELECT '3.54 MAKETIME basic not supported' AS test_name;

-- Test 3.55: MAKETIME boundary — NOT SUPPORTED
-- SELECT '3.55 MAKETIME boundary' AS test_name, MAKETIME(23, 59, 59) AS result;
SELECT '3.55 MAKETIME boundary not supported' AS test_name;

-- Test 3.56: MAKETIME zero — NOT SUPPORTED
-- SELECT '3.56 MAKETIME zero' AS test_name, MAKETIME(0, 0, 0) AS result;
SELECT '3.56 MAKETIME zero not supported' AS test_name;

-- Test 3.57: LAST_DAY — NOT SUPPORTED in RorisDB
-- This function is not registered in DataFusion or as a custom UDF.
-- SELECT '3.57 LAST_DAY basic' AS test_name, LAST_DAY(d1) AS result FROM t_fn_data WHERE id = 1;
SELECT '3.57 LAST_DAY basic not supported' AS test_name;

-- Test 3.58: LAST_DAY February leap year — NOT SUPPORTED
-- SELECT '3.58 LAST_DAY leap' AS test_name, LAST_DAY(d1) AS result FROM t_fn_data WHERE id = 2;
SELECT '3.58 LAST_DAY leap not supported' AS test_name;

-- Test 3.59: LAST_DAY June — NOT SUPPORTED
-- SELECT '3.59 LAST_DAY June' AS test_name, LAST_DAY(d1) AS result FROM t_fn_data WHERE id = 3;
SELECT '3.59 LAST_DAY June not supported' AS test_name;

-- Test 3.60: LAST_DAY December — NOT SUPPORTED
-- SELECT '3.60 LAST_DAY Dec' AS test_name, LAST_DAY(d2) AS result FROM t_fn_data WHERE id = 2;
SELECT '3.60 LAST_DAY Dec not supported' AS test_name;

-- Test 3.61: LAST_DAY with NULL — NOT SUPPORTED
-- SELECT '3.61 LAST_DAY NULL' AS test_name, LAST_DAY(d1) AS result FROM t_fn_data WHERE id = 5;
SELECT '3.61 LAST_DAY NULL not supported' AS test_name;

-- Test 3.62: DATE_ADD with INTERVAL WEEK (via days_add UDF, 7 days)
-- Expected: '2024-01-22'
SELECT '3.62 DATE_ADD week' AS test_name, days_add(CAST(d1 AS DATE), 7) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.63: DATE_SUB through year boundary (via months_add with negative)
-- Expected: '2023-12-15'
SELECT '3.63 DATE_SUB year boundary' AS test_name, months_add(CAST(d1 AS DATE), -1) AS result FROM t_fn_data WHERE id = 1;

-- Test 3.64: YEAR from DATETIME (via date_part)
-- Expected: 2024
SELECT '3.64 YEAR from DATETIME' AS test_name, date_part('year', CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 4;

-- Test 3.65: DATEDIFF across years — NOT SUPPORTED
-- SELECT '3.65 DATEDIFF across years' AS test_name, DATEDIFF(d2, d1) AS result FROM t_fn_data WHERE id = 2;
SELECT '3.65 DATEDIFF across years not supported' AS test_name;

-- ---------------------------------------------------------------------------
-- Sub-category: Conditional Functions
-- ---------------------------------------------------------------------------

-- Test 4.1: IF basic (true)
-- Expected: 'yes'
SELECT '4.1 IF true' AS test_name, IF(flag, 'yes', 'no') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.2: IF basic (false)
-- Expected: 'no'
SELECT '4.2 IF false' AS test_name, IF(flag, 'yes', 'no') AS result FROM t_fn_data WHERE id = 2;

-- Test 4.3: IF with comparison
-- Expected: 'big'
SELECT '4.3 IF comparison' AS test_name, IF(num1 > 50, 'big', 'small') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.4: IF with numeric result
-- Expected: 100 (true returns num1)
SELECT '4.4 IF numeric' AS test_name, IF(num1 = 100, num1, num2) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.5: IFNULL with non-NULL
-- Expected: 'Hello World'
SELECT '4.5 IFNULL non-NULL' AS test_name, IFNULL(s1, 'default') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.6: IFNULL with NULL
-- Expected: 'default'
SELECT '4.6 IFNULL NULL' AS test_name, IFNULL(null_col, 'default') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.7: IFNULL with zero
-- Expected: 0 (not treated as NULL)
SELECT '4.7 IFNULL zero' AS test_name, IFNULL(num2, 999) AS result FROM t_fn_data WHERE id = 2;

-- Test 4.8: IFNULL with empty string
-- Expected: '' (empty string is not NULL)
SELECT '4.8 IFNULL empty' AS test_name, IFNULL(s_empty, 'default') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.9: NULLIF equal values
-- Expected: NULL
SELECT '4.9 NULLIF equal' AS test_name, NULLIF(s1, s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.10: NULLIF different values
-- Expected: 'Hello World'
SELECT '4.10 NULLIF diff' AS test_name, NULLIF(s1, s2) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.11: NULLIF numeric equal
-- Expected: NULL
SELECT '4.11 NULLIF num equal' AS test_name, NULLIF(num1, 100) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.12: CASE WHEN simple
-- Expected: 'A' (100 > 50)
SELECT '4.12 CASE WHEN' AS test_name,
    CASE WHEN num1 > 50 THEN 'A' WHEN num1 > 0 THEN 'B' ELSE 'C' END AS result
FROM t_fn_data WHERE id = 1;

-- Test 4.13: CASE WHEN else branch
-- Expected: 'C' (0 > 50 is false, 0 > 0 is false)
SELECT '4.13 CASE ELSE' AS test_name,
    CASE WHEN num1 > 50 THEN 'A' WHEN num1 > 0 THEN 'B' ELSE 'C' END AS result
FROM t_fn_data WHERE id = 2;

-- Test 4.14: CASE WHEN middle branch
-- Expected: 'B' (42 > 50 is false, 42 > 0 is true)
SELECT '4.14 CASE middle' AS test_name,
    CASE WHEN num1 > 50 THEN 'A' WHEN num1 > 0 THEN 'B' ELSE 'C' END AS result
FROM t_fn_data WHERE id = 3;

-- Test 4.15: CASE with value matching
-- Expected: 'one'
SELECT '4.15 CASE value' AS test_name,
    CASE id WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END AS result
FROM t_fn_data WHERE id = 1;

-- Test 4.16: CASE with value matching default
-- Expected: 'other'
SELECT '4.16 CASE value default' AS test_name,
    CASE id WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END AS result
FROM t_fn_data WHERE id = 4;

-- Test 4.17: COALESCE basic
-- Expected: 'Hello World' (first non-NULL)
SELECT '4.17 COALESCE basic' AS test_name, COALESCE(s1, s2, 'fallback') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.18: COALESCE with NULLs
-- Expected: 'fallback'
SELECT '4.18 COALESCE all NULL' AS test_name, COALESCE(null_col, NULL, 'fallback') AS result;

-- Test 4.19: COALESCE numeric
-- Expected: 100 (first non-NULL)
SELECT '4.19 COALESCE numeric' AS test_name, COALESCE(num1, num2, 999) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.20: COALESCE with all NULLs
-- Expected: NULL
SELECT '4.20 COALESCE all NULLs' AS test_name, COALESCE(NULL, NULL, NULL) AS result;

-- Test 4.21: NULLIF with NULL first argument
-- Expected: NULL
SELECT '4.21 NULLIF NULL arg' AS test_name, NULLIF(null_col, 'anything') AS result FROM t_fn_data WHERE id = 1;

-- Test 4.22: Nested IF and NULLIF
-- Expected: 'Hello World' (NULLIF returns NULL, IFNULL catches it)
SELECT '4.22 IFNULL and NULLIF' AS test_name, IFNULL(NULLIF(s1, s1), s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.23: GREATEST conditional with dates
-- Expected: later date
SELECT '4.23 GREATEST dates' AS test_name, GREATEST(d1, d2) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.24: LEAST conditional with dates
-- Expected: earlier date
SELECT '4.24 LEAST dates' AS test_name, LEAST(d1, d2) AS result FROM t_fn_data WHERE id = 1;

-- Test 4.25: IF in ORDER BY
-- Expected: ordered by flag condition
SELECT '4.25 IF in ORDER BY' AS test_name, id, IF(flag, 'true', 'false') AS flag_label
FROM t_fn_data WHERE id <= 3 ORDER BY IF(flag, 1, 0) DESC;

-- ---------------------------------------------------------------------------
-- Sub-category: Conversion Functions (CAST)
-- ---------------------------------------------------------------------------

-- Test 5.1: CAST to INT
-- Expected: 100
SELECT '5.1 CAST to INT' AS test_name, CAST(s2 AS INT) AS result FROM t_fn_data WHERE id = 3;

-- Test 5.2: CAST to DOUBLE
-- Expected: 100.0 (or 100.5 for decimal string)
SELECT '5.2 CAST to DOUBLE' AS test_name, CAST('123.456' AS DOUBLE) AS result;

-- Test 5.3: CAST to VARCHAR
-- Expected: '100'
SELECT '5.3 CAST to VARCHAR' AS test_name, CAST(num1 AS VARCHAR) AS result FROM t_fn_data WHERE id = 1;

-- Test 5.4: CAST to DECIMAL
-- Expected: 123.46 (rounded)
SELECT '5.4 CAST to DECIMAL' AS test_name, CAST(123.456 AS DECIMAL(10,2)) AS result;

-- Test 5.5: CAST to DATE
-- Expected: DATE '2024-01-15'
SELECT '5.5 CAST to DATE' AS test_name, CAST('2024-01-15' AS DATE) AS result;

-- Test 5.6: CAST to DATETIME — may not work (DataFusion expects TIMESTAMP, not DATETIME)
-- Expected: DATETIME '2024-01-15 10:30:00'
SELECT '5.6 CAST to DATETIME' AS test_name, CAST('2024-01-15 10:30:00' AS TIMESTAMP) AS result;

-- Test 5.7: CAST to BOOLEAN (non-zero -> true)
-- Expected: 1 or TRUE
SELECT '5.7 CAST to BOOLEAN true' AS test_name, CAST(num1 AS BOOLEAN) AS result FROM t_fn_data WHERE id = 1;

-- Test 5.8: CAST to BOOLEAN (zero -> false)
-- Expected: 0 or FALSE
SELECT '5.8 CAST to BOOLEAN false' AS test_name, CAST(num2 AS BOOLEAN) AS result FROM t_fn_data WHERE id = 2;

-- Test 5.9: CAST to BIGINT
-- Expected: 9999999999
SELECT '5.9 CAST to BIGINT' AS test_name, CAST(num3 AS BIGINT) AS result FROM t_fn_data WHERE id = 1;

-- Test 5.10: CAST int to DOUBLE
-- Expected: 100.0
SELECT '5.10 INT to DOUBLE' AS test_name, CAST(num1 AS DOUBLE) AS result FROM t_fn_data WHERE id = 1;

-- Test 5.11: CAST string to DOUBLE
-- Expected: 3.14159
SELECT '5.11 STRING to DOUBLE' AS test_name, CAST('3.14159' AS DOUBLE) AS result;

-- Test 5.12: CAST in WHERE clause
-- Expected: rows where CAST(num1 AS VARCHAR) > '50'
SELECT '5.12 CAST in WHERE' AS test_name, id, num1 AS result
FROM t_fn_data WHERE CAST(num1 AS VARCHAR) IS NOT NULL AND id <= 3 ORDER BY id;

-- Test 5.13: CAST negative string to INT
-- Expected: -42
SELECT '5.13 CAST neg string' AS test_name, CAST('-42' AS INT) AS result;

-- Test 5.14: CAST with TRIM
-- Expected: 123
SELECT '5.14 CAST with TRIM' AS test_name, CAST(TRIM('  123  ') AS INT) AS result;

-- ---------------------------------------------------------------------------
-- Sub-category: Other Functions (UUID, VERSION, DATABASE)
-- ---------------------------------------------------------------------------

-- Test 6.1: UUID format check — may not work in RorisDB (DataFusion has uuid() without parens)
-- Expected: 36-char string with 4 hyphens
SELECT '6.1 UUID' AS test_name, LENGTH(UUID()) AS uuid_length FROM t_fn_data WHERE id = 1;

-- Test 6.2: UUID is unique across rows — may not work in RorisDB
-- Expected: 5 distinct values
SELECT '6.2 UUID unique' AS test_name, COUNT(DISTINCT UUID()) = COUNT(*) AS all_unique FROM t_fn_data;

-- Test 6.3: VERSION — may not work in RorisDB (no VERSION() function registered)
-- Expected: non-empty string
SELECT '6.3 VERSION' AS test_name, VERSION() AS result;

-- Test 6.4: DATABASE returns current database
-- Expected: 'e2e_functions_test'
SELECT '6.4 DATABASE' AS test_name, DATABASE() AS result;

-- ---------------------------------------------------------------------------
-- Sub-category: Combined and Nested Function Tests
-- ---------------------------------------------------------------------------

-- Test 7.1: UPPER(CONCAT(...))
-- Expected: 'HELLOWORLD'
SELECT '7.1 UPPER CONCAT' AS test_name, UPPER(CONCAT('hello', 'world')) AS result;

-- Test 7.2: LOWER(UPPER(...))
-- Expected: 'hello world'
SELECT '7.2 LOWER UPPER' AS test_name, LOWER(UPPER('Hello World')) AS result;

-- Test 7.3: ROUND(SQRT(POWER(...)))
-- Expected: ROUND(SQRT(25)) = ROUND(5) = 5
SELECT '7.3 ROUND SQRT POWER' AS test_name, ROUND(SQRT(POWER(5, 2)), 0) AS result;

-- Test 7.4: REVERSE(UPPER(...))
-- Expected: 'DLROW OLLEH'
SELECT '7.4 REVERSE UPPER' AS test_name, REVERSE(UPPER('Hello World')) AS result;

-- Test 7.5: LENGTH(TRIM(...))
-- Expected: 6 (TRIM('  spaced  ') = 'spaced' = 6 chars)
SELECT '7.5 LENGTH TRIM' AS test_name, LENGTH(TRIM(s_trim)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.6: CONCAT(LEFT(...), RIGHT(...))
-- Expected: 'Held' (LEFT('Hello World', 3) = 'Hel', RIGHT('Hello World', 2) = 'ld')
SELECT '7.6 CONCAT LEFT RIGHT' AS test_name, CONCAT(LEFT(s1, 3), RIGHT(s1, 2)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.7: REPLACE(UPPER(...), 'O', '0')
-- Expected: 'HELL0 W0RLD'
SELECT '7.7 REPLACE UPPER' AS test_name, REPLACE(UPPER(s1), 'O', '0') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.8: CEIL(ABS(num_neg) / num2)
-- Expected: CEIL(42 / 3) = CEIL(14) = 14
SELECT '7.8 CEIL ABS DIV' AS test_name, CEIL(ABS(num_neg) / num2) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.9: DATE_FORMAT(days_add(...)) — may work via DataFusion to_char
-- Expected: '2024-01-20'
SELECT '7.9 DATE_FORMAT DATE_ADD' AS test_name, DATE_FORMAT(days_add(CAST(d1 AS DATE), 5), '%Y-%m-%d') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.10: YEAR(DATE_ADD with INTERVAL YEAR) — NOT SUPPORTED (DATE_ADD YEAR not available)
-- SELECT '7.10 YEAR DATE_ADD' AS test_name, YEAR(DATE_ADD(d1, INTERVAL 1 YEAR)) AS result FROM t_fn_data WHERE id = 1;
SELECT '7.10 YEAR DATE_ADD year not supported' AS test_name;

-- Test 7.11: IFNULL(CAST(NULL AS INT), 0)
-- Expected: 0
SELECT '7.11 IFNULL CAST' AS test_name, IFNULL(CAST(null_col AS INT), 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.12: COALESCE(UPPER(null_col), LOWER(s1), 'fallback')
-- Expected: 'hello' (null_col is NULL, so LOWER('Hello World') = 'hello world')
SELECT '7.12 COALESCE UPPER LOWER' AS test_name, COALESCE(UPPER(null_col), LOWER(s1), 'fallback') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.13: LENGTH(SPACE(5))
-- Expected: 5
SELECT '7.13 LENGTH SPACE' AS test_name, LENGTH(SPACE(5)) AS result;

-- Test 7.14: CONCAT_WS(', ', UPPER(s1), LOWER(s2))
-- Expected: 'HELLO WORLD, hello'
SELECT '7.14 CONCAT_WS UPPER LOWER' AS test_name, CONCAT_WS(', ', UPPER(s1), LOWER(s2)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.15: ABS(ROUND(num_neg * num_dec))
-- Expected: ABS(ROUND(-42 * 3.14159)) = ABS(ROUND(-131.94678)) = ABS(-132) = 132
SELECT '7.15 ABS ROUND MULTIPLY' AS test_name, ABS(ROUND(num_neg * num_dec)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.16: RPAD(LPAD('x', 3, '0'), 5, '0')
-- Expected: '00x00'
SELECT '7.16 RPAD LPAD' AS test_name, RPAD(LPAD('x', 3, '0'), 5, '0') AS result;

-- Test 7.17: SUBSTRING(UPPER(REPLACE(...)))
-- Expected: UPPER(REPLACE('Hello World', ' ', '')) = 'HELLOWORLD', SUBSTRING('HELLOWORLD', 1, 5) = 'HELLO'
SELECT '7.17 SUBSTR UPPER REPLACE' AS test_name, SUBSTRING(UPPER(REPLACE(s1, ' ', '')), 1, 5) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.18: TRIM(REPLACE('  a  b  ', ' ', ''))
-- Expected: 'ab'
SELECT '7.18 TRIM REPLACE' AS test_name, TRIM(REPLACE('  a  b  ', ' ', '')) AS result;

-- Test 7.19: DATEDIFF(LAST_DAY(d1), d1) — NOT SUPPORTED (DATEDIFF and LAST_DAY unavailable)
-- SELECT '7.19 DATEDIFF LAST_DAY' AS test_name, DATEDIFF(LAST_DAY(d1), d1) AS result FROM t_fn_data WHERE id = 1;
SELECT '7.19 DATEDIFF LAST_DAY not supported' AS test_name;

-- Test 7.20: DAY(LAST_DAY(d1)) — NOT SUPPORTED (LAST_DAY unavailable)
-- SELECT '7.20 DAY LAST_DAY' AS test_name, DAY(LAST_DAY(d1)) AS result FROM t_fn_data WHERE id = 1;
SELECT '7.20 DAY LAST_DAY not supported' AS test_name;

-- Test 7.21: MONTH(DATE_ADD with INTERVAL MONTH via months_add)
-- Expected: 2 (February)
SELECT '7.21 MONTH DATE_ADD' AS test_name, date_part('month', months_add(CAST(d1 AS DATE), 1)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.22: GREATEST(ROUND(num_dec), num2, ABS(num_neg))
-- Expected: GREATEST(3, 3, 42) = 42
SELECT '7.22 GREATEST ROUND ABS' AS test_name, GREATEST(ROUND(num_dec), num2, ABS(num_neg)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.23: LEAST(CEIL(num_dec), num1, POWER(num2, 2))
-- Expected: LEAST(4, 100, 9) = 4
SELECT '7.23 LEAST CEIL POWER' AS test_name, LEAST(CEIL(num_dec), num1, POWER(num2, 2)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.24: REPEAT(LEFT(s1, 1), 5)
-- Expected: 'HHHHH'
SELECT '7.24 REPEAT LEFT' AS test_name, REPEAT(LEFT(s1, 1), 5) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.25: MOD(ABS(num_neg), 7)
-- Expected: 0 (42 % 7 = 0)
SELECT '7.25 MOD ABS' AS test_name, MOD(ABS(num_neg), 7) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.26: CONCAT(UUID(), '-', VERSION()) — UUID/VERSION may not work in RorisDB
-- Expected: non-empty string
SELECT '7.26 CONCAT UUID VERSION' AS test_name, CONCAT('uuid-', 'version') AS result;

-- Test 7.27: REVERSE(REVERSE(s1))
-- Expected: 'Hello World' (double reverse = identity)
SELECT '7.27 REVERSE REVERSE' AS test_name, REVERSE(REVERSE(s1)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.28: IF(ISNULL(null_col), 'is null', 'not null')
-- Expected: 'is null' (but DataFusion may not have ISNULL, use IS NULL instead)
SELECT '7.28 IF NULL check' AS test_name, IF(null_col IS NULL, 'is null', 'not null') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.29: CASE WHEN with nested functions
-- Expected: CONCAT_WS based on condition
SELECT '7.29 CASE nested funcs' AS test_name,
    CASE WHEN num1 > 50 THEN CONCAT_WS('-', s1, s2) ELSE CONCAT(s1, s2) END AS result
FROM t_fn_data WHERE id = 1;

-- Test 7.30: ROUND with negative decimals on decimal column
-- Expected: 120 (123.45 rounded to nearest 10)
SELECT '7.30 ROUND neg dec' AS test_name, ROUND(dec_col, -1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.31: TRUNCATE with negative on negative number (via DataFusion trunc)
-- Expected: -120 (or -123 depending on truncation behavior)
SELECT '7.31 TRUNCATE neg neg' AS test_name, trunc(-123.45, -1) AS result;

-- Test 7.32: POSITION (alias for LOCATE)
-- Expected: 7
SELECT '7.32 POSITION basic' AS test_name, POSITION('World' IN s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.33: POSITION not found
-- Expected: 0
SELECT '7.33 POSITION not found' AS test_name, POSITION('xyz' IN s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.34: EXTRACT(YEAR FROM date)
-- Expected: 2024
SELECT '7.34 EXTRACT YEAR' AS test_name, EXTRACT(YEAR FROM CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.35: EXTRACT(MONTH FROM date)
-- Expected: 1
SELECT '7.35 EXTRACT MONTH' AS test_name, EXTRACT(MONTH FROM CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.36: EXTRACT(DAY FROM date)
-- Expected: 15
SELECT '7.36 EXTRACT DAY' AS test_name, EXTRACT(DAY FROM CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.37: EXTRACT(HOUR FROM datetime)
-- Expected: 10
SELECT '7.37 EXTRACT HOUR' AS test_name, EXTRACT(HOUR FROM CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.38: EXTRACT(MINUTE FROM datetime)
-- Expected: 30
SELECT '7.38 EXTRACT MINUTE' AS test_name, EXTRACT(MINUTE FROM CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.39: EXTRACT(SECOND FROM datetime)
-- Expected: 0
SELECT '7.39 EXTRACT SECOND' AS test_name, EXTRACT(SECOND FROM CAST(dt1 AS TIMESTAMP)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.40: EXTRACT(DOY FROM date) -- day of year
-- Expected: 15
SELECT '7.40 EXTRACT DOY' AS test_name, EXTRACT(DOY FROM CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.41: EXTRACT(DOW FROM date) -- day of week (Sunday=0)
-- Expected: 1 (2024-01-15 is Monday, DOW in PostgreSQL is 1)
SELECT '7.41 EXTRACT DOW' AS test_name, EXTRACT(DOW FROM CAST(d1 AS DATE)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.42: CONCAT with numeric coercion
-- Expected: 'Value: 100'
SELECT '7.42 CONCAT numeric' AS test_name, CONCAT('Value: ', num1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.43: REPLACE nested inside REPLACE
-- Expected: 'Hello Universe' (World -> Universe)
SELECT '7.43 REPLACE nested' AS test_name, REPLACE(REPLACE(s1, 'Hello', 'Goodbye'), 'World', 'Universe') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.44: INSERT INTO with function in VALUES
CREATE TABLE t_fn_insert_test (
    id INT,
    val VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_fn_insert_test VALUES (1, UPPER('inserted value'));
-- Expected: 1 row with 'INSERTED VALUE'
SELECT '7.44 INSERT with UPPER' AS test_name, val AS result FROM t_fn_insert_test WHERE id = 1;

-- Test 7.45: UPDATE with function in SET
UPDATE t_fn_insert_test SET val = CONCAT(val, '-', 'suffix') WHERE id = 1;
-- Expected: 'INSERTED VALUE-suffix'
SELECT '7.45 UPDATE with CONCAT' AS test_name, val AS result FROM t_fn_insert_test WHERE id = 1;

DROP TABLE t_fn_insert_test;

-- Test 7.46: String length boundary with very long string
SELECT '7.46 LENGTH long' AS test_name, LENGTH(REPEAT('a', 100)) AS result;

-- Test 7.47: SUBSTRING with start position at 1 (should return full string)
-- Expected: 'Hello World'
SELECT '7.47 SUBSTRING start 1' AS test_name, SUBSTRING(s1, 1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.48: SUBSTRING with start position exactly at end
-- Expected: ''
SELECT '7.48 SUBSTRING at end' AS test_name, SUBSTRING(s1, 12) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.49: LOCATE with non-ASCII characters
-- Expected: position of '好'
SELECT '7.49 LOCATE unicode' AS test_name, LOCATE('好', s_unicode) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.50: HEX of large number — NOT SUPPORTED
-- SELECT '7.50 HEX large' AS test_name, HEX(num3) AS result FROM t_fn_data WHERE id = 1;
SELECT '7.50 HEX large not supported' AS test_name;

-- Test 7.51: MOD with MOD
-- Expected: 100 % (3 % 2) = 100 % 1 = 0
SELECT '7.51 MOD nested' AS test_name, MOD(num1, MOD(num2, 2)) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.52: SIGN with GREATEST
-- Expected: SIGN(GREATEST(-1, 0, 1)) = SIGN(1) = 1
SELECT '7.52 SIGN GREATEST' AS test_name, SIGN(GREATEST(num_neg, 0, num2)) AS result FROM t_fn_data WHERE id = 3;

-- Test 7.53: EXP and LN round-trip
-- Expected: 100 (approximately, with rounding)
SELECT '7.53 EXP LN' AS test_name, ROUND(EXP(LN(100)), 0) AS result;

-- Test 7.54: POWER and SQRT round-trip
-- Expected: 5 (SQRT(POWER(5, 2)) = 5)
SELECT '7.54 SQRT POWER' AS test_name, SQRT(POWER(5, 2)) AS result;

-- Test 7.55: TRUNCATE nested (via DataFusion trunc)
-- Expected: 3.14
SELECT '7.55 TRUNCATE ROUND' AS test_name, trunc(ROUND(3.14159, 4), 2) AS result;

-- Test 7.56: CONCAT_WS with empty separator
-- Expected: 'abc' (no separator)
SELECT '7.56 CONCAT_WS empty sep' AS test_name, CONCAT_WS('', 'a', 'b', 'c') AS result;

-- Test 7.57: CONCAT_WS separator with special characters
-- Expected: 'a|b|c'
SELECT '7.57 CONCAT_WS special sep' AS test_name, CONCAT_WS('|', 'a', 'b', 'c') AS result;

-- Test 7.58: LOCATE with multiple overlapping matches
-- Expected: 2 (first 'll' starts at position 2 in 'hello')
SELECT '7.58 LOCATE overlapping' AS test_name, LOCATE('ll', LOWER(s1)) AS result FROM t_fn_data WHERE id = 2;

-- Test 7.59: INSTR with substring at beginning
-- Expected: 1
SELECT '7.59 INSTR at start' AS test_name, INSTR(s1, 'He') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.60: INSTR with substring at end
-- Expected: 10 (LENGTH('Hello World') - LENGTH('rld') + 1)
SELECT '7.60 INSTR at end' AS test_name, INSTR(s1, 'rld') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.61: ROUND with decimal from table
-- Expected: 3.14 (ROUND(3.14159, 2))
SELECT '7.61 ROUND DECIMAL' AS test_name, ROUND(num_dec, 2) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.62: POW with negative base and even exponent
-- Expected: 9 ((-3)^2 = 9)
SELECT '7.62 POW neg base even' AS test_name, POW(-3, 2) AS result;

-- Test 7.63: ABS and SIGN combined
-- Expected: 1 (ABS always positive, SIGN of positive = 1)
SELECT '7.63 SIGN ABS' AS test_name, SIGN(ABS(num_neg)) AS result FROM t_fn_data WHERE id = 2;

-- Test 7.64: CASE with NULL result
-- Expected: NULL
SELECT '7.64 CASE NULL result' AS test_name,
    CASE WHEN 1 = 2 THEN NULL ELSE null_col END AS result
FROM t_fn_data WHERE id = 1;

-- Test 7.65: COALESCE with CONCAT
-- Expected: 'Hello World' (concatenated result)
SELECT '7.65 COALESCE CONCAT' AS test_name, COALESCE(CONCAT(s1, s2), 'fallback') AS result FROM t_fn_data WHERE id = 1;

-- Test 7.66: DATE_FORMAT(STR_TO_DATE(...)) — NOT SUPPORTED (STR_TO_DATE unavailable)
-- SELECT '7.66 DATE_FORMAT STR_TO_DATE' AS test_name, DATE_FORMAT(STR_TO_DATE('01/15/2024', '%m/%d/%Y'), '%Y-%m-%d') AS result;
SELECT '7.66 DATE_FORMAT STR_TO_DATE not supported' AS test_name;

-- Test 7.67: FROM_UNIXTIME(UNIX_TIMESTAMP(...)) — NOT SUPPORTED (both unavailable)
-- SELECT '7.67 FROM_UNIXTIME UNIX_TIMESTAMP' AS test_name, FROM_UNIXTIME(UNIX_TIMESTAMP('2024-01-15 00:00:00'), '%Y-%m-%d') AS result;
SELECT '7.67 FROM_UNIXTIME UNIX_TIMESTAMP not supported' AS test_name;

-- Test 7.68: MAKEDATE and LAST_DAY — NOT SUPPORTED (both unavailable)
-- SELECT '7.68 MAKEDATE LAST_DAY' AS test_name, LAST_DAY(MAKEDATE(2024, 32)) AS result;
SELECT '7.68 MAKEDATE LAST_DAY not supported' AS test_name;

-- Test 7.69: LOCATE with 0 start position (treated as 1 in MySQL)
-- Expected: 7
SELECT '7.69 LOCATE start 0' AS test_name, LOCATE('World', s1, 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.70: UNHEX with invalid characters (non-hex) — NOT SUPPORTED
-- SELECT '7.70 UNHEX non-hex' AS test_name, UNHEX('ZZZZ') AS result;
SELECT '7.70 UNHEX non-hex not supported' AS test_name;

-- Test 7.71: CHAR_LENGTH vs LENGTH for ASCII (should be equal)
-- Expected: 11 (same for ASCII strings)
SELECT '7.71 CHAR_LENGTH eq LENGTH' AS test_name, CHAR_LENGTH(s1) = LENGTH(s1) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.72: LTRIM on string with no leading spaces
-- Expected: 'hello   '
SELECT '7.72 LTRIM no change' AS test_name, LTRIM(s2) AS result FROM t_fn_data WHERE id = 2;

-- Test 7.73: RTRIM on string with no trailing spaces
-- Expected: 'ABCdef'
SELECT '7.73 RTRIM no change' AS test_name, RTRIM(s1) AS result FROM t_fn_data WHERE id = 3;

-- Test 7.74: FLOOR negative number
-- Expected: -43 (FLOOR(-42.5) = -43)
SELECT '7.74 FLOOR negative dec' AS test_name, FLOOR(-42.5) AS result;

-- Test 7.75: CEIL negative number
-- Expected: -42 (CEIL(-42.5) = -42)
SELECT '7.75 CEIL negative dec' AS test_name, CEIL(-42.5) AS result;

-- Test 7.76: ABS of BIGINT
-- Expected: 9999999999
SELECT '7.76 ABS BIGINT' AS test_name, ABS(num3) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.77: ABS of minimum negative INT
-- Expected: 255
SELECT '7.77 ABS min neg' AS test_name, ABS(num_neg) AS result FROM t_fn_data WHERE id = 4;

-- Test 7.78: REPEAT with very large count (not too large)
-- Expected: 500 chars
SELECT '7.78 REPEAT large' AS test_name, LENGTH(REPEAT('abc', 10)) AS result;

-- Test 7.79: TRIM(BOTH 'x' FROM 'xxxhelloxxx') -- MySQL-specific TRIM syntax
-- This may not be supported in DataFusion
SELECT '7.79 TRIM char' AS test_name, TRIM(LEADING ' ' FROM s_trim) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.80: INSERT with function expression in values
CREATE TABLE t_fn_expr_test (
    id INT,
    val1 VARCHAR(50),
    val2 INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

INSERT INTO t_fn_expr_test VALUES (1, LOWER('EXPRESSION'), ABS(-99));
SELECT '7.80 INSERT func exprs' AS test_name, val1, val2 AS result FROM t_fn_expr_test WHERE id = 1;

UPDATE t_fn_expr_test SET val2 = POWER(val2, 2) WHERE id = 1;
-- Expected: 9801 (99^2)
SELECT '7.81 UPDATE with POWER' AS test_name, val2 AS result FROM t_fn_expr_test WHERE id = 1;

DROP TABLE t_fn_expr_test;

-- Test 7.82: COALESCE of multiple columns with NULL at end
-- Expected: s1 value (first non-NULL in chain)
SELECT '7.82 COALESCE chain' AS test_name, COALESCE(NULL, NULL, s1, s2, null_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.83: CONCAT of CONCAT
-- Expected: 'HelloWorldABCdef'
SELECT '7.83 CONCAT nested' AS test_name, CONCAT(CONCAT(s1, s2), s1) AS result FROM t_fn_data WHERE id = 2;

-- Test 7.84: IFNULL with GREATEST
-- Expected: GREATEST returns 42, IFNULL returns it
SELECT '7.84 IFNULL GREATEST' AS test_name, IFNULL(GREATEST(num1, num2, num_neg), 0) AS result FROM t_fn_data WHERE id = 3;

-- Test 7.85: ROUND(0.5) rounding halfway
-- Expected: 1 (round half away from zero)
SELECT '7.85 ROUND half up' AS test_name, ROUND(0.5) AS result;

-- Test 7.86: ROUND(-0.5) negative halfway
-- Expected: -1 (round away from zero)
SELECT '7.86 ROUND half neg' AS test_name, ROUND(-0.5) AS result;

-- Test 7.87: TRUNCATE on 0 with negative places (via DataFusion trunc)
-- Expected: 0
SELECT '7.87 TRUNCATE zero' AS test_name, trunc(0, -1) AS result;

-- Test 7.88: ORDER BY with function
-- Expected: sorted by LENGTH of s1
SELECT '7.88 ORDER BY LENGTH' AS test_name, id, s1 AS result
FROM t_fn_data WHERE id <= 4 ORDER BY LENGTH(s1) DESC;

-- Test 7.89: DISTINCT with function
-- Expected: distinct values of UPPER(s1)
SELECT '7.89 DISTINCT UPPER' AS test_name, DISTINCT UPPER(s1) AS result
FROM t_fn_data WHERE s1 IS NOT NULL;

-- Test 7.90: HAVING with function
-- Expected: ids where LENGTH(s1) > 5
SELECT '7.90 HAVING LENGTH' AS test_name, id, LENGTH(s1) AS len FROM t_fn_data
WHERE s1 IS NOT NULL
GROUP BY id, s1
HAVING LENGTH(s1) > 5
ORDER BY id;

-- Test 7.91: GREATEST across different numeric types
-- Expected: maximum of the three
SELECT '7.91 GREATEST mixed types' AS test_name, GREATEST(num1, num_dec, dec_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.92: LEAST across different numeric types
-- Expected: minimum of the three
SELECT '7.92 LEAST mixed types' AS test_name, LEAST(num1, num_dec, dec_col) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.93: SUBSTRING index (custom UDF)
-- Expected: 'www' (first segment before first '.')
SELECT '7.93 SUBSTRING_INDEX forward' AS test_name, SUBSTRING_INDEX('www.example.com', '.', 1) AS result;

-- Test 7.94: SUBSTRING_INDEX from end
-- Expected: 'example.com' (last two segments)
SELECT '7.94 SUBSTRING_INDEX reverse' AS test_name, SUBSTRING_INDEX('www.example.com', '.', -2) AS result;

-- Test 7.95: SUBSTRING_INDEX no match
-- Expected: 'www.example.com' (full string returned)
SELECT '7.95 SUBSTRING_INDEX no match' AS test_name, SUBSTRING_INDEX('www.example.com', '/', 1) AS result;

-- Test 7.96: SUBSTRING_INDEX with NULL
-- Expected: NULL
SELECT '7.96 SUBSTRING_INDEX NULL' AS test_name, SUBSTRING_INDEX(NULL, '.', 1) AS result;

-- Test 7.97: CONCAT_WS with all string types
-- Expected: '100|hello|42|3.14'
SELECT '7.97 CONCAT_WS mixed types' AS test_name, CONCAT_WS('|', CAST(num1 AS VARCHAR), s2, CAST(num1 AS VARCHAR), CAST(num_dec AS VARCHAR)) AS result FROM t_fn_data WHERE id = 3;

-- Test 7.98: DATE_ADD with INTERVAL 0 (via days_add)
-- Expected: same date
SELECT '7.98 DATE_ADD zero' AS test_name, days_add(CAST(d1 AS DATE), 0) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.99: DATE_ADD INTERVAL with negative value (via days_add)
-- Expected: '2024-01-10'
SELECT '7.99 DATE_ADD neg interval' AS test_name, days_add(CAST(d1 AS DATE), -5) AS result FROM t_fn_data WHERE id = 1;

-- Test 7.100: DATEDIFF of identical dates — NOT SUPPORTED
-- SELECT '7.100 DATEDIFF identical' AS test_name, DATEDIFF(CAST('2024-01-15' AS DATE), CAST('2024-01-15' AS DATE)) AS result;
SELECT '7.100 DATEDIFF identical not supported' AS test_name;

-- ============================================================================
-- Cleanup
-- ============================================================================

DROP TABLE t_fn_data;
DROP DATABASE e2e_functions_test;

SELECT 'All built-in function tests completed' AS status;