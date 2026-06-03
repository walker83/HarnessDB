-- HarnessDB Account/Security and UNIQUE Constraint Integration Test Script
-- Run with: mysql -h 127.0.0.1 -P 9030 -uroot < tests/integration/security_unique_test.sql

-- ============================================================================
-- Section 1: User Management - CREATE USER
-- ============================================================================

-- Test 1.1: Create a simple user
CREATE USER 'test_user1'@'localhost';

-- Test 1.2: Create user with password
CREATE USER 'test_user2'@'localhost' IDENTIFIED BY 'password123';

-- Test 1.3: Create user with password and host wildcard
CREATE USER 'test_user3'@'%' IDENTIFIED BY 'pass@word!456';

-- Test 1.4: Create user with comment
CREATE USER 'test_user4'@'localhost' IDENTIFIED BY 'test123' COMMENT 'Test user for integration tests';

-- Test 1.5: Create if not exists (should succeed even if user already exists)
CREATE USER IF NOT EXISTS 'test_user1'@'localhost';

-- ============================================================================
-- Section 2: User Management - ALTER USER
-- ============================================================================

-- Test 2.1: Alter user password
ALTER USER 'test_user1'@'localhost' IDENTIFIED BY 'newpassword123';

-- Test 2.2: Alter user with multiple properties
ALTER USER 'test_user2'@'localhost' IDENTIFIED BY 'updatedpwd' COMMENT 'Updated user';

-- ============================================================================
-- Section 3: Password Management - SET PASSWORD
-- ============================================================================

-- Test 3.1: Set password for specific user
SET PASSWORD FOR 'test_user3'@'%' = 'newsecretpass';

-- Test 3.2: Set password for current user (root)
SET PASSWORD = 'root_password';

-- Reset root password to empty for simplicity
SET PASSWORD = '';

-- ============================================================================
-- Section 4: Privilege Management - GRANT
-- ============================================================================

-- Create test database and tables for privilege testing
CREATE DATABASE IF NOT EXISTS security_test_db;
USE security_test_db;
CREATE TABLE IF NOT EXISTS test_table1 (
    id INT,
    name VARCHAR(100)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

CREATE TABLE IF NOT EXISTS test_table2 (
    id INT,
    value INT
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Test 4.1: Grant all privileges on database
GRANT ALL PRIVILEGES ON security_test_db.* TO 'test_user1'@'localhost';

-- Test 4.2: Grant specific privileges on table
GRANT SELECT, INSERT ON security_test_db.test_table1 TO 'test_user2'@'localhost';

-- Test 4.3: Grant multiple privileges on database
GRANT SELECT, INSERT, UPDATE, DELETE ON security_test_db.* TO 'test_user3'@'%';

-- Test 4.4: Grant with GRANT OPTION
GRANT SELECT ON security_test_db.* TO 'test_user4'@'localhost' WITH GRANT OPTION;

-- ============================================================================
-- Section 5: Privilege Management - SHOW GRANTS
-- ============================================================================

-- Test 5.1: Show grants for specific user
SHOW GRANTS FOR 'test_user1'@'localhost';

-- Test 5.2: Show grants for another user
SHOW GRANTS FOR 'test_user2'@'localhost';

-- Test 5.3: Show grants for wildcard host user
SHOW GRANTS FOR 'test_user3'@'%';

-- Test 5.4: Show grants for current user
SHOW GRANTS;

-- ============================================================================
-- Section 6: Privilege Management - REVOKE
-- ============================================================================

-- Test 6.1: Revoke specific privileges
REVOKE INSERT, UPDATE ON security_test_db.* FROM 'test_user1'@'localhost';

-- Test 6.2: Revoke all privileges on table
REVOKE ALL PRIVILEGES ON security_test_db.test_table1 FROM 'test_user2'@'localhost';

-- Test 6.3: Revoke grant option
REVOKE GRANT OPTION ON security_test_db.* FROM 'test_user4'@'localhost';

-- Test 6.4: Verify grants after revoke
SHOW GRANTS FOR 'test_user1'@'localhost';
SHOW GRANTS FOR 'test_user2'@'localhost';

-- ============================================================================
-- Section 7: UNIQUE Constraint - Table Creation
-- ============================================================================

-- Create database for UNIQUE constraint testing
CREATE DATABASE IF NOT EXISTS unique_test_db;
USE unique_test_db;

-- Test 7.1: Create table with single UNIQUE constraint
CREATE TABLE IF NOT EXISTS unique_single (
    id INT,
    username VARCHAR(50),
    email VARCHAR(100),
    UNIQUE KEY uk_username (username)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Test 7.2: Create table with multiple UNIQUE constraints
CREATE TABLE IF NOT EXISTS unique_multiple (
    id INT,
    username VARCHAR(50),
    email VARCHAR(100),
    phone VARCHAR(20),
    UNIQUE KEY uk_username (username),
    UNIQUE KEY uk_email (email),
    UNIQUE KEY uk_phone (phone)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Test 7.3: Create table with composite UNIQUE constraint
CREATE TABLE IF NOT EXISTS unique_composite (
    id INT,
    user_id INT,
    order_id INT,
    product_code VARCHAR(50),
    UNIQUE KEY uk_user_order (user_id, order_id)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- Test 7.4: Create table with UNIQUE constraint on NOT NULL column
CREATE TABLE IF NOT EXISTS unique_notnull (
    id INT,
    username VARCHAR(50) NOT NULL,
    UNIQUE KEY uk_username (username)
) DISTRIBUTED BY HASH(id) BUCKETS 3;

-- ============================================================================
-- Section 8: UNIQUE Constraint - Data Insertion
-- ============================================================================

-- Test 8.1: Insert valid data into single UNIQUE table
INSERT INTO unique_single VALUES (1, 'user1', 'user1@example.com');
INSERT INTO unique_single VALUES (2, 'user2', 'user2@example.com');
INSERT INTO unique_single VALUES (3, 'user3', 'user3@example.com');

-- Test 8.2: Verify data inserted
SELECT * FROM unique_single ORDER BY id;

-- Test 8.3: Insert valid data into multiple UNIQUE table
INSERT INTO unique_multiple VALUES (1, 'user_a', 'user_a@example.com', '1234567890');
INSERT INTO unique_multiple VALUES (2, 'user_b', 'user_b@example.com', '1234567891');
INSERT INTO unique_multiple VALUES (3, 'user_c', 'user_c@example.com', '1234567892');

-- Test 8.4: Verify data inserted
SELECT * FROM unique_multiple ORDER BY id;

-- Test 8.5: Insert valid data into composite UNIQUE table
INSERT INTO unique_composite VALUES (1, 100, 1001, 'PROD001');
INSERT INTO unique_composite VALUES (2, 100, 1002, 'PROD002');
INSERT INTO unique_composite VALUES (3, 101, 1001, 'PROD003');

-- Test 8.6: Verify data inserted
SELECT * FROM unique_composite ORDER BY id;

-- ============================================================================
-- Section 9: UNIQUE Constraint - Violation Tests (Single Column)
-- ============================================================================

-- Test 9.1: Attempt to insert duplicate username (should fail)
INSERT INTO unique_single VALUES (4, 'user1', 'user4@example.com');

-- Test 9.2: Verify no duplicate was inserted
SELECT COUNT(*) AS count FROM unique_single WHERE username = 'user1';

-- Test 9.3: Attempt to insert duplicate username with different id (should fail)
INSERT INTO unique_single VALUES (5, 'user2', 'user5@example.com');

-- Test 9.4: Verify table state unchanged
SELECT * FROM unique_single ORDER BY id;

-- ============================================================================
-- Section 10: UNIQUE Constraint - Violation Tests (Multiple Columns)
-- ============================================================================

-- Test 10.1: Attempt to insert duplicate username (should fail)
INSERT INTO unique_multiple VALUES (4, 'user_a', 'user_d@example.com', '1234567893');

-- Test 10.2: Attempt to insert duplicate email (should fail)
INSERT INTO unique_multiple VALUES (5, 'user_d', 'user_a@example.com', '1234567894');

-- Test 10.3: Attempt to insert duplicate phone (should fail)
INSERT INTO unique_multiple VALUES (6, 'user_e', 'user_e@example.com', '1234567890');

-- Test 10.4: Verify no duplicates were inserted
SELECT COUNT(*) AS username_count FROM unique_multiple WHERE username = 'user_a';
SELECT COUNT(*) AS email_count FROM unique_multiple WHERE email = 'user_a@example.com';
SELECT COUNT(*) AS phone_count FROM unique_multiple WHERE phone = '1234567890';

-- ============================================================================
-- Section 11: UNIQUE Constraint - Violation Tests (Composite)
-- ============================================================================

-- Test 11.1: Attempt to insert duplicate composite key (same user_id and order_id)
INSERT INTO unique_composite VALUES (4, 100, 1001, 'PROD004');

-- Test 11.2: Insert with same user_id but different order_id (should succeed)
INSERT INTO unique_composite VALUES (5, 100, 1003, 'PROD005');

-- Test 11.3: Insert with different user_id but same order_id (should succeed)
INSERT INTO unique_composite VALUES (6, 102, 1001, 'PROD006');

-- Test 11.4: Verify correct data state
SELECT * FROM unique_composite ORDER BY id;

-- ============================================================================
-- Section 12: UNIQUE Constraint - Update Violation Tests
-- ============================================================================

-- Test 12.1: Update to duplicate username (should fail)
UPDATE unique_single SET username = 'user2' WHERE id = 1;

-- Test 12.2: Update to non-duplicate username (should succeed)
UPDATE unique_single SET username = 'user1_updated' WHERE id = 1;

-- Test 12.3: Verify update succeeded
SELECT * FROM unique_single WHERE id = 1;

-- Test 12.4: Update multiple UNIQUE column to existing value (should fail)
UPDATE unique_multiple SET email = 'user_b@example.com' WHERE id = 1;

-- Test 12.5: Update composite key to duplicate (should fail)
UPDATE unique_composite SET user_id = 100, order_id = 1002 WHERE id = 1;

-- Test 12.6: Update composite key to non-duplicate (should succeed)
UPDATE unique_composite SET user_id = 200, order_id = 2001 WHERE id = 1;

-- Test 12.7: Verify composite update succeeded
SELECT * FROM unique_composite WHERE id = 1;

-- ============================================================================
-- Section 13: UNIQUE Constraint - NULL Handling
-- ============================================================================

-- Test 13.1: Insert multiple NULL values in UNIQUE column (should succeed if NULLs are allowed)
INSERT INTO unique_single VALUES (10, NULL, 'null1@example.com');
INSERT INTO unique_single VALUES (11, NULL, 'null2@example.com');

-- Test 13.2: Verify NULL inserts
SELECT * FROM unique_single WHERE id IN (10, 11);

-- Test 13.3: Insert NULL in NOT NULL UNIQUE column (should fail)
INSERT INTO unique_notnull VALUES (1, NULL);

-- ============================================================================
-- Section 14: UNIQUE Constraint - SHOW CREATE TABLE
-- ============================================================================

-- Test 14.1: Verify UNIQUE constraints in table definition
SHOW CREATE TABLE unique_single;
SHOW CREATE TABLE unique_multiple;
SHOW CREATE TABLE unique_composite;

-- ============================================================================
-- Section 15: UNIQUE Constraint - DROP INDEX
-- ============================================================================

-- Test 15.1: Drop UNIQUE constraint
ALTER TABLE unique_single DROP INDEX uk_username;

-- Test 15.2: Verify constraint removed
SHOW CREATE TABLE unique_single;

-- Test 15.3: Now insert should succeed (no constraint)
INSERT INTO unique_single VALUES (20, 'user1', 'duplicate@example.com');

-- Test 15.4: Verify duplicate inserted
SELECT COUNT(*) AS count FROM unique_single WHERE username = 'user1';

-- ============================================================================
-- Section 16: User Management - DROP USER
-- ============================================================================

-- Test 16.1: Drop single user
DROP USER 'test_user4'@'localhost';

-- Test 16.2: Verify user dropped (show grants should fail)
SHOW GRANTS FOR 'test_user4'@'localhost';

-- Test 16.3: Drop multiple users
DROP USER 'test_user1'@'localhost', 'test_user2'@'localhost', 'test_user3'@'%';

-- ============================================================================
-- Section 17: Cleanup
-- ============================================================================

-- Drop test databases
DROP DATABASE IF EXISTS security_test_db;
DROP DATABASE IF EXISTS unique_test_db;

-- ============================================================================
-- Summary
-- ============================================================================

SELECT 'All Account/Security and UNIQUE constraint tests completed!' AS status;
SELECT 'Test coverage:' AS info
UNION ALL SELECT '1. CREATE USER with various options'
UNION ALL SELECT '2. ALTER USER password and properties'
UNION ALL SELECT '3. SET PASSWORD for users'
UNION ALL SELECT '4. GRANT privileges on databases and tables'
UNION ALL SELECT '5. SHOW GRANTS for users'
UNION ALL SELECT '6. REVOKE privileges and grant options'
UNION ALL SELECT '7. UNIQUE KEY table creation (single, multiple, composite)'
UNION ALL SELECT '8. UNIQUE constraint validation on INSERT'
UNION ALL SELECT '9. UNIQUE constraint validation on UPDATE'
UNION ALL SELECT '10. UNIQUE constraint NULL handling'
UNION ALL SELECT '11. DROP USER cleanup';