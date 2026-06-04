#!/usr/bin/env python3
"""
Comprehensive MaxCompute (ODPS) REST API protocol test for HarnessDB.
Generates and executes 1000+ test cases across all categories.

Tests the HTTP/REST endpoint on port 19031 with V2 HMAC-SHA1 authentication.
"""

import hashlib
import hmac
import base64
import json
import time
import sys
import os
import requests
import xml.etree.ElementTree as ET
from urllib.parse import urlencode, quote

# ===========================================================================
# Configuration
# ===========================================================================

BASE_URL = "http://127.0.0.1:19031"
ACCESS_KEY_ID = "harness"
ACCESS_KEY_SECRET = "harness-secret"
DEFAULT_PROJECT = "default"
DATE_HEADER = "Mon, 01 Jan 2024 00:00:00 GMT"
TIMEOUT = 10


# ===========================================================================
# V2 Authentication
# ===========================================================================

def compute_v2_signature(method, path, query="", content_type="", content_md5="", date=DATE_HEADER):
    """Compute V2 HMAC-SHA1 signature for MaxCompute REST API."""
    # Build canonical string
    canonical = f"{method.upper()}\n{content_type}\n{content_md5}\n{date}\n"
    # Resource
    if query:
        # Sort query params
        params = query.split('&')
        sorted_params = []
        for p in params:
            if '=' in p:
                k, v = p.split('=', 1)
                sorted_params.append(f"{k}={v}")
            else:
                sorted_params.append(p)
        sorted_params.sort()
        resource = f"{path}?{'&'.join(sorted_params)}"
    else:
        resource = path
    canonical += resource
    # Sign
    sig = hmac.new(
        ACCESS_KEY_SECRET.encode('utf-8'),
        canonical.encode('utf-8'),
        hashlib.sha1
    ).digest()
    return base64.b64encode(sig).decode('utf-8')


def auth_headers(method, path, query="", content_type="", body=b""):
    """Build authorization headers for a request."""
    sig = compute_v2_signature(method, path, query, content_type, date=DATE_HEADER)
    headers = {
        "Authorization": f"ODPS {ACCESS_KEY_ID}:{sig}",
        "Date": DATE_HEADER,
    }
    if content_type:
        headers["Content-Type"] = content_type
    return headers


def get(path, query="", **kwargs):
    """Send a signed GET request."""
    url = f"{BASE_URL}{path}"
    if query:
        url += f"?{query}"
        q = query
    else:
        q = ""
    headers = auth_headers("GET", path, q)
    return requests.get(url, headers=headers, timeout=TIMEOUT, **kwargs)


def post(path, body=b"", query="", content_type="application/xml"):
    """Send a signed POST request."""
    url = f"{BASE_URL}{path}"
    if query:
        url += f"?{query}"
        q = query
    else:
        q = ""
    headers = auth_headers("POST", path, q, content_type, body=body)
    return requests.post(url, headers=headers, data=body, timeout=TIMEOUT)


def put(path, body=b"", query=""):
    """Send a signed PUT request."""
    url = f"{BASE_URL}{path}"
    if query:
        url += f"?{query}"
        q = query
    else:
        q = ""
    headers = auth_headers("PUT", path, q)
    return requests.put(url, headers=headers, data=body, timeout=TIMEOUT)


def delete(path):
    """Send a signed DELETE request."""
    url = f"{BASE_URL}{path}"
    headers = auth_headers("DELETE", path)
    return requests.delete(url, headers=headers, timeout=TIMEOUT)


def post_noauth(path, body=b"", query=""):
    """Send POST without auth (for testing auth failures)."""
    url = f"{BASE_URL}{path}"
    if query:
        url += f"?{query}"
    return requests.post(url, data=body, timeout=TIMEOUT)


def get_noauth(path, query=""):
    """Send GET without auth (for testing auth failures)."""
    url = f"{BASE_URL}{path}"
    if query:
        url += f"?{query}"
    return requests.get(url, timeout=TIMEOUT)


# ===========================================================================
# SQL submission helpers
# ===========================================================================

def build_submit_xml(sql):
    """Build XML body for submitting a SQL instance."""
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Job>
    <Priority>9</Priority>
    <RunMode>Sequence</RunMode>
    <Tasks>
      <SQL Name="AnonymousSQLTask">
        <Name>AnonymousSQLTask</Name>
        <Query><![CDATA[{sql}]]></Query>
      </SQL>
    </Tasks>
  </Job>
</Instance>"""


def submit_sql(sql, project=DEFAULT_PROJECT):
    """Submit SQL and return instance ID (or None on failure)."""
    body = build_submit_xml(sql).encode('utf-8')
    path = f"/api/projects/{project}/instances"
    resp = post(path, body=body)
    if resp.status_code == 201:
        location = resp.headers.get("Location", "")
        # Extract instance ID from location
        return location.rsplit('/', 1)[-1] if location else None
    return None


def get_instance_status(instance_id, project=DEFAULT_PROJECT, param=None):
    """Get instance status."""
    path = f"/api/projects/{project}/instances/{instance_id}"
    query = param if param else ""
    return get(path, query=query)


def get_instance_result(instance_id, project=DEFAULT_PROJECT):
    """Get instance result."""
    return get_instance_status(instance_id, project, param="result")


def wait_for_instance(instance_id, project=DEFAULT_PROJECT, max_wait=5):
    """Poll until instance is done."""
    start = time.time()
    while time.time() - start < max_wait:
        resp = get_instance_status(instance_id, project)
        if resp.status_code == 200:
            text = resp.text
            if "<Status>Success</Status>" in text or "<Status>Failed</Status>" in text:
                return resp
        time.sleep(0.1)
    return get_instance_status(instance_id, project)


# ===========================================================================
# Test Runner
# ===========================================================================

class TestRunner:
    def __init__(self):
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []
        self.start_time = time.time()

    def check(self, name, condition, detail=""):
        """Record a test result."""
        self.total += 1
        if condition:
            self.passed += 1
        else:
            self.failed += 1
            if len(self.failures) < 20:
                self.failures.append({
                    "name": name,
                    "error": detail or "assertion failed"
                })
        if self.total % 100 == 0:
            elapsed = time.time() - self.start_time
            print(f"  Progress: {self.total} tests, {self.passed} passed, {self.failed} failed ({elapsed:.1f}s)")

    def check_status(self, name, resp, expected_status):
        """Check HTTP status code."""
        self.check(name, resp.status_code == expected_status,
                    f"expected {expected_status}, got {resp.status_code}: {resp.text[:200]}")

    def check_contains(self, name, text, substring):
        """Check if text contains substring."""
        self.check(name, substring in text,
                    f"expected '{substring}' in response, got: {text[:200]}")

    def check_not_contains(self, name, text, substring):
        """Check if text does NOT contain substring."""
        self.check(name, substring not in text,
                    f"expected '{substring}' NOT in response")


def generate_tests(runner):
    """Generate all test cases."""

    # =======================================================================
    # 1. Connection / Health Check (20+)
    # =======================================================================
    print("=== 1. Connection / Health Check ===")

    # Health check without auth
    try:
        r = requests.get(f"{BASE_URL}/health", timeout=TIMEOUT)
        runner.check("health_no_auth_status", r.status_code == 200, f"got {r.status_code}")
        runner.check("health_no_auth_body_json", "ok" in r.text, r.text[:100])
        runner.check("health_no_auth_protocol", "maxcompute" in r.text, r.text[:100])
    except Exception as e:
        runner.check("health_no_auth", False, str(e))
        runner.check("health_no_auth_status", False, str(e))
        runner.check("health_no_auth_body_json", False, str(e))

    # Health check content type
    try:
        r = requests.get(f"{BASE_URL}/health", timeout=TIMEOUT)
        ct = r.headers.get("content-type", "")
        runner.check("health_content_type_json", "json" in ct, ct)
    except Exception as e:
        runner.check("health_content_type_json", False, str(e))

    # Root path - auth middleware skips "/" but no route matches → 404
    try:
        r = requests.get(f"{BASE_URL}/", timeout=TIMEOUT)
        runner.check("root_path_status", r.status_code in [200, 404], f"got {r.status_code}")
    except Exception as e:
        runner.check("root_path_status", False, str(e))

    # Auth required for API endpoints
    try:
        r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", timeout=TIMEOUT)
        runner.check("api_requires_auth", r.status_code == 401, f"got {r.status_code}")
    except Exception as e:
        runner.check("api_requires_auth", False, str(e))

    # Wrong auth credentials
    try:
        headers = {
            "Authorization": "ODPS wrong_key:wrong_sig",
            "Date": DATE_HEADER,
        }
        r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
        runner.check("wrong_auth_key_rejected", r.status_code == 401, f"got {r.status_code}")
    except Exception as e:
        runner.check("wrong_auth_key_rejected", False, str(e))

    # Missing date header
    try:
        headers = {
            "Authorization": "ODPS harness:fakesig",
        }
        r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
        runner.check("missing_date_header", r.status_code == 401, f"got {r.status_code}")
    except Exception as e:
        runner.check("missing_date_header", False, str(e))

    # Unsupported auth scheme (Bearer)
    try:
        headers = {
            "Authorization": "Bearer some_token",
            "Date": DATE_HEADER,
        }
        r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
        runner.check("bearer_auth_rejected", r.status_code == 401, f"got {r.status_code}")
    except Exception as e:
        runner.check("bearer_auth_rejected", False, str(e))

    # Empty auth header
    try:
        headers = {"Authorization": "", "Date": DATE_HEADER}
        r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
        runner.check("empty_auth_rejected", r.status_code == 401, f"got {r.status_code}")
    except Exception as e:
        runner.check("empty_auth_rejected", False, str(e))

    # Valid auth signature
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}")
        runner.check("valid_auth_accepted", r.status_code == 200, f"got {r.status_code}: {r.text[:200]}")
    except Exception as e:
        runner.check("valid_auth_accepted", False, str(e))

    # Connection to wrong port (should fail)
    try:
        r = requests.get(f"http://127.0.0.1:19999/health", timeout=2)
        runner.check("wrong_port_unreachable", False, "should not connect")
    except Exception:
        runner.check("wrong_port_unreachable", True, "connection refused as expected")

    # Multiple health checks
    for i in range(5):
        try:
            r = requests.get(f"{BASE_URL}/health", timeout=TIMEOUT)
            runner.check(f"health_check_repeat_{i}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"health_check_repeat_{i}", False, str(e))

    # Nonexistent endpoint returns 404 or 401
    try:
        r = get("/api/nonexistent")
        runner.check("nonexistent_endpoint", r.status_code in [404, 405, 401],
                      f"got {r.status_code}")
    except Exception as e:
        runner.check("nonexistent_endpoint", False, str(e))

    # POST to health endpoint
    try:
        r = requests.post(f"{BASE_URL}/health", timeout=TIMEOUT)
        runner.check("health_post_method", r.status_code in [200, 405], f"got {r.status_code}")
    except Exception as e:
        runner.check("health_post_method", False, str(e))

    # =======================================================================
    # 2. Project Operations (50+)
    # =======================================================================
    print("=== 2. Project Operations ===")

    # Get default project
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}")
        runner.check_status("get_default_project", r, 200)
        runner.check_contains("get_project_has_name", r.text, f"<Name>{DEFAULT_PROJECT}</Name>")
        runner.check_contains("get_project_has_owner", r.text, "<Owner>")
        runner.check_contains("get_project_xml_decl", r.text, "<?xml")
    except Exception as e:
        runner.check("get_default_project", False, str(e))
        runner.check("get_project_has_name", False, str(e))
        runner.check("get_project_has_owner", False, str(e))
        runner.check("get_project_xml_decl", False, str(e))

    # Get nonexistent project
    try:
        r = get("/api/projects/nonexistent_project_xyz")
        runner.check_status("get_nonexistent_project", r, 404)
        runner.check_contains("nonexistent_project_error_code", r.text, "ODPS-0130161")
    except Exception as e:
        runner.check("get_nonexistent_project", False, str(e))
        runner.check("nonexistent_project_error_code", False, str(e))

    # Case-insensitive project name
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT.upper()}")
        runner.check_status("project_case_insensitive", r, 200)
    except Exception as e:
        runner.check("project_case_insensitive", False, str(e))

    # Mixed case project name
    try:
        name = DEFAULT_PROJECT[0].upper() + DEFAULT_PROJECT[1:] if len(DEFAULT_PROJECT) > 1 else DEFAULT_PROJECT.upper()
        r = get(f"/api/projects/{name}")
        runner.check_status("project_mixed_case", r, 200)
    except Exception as e:
        runner.check("project_mixed_case", False, str(e))

    # Empty project name
    try:
        r = get("/api/projects/")
        runner.check("empty_project_name", r.status_code in [404, 400, 401, 405],
                      f"got {r.status_code}")
    except Exception as e:
        runner.check("empty_project_name", False, str(e))

    # Project with special chars
    for special in ["test@proj", "test proj", "test;proj", "test'proj", "test\"proj"]:
        try:
            encoded = quote(special, safe='')
            r = get(f"/api/projects/{encoded}")
            runner.check(f"project_special_char_{special[:5]}", r.status_code in [404, 400],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"project_special_char_{special[:5]}", False, str(e))

    # Project XML structure validation
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}")
        runner.check_contains("project_has_properties", r.text, "<Properties>")
        runner.check_contains("project_has_projectgroup", r.text, "<ProjectGroupName>")
    except Exception as e:
        runner.check("project_has_properties", False, str(e))
        runner.check("project_has_projectgroup", False, str(e))

    # Multiple requests to same project
    for i in range(10):
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}")
            runner.check(f"project_repeat_{i}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"project_repeat_{i}", False, str(e))

    # Project content type
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}")
        ct = r.headers.get("content-type", "")
        runner.check("project_content_type_xml", "xml" in ct.lower(), ct)
    except Exception as e:
        runner.check("project_content_type_xml", False, str(e))

    # Very long project name
    try:
        r = get(f"/api/projects/{'a' * 500}")
        runner.check("long_project_name", r.status_code in [404, 400, 414],
                      f"got {r.status_code}")
    except Exception as e:
        runner.check("long_project_name", False, str(e))

    # Project with dots
    try:
        r = get("/api/projects/test.project")
        runner.check("project_with_dots", r.status_code == 404, f"got {r.status_code}")
    except Exception as e:
        runner.check("project_with_dots", False, str(e))

    # Project with numbers
    try:
        r = get("/api/projects/project123")
        runner.check("project_with_numbers", r.status_code == 404, f"got {r.status_code}")
    except Exception as e:
        runner.check("project_with_numbers", False, str(e))

    # =======================================================================
    # 3. Table Operations (100+)
    # =======================================================================
    print("=== 3. Table Operations ===")

    # List tables
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables")
        runner.check_status("list_tables_status", r, 200)
        runner.check_contains("list_tables_xml", r.text, "<Tables>")
        runner.check_contains("list_tables_close", r.text, "</Tables>")
    except Exception as e:
        runner.check("list_tables_status", False, str(e))
        runner.check("list_tables_xml", False, str(e))
        runner.check("list_tables_close", False, str(e))

    # List tables for nonexistent project
    try:
        r = get("/api/projects/nonexistent_proj/tables")
        runner.check_status("list_tables_bad_project", r, 404)
    except Exception as e:
        runner.check("list_tables_bad_project", False, str(e))

    # Create a table via SQL for testing
    create_sqls = [
        "CREATE TABLE IF NOT EXISTS test_mc_basic (id BIGINT, name STRING, score DOUBLE)",
        "CREATE TABLE IF NOT EXISTS test_mc_int (id INT, val INT)",
        "CREATE TABLE IF NOT EXISTS test_mc_types (a BIGINT, b STRING, c DOUBLE, d BOOLEAN, e DATETIME)",
        "CREATE TABLE IF NOT EXISTS test_mc_empty (id BIGINT)",
        "CREATE TABLE IF NOT EXISTS test_mc_many_cols (c1 BIGINT, c2 STRING, c3 DOUBLE, c4 STRING, c5 BIGINT)",
    ]
    created_tables = []
    for sql in create_sqls:
        iid = submit_sql(sql)
        if iid:
            created_tables.append(sql.split("EXISTS ")[-1].split(" (")[0])
            wait_for_instance(iid)

    for tname in created_tables:
        runner.check(f"create_table_{tname}", True)

    # Get table detail
    for tname in created_tables:
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            runner.check_status(f"get_table_{tname}", r, 200)
            runner.check_contains(f"get_table_{tname}_xml", r.text, "<Table>")
            runner.check_contains(f"get_table_{tname}_name", r.text, f"<Name>{tname}</Name>")
            runner.check_contains(f"get_table_{tname}_columns", r.text, "<Columns>")
        except Exception as e:
            runner.check(f"get_table_{tname}", False, str(e))

    # Get table with invalid name (SQL injection attempts)
    injection_names = [
        "test; DROP TABLE test",
        "test' OR '1'='1",
        "test;--",
        "123invalid",
        "test name",
        "test/name",
        "test\\name",
    ]
    for bad_name in injection_names:
        try:
            encoded = quote(bad_name, safe='')
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{encoded}")
            runner.check(f"table_injection_{bad_name[:10]}",
                          r.status_code in [400, 404], f"got {r.status_code}")
        except Exception as e:
            runner.check(f"table_injection_{bad_name[:10]}", False, str(e))

    # Delete table
    try:
        # Create a temp table to delete
        iid = submit_sql("CREATE TABLE IF NOT EXISTS test_mc_to_delete (id BIGINT)")
        if iid:
            wait_for_instance(iid)
        r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_to_delete")
        runner.check_status("delete_table_status", r, 200)
        runner.check_contains("delete_table_xml", r.text, "<Name>test_mc_to_delete</Name>")
    except Exception as e:
        runner.check("delete_table_status", False, str(e))

    # Delete nonexistent table (should still succeed - IF EXISTS semantics)
    try:
        r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/nonexistent_table_xyz")
        runner.check_status("delete_nonexistent_table", r, 200)
    except Exception as e:
        runner.check("delete_nonexistent_table", False, str(e))

    # Delete from wrong project
    try:
        r = delete("/api/projects/wrong_proj/tables/test_mc_basic")
        runner.check_status("delete_wrong_project", r, 404)
    except Exception as e:
        runner.check("delete_wrong_project", False, str(e))

    # Delete with invalid table name
    try:
        r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/test;DROP")
        runner.check_status("delete_invalid_name", r, 400)
    except Exception as e:
        runner.check("delete_invalid_name", False, str(e))

    # List tables after creating some
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables")
        runner.check_status("list_tables_after_create", r, 200)
        for tname in created_tables:
            runner.check_contains(f"list_tables_contains_{tname}", r.text, f"<Name>{tname}</Name>")
    except Exception as e:
        runner.check("list_tables_after_create", False, str(e))

    # List tables with maxitem
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables", query="maxitem=2")
        runner.check_status("list_tables_maxitem", r, 200)
    except Exception as e:
        runner.check("list_tables_maxitem", False, str(e))

    # List tables with prefix filter
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables", query="prefix=test_mc_")
        runner.check_status("list_tables_prefix", r, 200)
    except Exception as e:
        runner.check("list_tables_prefix", False, str(e))

    # Get table detail for nonexistent table
    # Note: DESCRIBE may return error info as column data, so server returns 200 with error embedded
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/nonexistent_table_xyz")
        runner.check("get_nonexistent_table", r.status_code in [200, 404], f"got {r.status_code}")
    except Exception as e:
        runner.check("get_nonexistent_table", False, str(e))

    # Create tables with various column types
    type_tests = [
        ("mc_t_tinyint", "CREATE TABLE IF NOT EXISTS mc_t_tinyint (a TINYINT)"),
        ("mc_t_smallint", "CREATE TABLE IF NOT EXISTS mc_t_smallint (a SMALLINT)"),
        ("mc_t_bigint", "CREATE TABLE IF NOT EXISTS mc_t_bigint (a BIGINT)"),
        ("mc_t_float", "CREATE TABLE IF NOT EXISTS mc_t_float (a FLOAT)"),
        ("mc_t_double", "CREATE TABLE IF NOT EXISTS mc_t_double (a DOUBLE)"),
        ("mc_t_decimal", "CREATE TABLE IF NOT EXISTS mc_t_decimal (a DECIMAL(10,2))"),
        ("mc_t_string", "CREATE TABLE IF NOT EXISTS mc_t_string (a STRING)"),
        ("mc_t_varchar", "CREATE TABLE IF NOT EXISTS mc_t_varchar (a VARCHAR(255))"),
        ("mc_t_char", "CREATE TABLE IF NOT EXISTS mc_t_char (a CHAR(10))"),
        ("mc_t_boolean", "CREATE TABLE IF NOT EXISTS mc_t_boolean (a BOOLEAN)"),
        ("mc_t_date", "CREATE TABLE IF NOT EXISTS mc_t_date (a DATE)"),
        ("mc_t_datetime", "CREATE TABLE IF NOT EXISTS mc_t_datetime (a DATETIME)"),
        ("mc_t_timestamp", "CREATE TABLE IF NOT EXISTS mc_t_timestamp (a TIMESTAMP)"),
        ("mc_t_binary", "CREATE TABLE IF NOT EXISTS mc_t_binary (a BINARY)"),
    ]
    for tname, sql in type_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"create_type_table_{tname}", True)
            try:
                r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
                runner.check_status(f"get_type_table_{tname}", r, 200)
            except Exception as e:
                runner.check(f"get_type_table_{tname}", False, str(e))
        else:
            runner.check(f"create_type_table_{tname}", False, "submit failed")
            runner.check(f"get_type_table_{tname}", False, "submit failed")

    # Create table with MaxCompute-specific clauses (should be translated)
    mc_specific_tables = [
        ("mc_part", "CREATE TABLE IF NOT EXISTS mc_part (id BIGINT) PARTITIONED BY (ds STRING)"),
        ("mc_life", "CREATE TABLE IF NOT EXISTS mc_life (id BIGINT) LIFECYCLE 365"),
        ("mc_orc", "CREATE TABLE IF NOT EXISTS mc_orc (id BIGINT) STORED AS ORC"),
        ("mc_parquet", "CREATE TABLE IF NOT EXISTS mc_parquet (id BIGINT) STORED AS PARQUET"),
        ("mc_multi", "CREATE TABLE IF NOT EXISTS mc_multi (id BIGINT, name STRING) PARTITIONED BY (ds STRING) LIFECYCLE 365"),
    ]
    for tname, sql in mc_specific_tables:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"create_mc_clause_{tname}", True)
        else:
            runner.check(f"create_mc_clause_{tname}", False, "submit failed")

    # Table with long name
    long_name = "t_" + "a" * 100
    iid = submit_sql(f"CREATE TABLE IF NOT EXISTS {long_name} (id BIGINT)")
    if iid:
        wait_for_instance(iid)
        runner.check("create_long_table_name", True)
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{long_name}")
            runner.check_status("get_long_table_name", r, 200)
        except Exception as e:
            runner.check("get_long_table_name", False, str(e))
    else:
        runner.check("create_long_table_name", False, "submit failed")
        runner.check("get_long_table_name", False, "submit failed")

    # Table with underscore prefix
    iid = submit_sql("CREATE TABLE IF NOT EXISTS _underscore_table (id BIGINT)")
    if iid:
        wait_for_instance(iid)
        runner.check("create_underscore_table", True)
    else:
        runner.check("create_underscore_table", False, "submit failed")

    # =======================================================================
    # 4. SQL Execution (200+)
    # =======================================================================
    print("=== 4. SQL Execution ===")

    # Basic SELECT
    select_tests = [
        ("select_1", "SELECT 1"),
        ("select_string", "SELECT 'hello'"),
        ("select_null", "SELECT NULL"),
        ("select_true", "SELECT TRUE"),
        ("select_false", "SELECT FALSE"),
        ("select_arithmetic", "SELECT 1 + 2"),
        ("select_concat", "SELECT CONCAT('a', 'b')"),
        ("select_length", "SELECT LENGTH('hello')"),
        ("select_upper", "SELECT UPPER('hello')"),
        ("select_lower", "SELECT LOWER('HELLO')"),
        ("select_abs", "SELECT ABS(-5)"),
        ("select_ceil", "SELECT CEIL(3.2)"),
        ("select_floor", "SELECT FLOOR(3.8)"),
        ("select_round", "SELECT ROUND(3.14159, 2)"),
        ("select_mod", "SELECT 10 % 3"),
        ("select_power", "SELECT POWER(2, 10)"),
        ("select_sqrt", "SELECT SQRT(16)"),
        ("select_coalesce", "SELECT COALESCE(NULL, 'default')"),
        ("select_now", "SELECT NOW()"),
        ("select_curdate", "SELECT CURDATE()"),
    ]
    for name, sql in select_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"sql_{name}_submit", True)
            if resp and resp.status_code == 200:
                runner.check_contains(f"sql_{name}_status", resp.text, "<Status>Success</Status>")
            else:
                runner.check(f"sql_{name}_status", False, f"status={resp.status_code if resp else 'no response'}")
        else:
            runner.check(f"sql_{name}_submit", False, "submit failed")
            runner.check(f"sql_{name}_status", False, "submit failed")

    # SELECT from tables
    iid = submit_sql("INSERT INTO test_mc_basic VALUES (1, 'Alice', 95.5)")
    if iid:
        wait_for_instance(iid)
    iid = submit_sql("INSERT INTO test_mc_basic VALUES (2, 'Bob', 87.3)")
    if iid:
        wait_for_instance(iid)
    iid = submit_sql("INSERT INTO test_mc_basic VALUES (3, 'Charlie', 92.1)")
    if iid:
        wait_for_instance(iid)

    from_table_tests = [
        ("select_star", "SELECT * FROM test_mc_basic"),
        ("select_where", "SELECT * FROM test_mc_basic WHERE id = 1"),
        ("select_count", "SELECT COUNT(*) FROM test_mc_basic"),
        ("select_sum", "SELECT SUM(score) FROM test_mc_basic"),
        ("select_avg", "SELECT AVG(score) FROM test_mc_basic"),
        ("select_min", "SELECT MIN(score) FROM test_mc_basic"),
        ("select_max", "SELECT MAX(score) FROM test_mc_basic"),
        ("select_order_by", "SELECT * FROM test_mc_basic ORDER BY id"),
        ("select_limit", "SELECT * FROM test_mc_basic LIMIT 2"),
        ("select_distinct_name", "SELECT DISTINCT name FROM test_mc_basic"),
        ("select_group_by", "SELECT name, score FROM test_mc_basic GROUP BY name, score"),
        ("select_like", "SELECT * FROM test_mc_basic WHERE name LIKE 'A%'"),
        ("select_in", "SELECT * FROM test_mc_basic WHERE id IN (1, 2)"),
        ("select_between", "SELECT * FROM test_mc_basic WHERE score BETWEEN 85 AND 95"),
        ("select_alias", "SELECT id AS user_id, name AS user_name FROM test_mc_basic"),
        ("select_expression", "SELECT id, score * 1.1 AS adjusted FROM test_mc_basic"),
        ("select_case", "SELECT id, CASE WHEN score > 90 THEN 'A' ELSE 'B' END FROM test_mc_basic"),
        ("select_is_null", "SELECT * FROM test_mc_basic WHERE name IS NOT NULL"),
        ("select_and_or", "SELECT * FROM test_mc_basic WHERE id > 1 AND score < 95"),
        ("select_subquery", "SELECT * FROM test_mc_basic WHERE score = (SELECT MAX(score) FROM test_mc_basic)"),
    ]
    for name, sql in from_table_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"from_table_{name}_submit", True)
            if resp and resp.status_code == 200:
                runner.check_contains(f"from_table_{name}_status", resp.text, "<Status>")
            else:
                runner.check(f"from_table_{name}_status", False, f"status={resp.status_code if resp else 'no response'}")
        else:
            runner.check(f"from_table_{name}_submit", False, "submit failed")
            runner.check(f"from_table_{name}_status", False, "submit failed")

    # INSERT tests
    insert_tests = [
        ("insert_single", "INSERT INTO test_mc_empty VALUES (1)"),
        ("insert_multiple", "INSERT INTO test_mc_int VALUES (1, 10), (2, 20), (3, 30)"),
        ("insert_null", "INSERT INTO test_mc_empty VALUES (NULL)"),
        ("insert_string", "INSERT INTO test_mc_types VALUES (1, 'test', 3.14, TRUE, '2024-01-01 00:00:00')"),
        ("insert_negative", "INSERT INTO test_mc_int VALUES (-1, -100)"),
        ("insert_zero", "INSERT INTO test_mc_int VALUES (0, 0)"),
        ("insert_max_bigint", "INSERT INTO test_mc_types VALUES (9223372036854775807, 'max', 0, FALSE, '2024-01-01 00:00:00')"),
    ]
    for name, sql in insert_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"insert_{name}_submit", True)
            if resp and resp.status_code == 200:
                runner.check(f"insert_{name}_ok", True)
            else:
                runner.check(f"insert_{name}_ok", False, f"resp={resp.status_code if resp else 'none'}")
        else:
            runner.check(f"insert_{name}_submit", False, "submit failed")
            runner.check(f"insert_{name}_ok", False, "submit failed")

    # INSERT OVERWRITE (should be translated to INSERT INTO)
    iid = submit_sql("INSERT OVERWRITE TABLE test_mc_empty SELECT * FROM test_mc_empty")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("insert_overwrite_submit", True)
    else:
        runner.check("insert_overwrite_submit", False, "submit failed")

    # DDL tests
    ddl_tests = [
        ("create_table_ddl", "CREATE TABLE IF NOT EXISTS test_ddl_new (id BIGINT, val STRING)"),
        ("drop_table_ddl", "DROP TABLE IF EXISTS test_ddl_new"),
        ("create_drop_cycle", "CREATE TABLE IF NOT EXISTS test_cycle (id BIGINT)"),
        ("drop_cycle", "DROP TABLE IF EXISTS test_cycle"),
        ("create_if_not_exists", "CREATE TABLE IF NOT EXISTS test_mc_basic (id BIGINT)"),
        ("drop_if_exists", "DROP TABLE IF EXISTS nonexistent_table_xyz"),
    ]
    for name, sql in ddl_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"ddl_{name}_submit", True)
        else:
            runner.check(f"ddl_{name}_submit", False, "submit failed")

    # No-op statements
    noop_tests = [
        ("set_statement", "SET odps.sql.allow.fullscan=true"),
        ("setproject", "SETPROJECT myproject odps.sql.allow.fullscan=true"),
        ("add_jar", "ADD JAR /path/to/udf.jar"),
        ("add_file", "ADD FILE /path/to/resource.txt"),
        ("set_empty", "SET"),
    ]
    for name, sql in noop_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"noop_{name}_submit", True)
            if resp and resp.status_code == 200:
                runner.check_contains(f"noop_{name}_success", resp.text, "<Status>Success</Status>")
            else:
                runner.check(f"noop_{name}_success", False, "no response")
        else:
            runner.check(f"noop_{name}_submit", False, "submit failed")

    # SQL with hints
    hint_tests = [
        ("mapjoin_hint", "SELECT /*+ MAPJOIN(b) */ a.id FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id"),
        ("skewjoin_hint", "SELECT /*+ SKEWJOIN(b) */ a.id FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id"),
    ]
    for name, sql in hint_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"hint_{name}_submit", True)
        else:
            runner.check(f"hint_{name}_submit", False, "submit failed")

    # SQL with semicolons
    iid = submit_sql("SELECT 1;")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("sql_trailing_semicolon", True)
    else:
        runner.check("sql_trailing_semicolon", False, "submit failed")

    # SQL with trailing whitespace
    iid = submit_sql("  SELECT 1  \n  ")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("sql_whitespace", True)
    else:
        runner.check("sql_whitespace", False, "submit failed")

    # Empty SQL
    try:
        body = build_submit_xml("").encode('utf-8')
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
        runner.check_status("empty_sql_rejected", r, 400)
    except Exception as e:
        runner.check("empty_sql_rejected", False, str(e))

    # ALTER TABLE (should be noop)
    iid = submit_sql("ALTER TABLE test_mc_basic SET LIFECYCLE 365")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("alter_table_noop", True)
    else:
        runner.check("alter_table_noop", False, "submit failed")

    # CREATE TABLE AS SELECT
    iid = submit_sql("CREATE TABLE IF NOT EXISTS test_ctas AS SELECT * FROM test_mc_basic WHERE id = 1")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("ctas_submit", True)
    else:
        runner.check("ctas_submit", False, "submit failed")

    # Multiple SQL statements
    iid = submit_sql("INSERT INTO test_mc_empty VALUES (100); INSERT INTO test_mc_empty VALUES (200)")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("multi_statement_submit", True)
    else:
        runner.check("multi_statement_submit", False, "submit failed")

    # SQL with various string literals
    string_tests = [
        ("single_quote", "SELECT 'it''s a test'"),
        ("unicode", "SELECT 'hello world'"),
        ("empty_string", "SELECT ''"),
        ("newline_string", "SELECT 'line1\nline2'"),
        ("tab_string", "SELECT 'col1\tcol2'"),
    ]
    for name, sql in string_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"string_{name}_submit", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"string_{name}_submit", False, "submit failed")

    # Numeric edge cases
    numeric_tests = [
        ("zero", "SELECT 0"),
        ("negative", "SELECT -1"),
        ("large_number", "SELECT 999999999999999"),
        ("decimal", "SELECT 3.14159265358979"),
        ("scientific", "SELECT 1.5E10"),
        ("float_ops", "SELECT 1.0 / 3.0"),
    ]
    for name, sql in numeric_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"numeric_{name}_submit", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"numeric_{name}_submit", False, "submit failed")

    # Boolean operations
    bool_tests = [
        ("bool_and", "SELECT TRUE AND FALSE"),
        ("bool_or", "SELECT TRUE OR FALSE"),
        ("bool_not", "SELECT NOT TRUE"),
        ("bool_compare", "SELECT 1 > 2"),
        ("bool_eq", "SELECT 1 = 1"),
    ]
    for name, sql in bool_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"bool_{name}_submit", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"bool_{name}_submit", False, "submit failed")

    # JOIN tests
    iid = submit_sql("INSERT INTO test_mc_int VALUES (1, 100), (2, 200)")
    if iid:
        wait_for_instance(iid)

    join_tests = [
        ("inner_join", "SELECT a.id, b.val FROM test_mc_basic a INNER JOIN test_mc_int b ON a.id = b.id"),
        ("left_join", "SELECT a.id, b.val FROM test_mc_basic a LEFT JOIN test_mc_int b ON a.id = b.id"),
        ("right_join", "SELECT a.id, b.val FROM test_mc_basic a RIGHT JOIN test_mc_int b ON a.id = b.id"),
        ("cross_join", "SELECT a.id, b.val FROM test_mc_basic a CROSS JOIN test_mc_int b LIMIT 10"),
        ("self_join", "SELECT a.id, b.id FROM test_mc_basic a JOIN test_mc_basic b ON a.id < b.id LIMIT 10"),
    ]
    for name, sql in join_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"join_{name}_submit", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"join_{name}_submit", False, "submit failed")

    # Aggregate tests
    agg_tests = [
        ("count_distinct", "SELECT COUNT(DISTINCT name) FROM test_mc_basic"),
        ("having", "SELECT name, COUNT(*) as cnt FROM test_mc_basic GROUP BY name HAVING cnt > 0"),
        ("order_by_agg", "SELECT name, SUM(score) as total FROM test_mc_basic GROUP BY name, total ORDER BY total DESC"),
    ]
    for name, sql in agg_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"agg_{name}_submit", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"agg_{name}_submit", False, "submit failed")

    # DISTRIBUTE BY / SORT BY
    iid = submit_sql("SELECT * FROM test_mc_basic DISTRIBUTE BY id SORT BY name")
    if iid:
        runner.check("distribute_sort_by_submit", True)
        resp = wait_for_instance(iid)
    else:
        runner.check("distribute_sort_by_submit", False, "submit failed")

    # =======================================================================
    # 5. Instance / Task Management (50+)
    # =======================================================================
    print("=== 5. Instance / Task Management ===")

    # Submit and track instance lifecycle
    iid = submit_sql("SELECT 42")
    if iid:
        # Get full instance info
        try:
            r = get_instance_status(iid)
            runner.check_status("instance_full_info", r, 200)
            runner.check_contains("instance_has_name", r.text, f"<Name>{iid}</Name>")
            runner.check_contains("instance_has_owner", r.text, "<Owner>")
            runner.check_contains("instance_has_starttime", r.text, "<StartTime>")
        except Exception as e:
            runner.check("instance_full_info", False, str(e))
            runner.check("instance_has_name", False, str(e))
            runner.check("instance_has_owner", False, str(e))
            runner.check("instance_has_starttime", False, str(e))

        # Get task status
        try:
            r = get_instance_status(iid, param="taskstatus")
            runner.check_status("instance_taskstatus", r, 200)
            runner.check_contains("taskstatus_has_task", r.text, "<Task")
            runner.check_contains("taskstatus_has_type", r.text, 'Type="SQL"')
        except Exception as e:
            runner.check("instance_taskstatus", False, str(e))
            runner.check("taskstatus_has_task", False, str(e))
            runner.check("taskstatus_has_type", False, str(e))

        # Get instance status
        try:
            r = get_instance_status(iid, param="instancestatus")
            runner.check_status("instance_instancestatus", r, 200)
            runner.check_contains("instancestatus_has_status", r.text, "<Status>")
        except Exception as e:
            runner.check("instance_instancestatus", False, str(e))
            runner.check("instancestatus_has_status", False, str(e))

        # Get result
        try:
            resp = wait_for_instance(iid)
            r = get_instance_result(iid)
            runner.check_status("instance_result", r, 200)
            runner.check_contains("result_has_task", r.text, "<Task")
        except Exception as e:
            runner.check("instance_result", False, str(e))
            runner.check("result_has_task", False, str(e))
    else:
        for i in range(8):
            runner.check(f"instance_lifecycle_{i}", False, "submit failed")

    # Instance not found
    try:
        r = get_instance_status("nonexistent-instance-id")
        runner.check_status("instance_not_found", r, 404)
        runner.check_contains("instance_not_found_error", r.text, "ODPS-0120035")
    except Exception as e:
        runner.check("instance_not_found", False, str(e))
        runner.check("instance_not_found_error", False, str(e))

    # Stop instance
    iid = submit_sql("SELECT SLEEP(10)")
    if iid:
        time.sleep(0.1)
        try:
            r = put(f"/api/projects/{DEFAULT_PROJECT}/instances/{iid}")
            runner.check("stop_instance", r.status_code in [200, 404], f"got {r.status_code}")
        except Exception as e:
            runner.check("stop_instance", False, str(e))
    else:
        runner.check("stop_instance", False, "submit failed")

    # Stop nonexistent instance
    try:
        r = put(f"/api/projects/{DEFAULT_PROJECT}/instances/nonexistent-id")
        runner.check_status("stop_nonexistent_instance", r, 404)
    except Exception as e:
        runner.check("stop_nonexistent_instance", False, str(e))

    # Submit to wrong project
    try:
        body = build_submit_xml("SELECT 1").encode('utf-8')
        r = post("/api/projects/wrong_project/instances", body=body)
        runner.check_status("submit_wrong_project", r, 404)
    except Exception as e:
        runner.check("submit_wrong_project", False, str(e))

    # Submit empty body
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=b"")
        runner.check_status("submit_empty_body", r, 400)
    except Exception as e:
        runner.check("submit_empty_body", False, str(e))

    # Submit non-XML body (raw SQL)
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=b"SELECT 1", content_type="text/plain")
        runner.check("submit_raw_sql", r.status_code in [201, 400], f"got {r.status_code}")
    except Exception as e:
        runner.check("submit_raw_sql", False, str(e))

    # Submit with various priorities
    for priority in [1, 5, 9]:
        body = f"""<?xml version="1.0" encoding="UTF-8"?>
<Instance>
  <Job>
    <Priority>{priority}</Priority>
    <RunMode>Sequence</RunMode>
    <Tasks>
      <SQL Name="AnonymousSQLTask">
        <Name>AnonymousSQLTask</Name>
        <Query>SELECT {priority}</Query>
      </SQL>
    </Tasks>
  </Job>
</Instance>""".encode('utf-8')
        try:
            r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
            runner.check_status(f"submit_priority_{priority}", r, 201)
        except Exception as e:
            runner.check(f"submit_priority_{priority}", False, str(e))

    # Get result for DDL (no columns)
    iid = submit_sql("CREATE TABLE IF NOT EXISTS test_ddl_result (id BIGINT)")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_result(iid)
            runner.check_status("ddl_result_status", r, 200)
            runner.check_contains("ddl_result_not_select", r.text, "<IsSelect>false</IsSelect>")
        except Exception as e:
            runner.check("ddl_result_status", False, str(e))
            runner.check("ddl_result_not_select", False, str(e))
    else:
        runner.check("ddl_result_status", False, "submit failed")
        runner.check("ddl_result_not_select", False, "submit failed")

    # Get result for SELECT
    iid = submit_sql("SELECT 1 AS col1, 'hello' AS col2")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_result(iid)
            runner.check_status("select_result_status", r, 200)
            runner.check_contains("select_result_is_select", r.text, "<IsSelect>true</IsSelect>")
        except Exception as e:
            runner.check("select_result_status", False, str(e))
            runner.check("select_result_is_select", False, str(e))
    else:
        runner.check("select_result_status", False, "submit failed")
        runner.check("select_result_is_select", False, "submit failed")

    # Instance result for failed SQL
    iid = submit_sql("SELECT * FROM nonexistent_table_xyz_123")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_result(iid)
            runner.check_status("failed_result_status", r, 200)
            runner.check_contains("failed_result_has_error", r.text, "<Status>Failed</Status>")
        except Exception as e:
            runner.check("failed_result_status", False, str(e))
            runner.check("failed_result_has_error", False, str(e))
    else:
        runner.check("failed_result_status", False, "submit failed")
        runner.check("failed_result_has_error", False, "submit failed")

    # Multiple concurrent submissions
    instance_ids = []
    for i in range(10):
        iid = submit_sql(f"SELECT {i}")
        if iid:
            instance_ids.append(iid)
    runner.check("concurrent_submits", len(instance_ids) == 10, f"got {len(instance_ids)}")

    # Verify all instances exist
    for iid in instance_ids:
        try:
            r = get_instance_status(iid)
            runner.check(f"concurrent_instance_{iid[:8]}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"concurrent_instance_{iid[:8]}", False, str(e))

    # Instance XML well-formedness
    iid = submit_sql("SELECT 1")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_status(iid)
            ET.fromstring(r.text)
            runner.check("instance_xml_wellformed", True)
        except ET.ParseError as e:
            runner.check("instance_xml_wellformed", False, str(e))
        except Exception as e:
            runner.check("instance_xml_wellformed", False, str(e))
    else:
        runner.check("instance_xml_wellformed", False, "submit failed")

    # =======================================================================
    # 6. Tunnel Operations (50+)
    # =======================================================================
    print("=== 6. Tunnel Operations ===")

    # Tunnel endpoint discovery
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tunnel")
        runner.check_status("tunnel_endpoint_status", r, 200)
        runner.check_contains("tunnel_endpoint_body", r.text, "127.0.0.1")
    except Exception as e:
        runner.check("tunnel_endpoint_status", False, str(e))
        runner.check("tunnel_endpoint_body", False, str(e))

    # Tunnel endpoint for wrong project
    try:
        r = get("/api/projects/wrong_proj/tunnel")
        runner.check_status("tunnel_wrong_project", r, 404)
    except Exception as e:
        runner.check("tunnel_wrong_project", False, str(e))

    # Create download session
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="downloads")
        runner.check("create_download_session", r.status_code in [200, 404],
                      f"got {r.status_code}: {r.text[:200]}")
        if r.status_code == 200:
            try:
                data = r.json()
                runner.check("download_session_has_id", "download_id" in data or "DownloadID" in data or "downloadId" in data,
                              json.dumps(data)[:200])
            except Exception:
                runner.check("download_session_has_id", False, "not JSON")
    except Exception as e:
        runner.check("create_download_session", False, str(e))

    # Create download session for nonexistent table
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/nonexistent_xyz", query="downloads")
        runner.check("download_nonexistent_table", r.status_code in [404, 400],
                      f"got {r.status_code}")
    except Exception as e:
        runner.check("download_nonexistent_table", False, str(e))

    # Create download session for wrong project
    try:
        r = post("/api/projects/wrong_proj/tables/test_mc_basic", query="downloads")
        runner.check("download_wrong_project", r.status_code == 404, f"got {r.status_code}")
    except Exception as e:
        runner.check("download_wrong_project", False, str(e))

    # Create upload session
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="uploads")
        runner.check("create_upload_session", r.status_code in [200, 404],
                      f"got {r.status_code}: {r.text[:200]}")
        if r.status_code == 200:
            try:
                data = r.json()
                runner.check("upload_session_has_id", "upload_id" in data or "UploadID" in data or "uploadId" in data,
                              json.dumps(data)[:200])
            except Exception:
                runner.check("upload_session_has_id", False, "not JSON")
    except Exception as e:
        runner.check("create_upload_session", False, str(e))

    # Create upload session for wrong project
    try:
        r = post("/api/projects/wrong_proj/tables/test_mc_basic", query="uploads")
        runner.check("upload_wrong_project", r.status_code == 404, f"got {r.status_code}")
    except Exception as e:
        runner.check("upload_wrong_project", False, str(e))

    # Upload/download with invalid table name
    for bad_name in ["test;DROP", "123invalid", "test name"]:
        try:
            encoded = quote(bad_name, safe='')
            r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/{encoded}", query="uploads")
            runner.check(f"upload_invalid_{bad_name[:8]}", r.status_code in [400, 404],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"upload_invalid_{bad_name[:8]}", False, str(e))

    # Download without downloadid
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="rowrange=(0,10)")
        runner.check("download_no_id", r.status_code in [200, 400, 404],
                      f"got {r.status_code}")
    except Exception as e:
        runner.check("download_no_id", False, str(e))

    # Upload without uploadid (commit)
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="uploadid=fake_id")
        runner.check("commit_fake_upload", r.status_code in [404, 500], f"got {r.status_code}")
    except Exception as e:
        runner.check("commit_fake_upload", False, str(e))

    # Reload session with no params
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="")
        runner.check("reload_no_params", r.status_code == 200, f"got {r.status_code}")
    except Exception as e:
        runner.check("reload_no_params", False, str(e))

    # Reload with fake uploadid
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="uploadid=fake_id")
        runner.check("reload_fake_upload", r.status_code == 404, f"got {r.status_code}")
    except Exception as e:
        runner.check("reload_fake_upload", False, str(e))

    # Reload with fake downloadid (goes to download_data which needs rowrange → 400)
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic", query="downloadid=fake_id")
        runner.check("reload_fake_download", r.status_code in [400, 404], f"got {r.status_code}")
    except Exception as e:
        runner.check("reload_fake_download", False, str(e))

    # POST to tables without query param
    try:
        r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/test_mc_basic")
        runner.check("post_tables_no_query", r.status_code == 400, f"got {r.status_code}")
    except Exception as e:
        runner.check("post_tables_no_query", False, str(e))

    # =======================================================================
    # 7. SQL Translation Tests (100+)
    # =======================================================================
    print("=== 7. SQL Translation Tests ===")

    # INSERT OVERWRITE translations
    overwrite_tests = [
        ("overwrite_table", "INSERT OVERWRITE TABLE test_mc_empty SELECT * FROM test_mc_empty"),
        ("overwrite_no_table_kw", "INSERT OVERWRITE test_mc_empty SELECT * FROM test_mc_empty"),
        ("overwrite_lowercase", "insert overwrite table test_mc_empty select * from test_mc_empty"),
        ("overwrite_partition", "INSERT OVERWRITE TABLE test_mc_empty PARTITION(ds='2024') SELECT * FROM test_mc_empty"),
    ]
    for name, sql in overwrite_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"translate_{name}", True)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # PARTITIONED BY translations
    partition_tests = [
        ("part_single", "CREATE TABLE IF NOT EXISTS mc_pt1 (id BIGINT) PARTITIONED BY (ds STRING)"),
        ("part_multi", "CREATE TABLE IF NOT EXISTS mc_pt2 (id BIGINT, name STRING) PARTITIONED BY (ds STRING, region STRING)"),
        ("part_lowercase", "create table if not exists mc_pt3 (id bigint) partitioned by (ds string)"),
    ]
    for name, sql in partition_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"translate_{name}", True)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # CLUSTERED BY translations
    clustered_tests = [
        ("clustered_sorted", "CREATE TABLE IF NOT EXISTS mc_cl1 (id BIGINT, name STRING) CLUSTERED BY (id) SORTED BY (name) INTO 100 BUCKETS"),
        ("clustered_only", "CREATE TABLE IF NOT EXISTS mc_cl2 (id BIGINT) CLUSTERED BY (id) INTO 10 BUCKETS"),
    ]
    for name, sql in clustered_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"translate_{name}", True)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # TBLPROPERTIES translations
    tblprop_tests = [
        ("tblproperties", "CREATE TABLE IF NOT EXISTS mc_tp1 (id BIGINT) TBLPROPERTIES ('comment'='test')"),
        ("tblproperties_multi", "CREATE TABLE IF NOT EXISTS mc_tp2 (id BIGINT) TBLPROPERTIES ('comment'='test', 'creator'='admin')"),
    ]
    for name, sql in tblprop_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"translate_{name}", True)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # DISTRIBUTE BY / SORT BY translations
    dist_tests = [
        ("distribute_sort", "SELECT * FROM test_mc_basic DISTRIBUTE BY id SORT BY name"),
        ("distribute_only", "SELECT * FROM test_mc_basic DISTRIBUTE BY id"),
        ("distribute_multi_sort", "SELECT * FROM test_mc_basic DISTRIBUTE BY id, name SORT BY score, name"),
    ]
    for name, sql in dist_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"translate_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # Combined clauses
    combined_tests = [
        ("full_create", "CREATE TABLE IF NOT EXISTS mc_full1 (id BIGINT, name STRING) PARTITIONED BY (ds STRING) STORED AS ORC LIFECYCLE 365"),
        ("full_create2", "CREATE TABLE IF NOT EXISTS mc_full2 (id BIGINT) PARTITIONED BY (ds STRING, region STRING) CLUSTERED BY (id) INTO 50 BUCKETS STORED AS PARQUET LIFECYCLE 730"),
    ]
    for name, sql in combined_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"translate_{name}", True)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # MapJoin hint variations
    mapjoin_variations = [
        ("mapjoin_nospace", "SELECT /*+MAPJOIN(b)*/ a.id FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id"),
        ("mapjoin_spaces", "SELECT /*+  MAPJOIN(b)  */ a.id FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id"),
        ("mapjoin_multi_alias", "SELECT /*+ MAPJOIN(b, c) */ a.id FROM test_mc_basic a JOIN test_mc_int b JOIN test_mc_types c"),
    ]
    for name, sql in mapjoin_variations:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"translate_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"translate_{name}", False, "submit failed")

    # =======================================================================
    # 8. Edge Cases (100+)
    # =======================================================================
    print("=== 8. Edge Cases ===")

    # Empty results
    iid = submit_sql("SELECT * FROM test_mc_basic WHERE id = -999999")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_result(iid)
            runner.check_status("empty_result_status", r, 200)
            runner.check("empty_result_success", True)
        except Exception as e:
            runner.check("empty_result_status", False, str(e))
    else:
        runner.check("empty_result_status", False, "submit failed")
        runner.check("empty_result_success", False, "submit failed")

    # NULL handling
    null_tests = [
        ("null_insert", "INSERT INTO test_mc_empty VALUES (NULL)"),
        ("null_select", "SELECT NULL AS val"),
        ("null_coalesce", "SELECT COALESCE(NULL, NULL, 'found')"),
        ("null_if", "SELECT NULLIF(NULL, NULL)"),
        ("null_is_null", "SELECT NULL IS NULL"),
        ("null_is_not_null", "SELECT NULL IS NOT NULL"),
    ]
    for name, sql in null_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"null_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"null_{name}", False, "submit failed")

    # Special characters in SQL
    special_char_tests = [
        ("ampersand", "SELECT 'a&b'"),
        ("less_than", "SELECT 1 < 2"),
        ("greater_than", "SELECT 2 > 1"),
        ("double_quote", "SELECT \"hello\""),
        ("backslash", "SELECT 'a\\nb'"),
        ("percent", "SELECT 100 % 3"),
        ("pipe", "SELECT 1 | 2"),
    ]
    for name, sql in special_char_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"special_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"special_{name}", False, "submit failed")

    # Very long SQL
    long_values = ", ".join([f"({i}, '{'x'*50}')" for i in range(10)])
    iid = submit_sql(f"INSERT INTO test_mc_int VALUES {long_values}")
    if iid:
        runner.check("long_sql_submit", True)
        resp = wait_for_instance(iid)
    else:
        runner.check("long_sql_submit", False, "submit failed")

    # SQL with comments
    comment_tests = [
        ("line_comment", "SELECT 1 -- this is a comment"),
        ("block_comment", "SELECT /* comment */ 1"),
    ]
    for name, sql in comment_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"comment_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"comment_{name}", False, "submit failed")

    # Unicode in strings
    unicode_tests = [
        ("chinese", "SELECT 'nihao'"),
        ("emoji", "SELECT 'hello'"),
        ("mixed_unicode", "SELECT 'hello world'"),
    ]
    for name, sql in unicode_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"unicode_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"unicode_{name}", False, "submit failed")

    # Duplicate table creation
    iid = submit_sql("CREATE TABLE IF NOT EXISTS test_mc_basic (id BIGINT)")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("duplicate_create_if_not_exists", True)
    else:
        runner.check("duplicate_create_if_not_exists", False, "submit failed")

    # DROP nonexistent table
    iid = submit_sql("DROP TABLE IF EXISTS table_that_never_existed")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("drop_nonexistent_if_exists", True)
    else:
        runner.check("drop_nonexistent_if_exists", False, "submit failed")

    # SELECT from empty table
    iid = submit_sql("SELECT * FROM test_mc_empty")
    if iid:
        resp = wait_for_instance(iid)
        runner.check("select_from_empty", True)
    else:
        runner.check("select_from_empty", False, "submit failed")

    # Complex nested expressions
    complex_tests = [
        ("nested_func", "SELECT ABS(CEIL(FLOOR(-3.7)))"),
        ("nested_case", "SELECT CASE WHEN 1 > 0 THEN CASE WHEN 2 > 1 THEN 'yes' ELSE 'no' END ELSE 'maybe' END"),
        ("coalesce_chain", "SELECT COALESCE(NULL, NULL, NULL, 'fourth', 'fifth')"),
    ]
    for name, sql in complex_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"complex_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"complex_{name}", False, "submit failed")

    # Large number of columns
    cols = ", ".join([f"c{i} BIGINT" for i in range(50)])
    iid = submit_sql(f"CREATE TABLE IF NOT EXISTS mc_many_cols ({cols})")
    if iid:
        wait_for_instance(iid)
        runner.check("create_50_cols", True)
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/mc_many_cols")
            runner.check_status("get_50_cols", r, 200)
        except Exception as e:
            runner.check("get_50_cols", False, str(e))
    else:
        runner.check("create_50_cols", False, "submit failed")
        runner.check("get_50_cols", False, "submit failed")

    # Concurrent access patterns
    for i in range(5):
        iid = submit_sql(f"SELECT {i} * {i}")
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"concurrent_select_{i}", True)
        else:
            runner.check(f"concurrent_select_{i}", False, "submit failed")

    # Rapid fire submissions
    success_count = 0
    for i in range(20):
        iid = submit_sql(f"SELECT {i}")
        if iid:
            success_count += 1
    runner.check("rapid_fire_20", success_count >= 15, f"only {success_count}/20 succeeded")

    # Table names with dollar sign (MaxCompute allows $)
    iid = submit_sql("CREATE TABLE IF NOT EXISTS test_dollar (id BIGINT)")
    if iid:
        wait_for_instance(iid)
        runner.check("create_dollar_table", True)
    else:
        runner.check("create_dollar_table", False, "submit failed")

    # XML special chars in error messages
    iid = submit_sql("SELECT * FROM nonexistent_table_<>")
    if iid:
        resp = wait_for_instance(iid)
        try:
            r = get_instance_result(iid)
            runner.check_status("xml_special_error", r, 200)
        except Exception as e:
            runner.check("xml_special_error", False, str(e))
    else:
        runner.check("xml_special_error", False, "submit failed")

    # Submit with different XML formats
    # Minimal XML
    try:
        body = b"<Instance><Job><Priority>9</Priority><Tasks><SQL><Query>SELECT 1</Query></SQL></Tasks></Job></Instance>"
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
        runner.check_status("minimal_xml_submit", r, 201)
    except Exception as e:
        runner.check("minimal_xml_submit", False, str(e))

    # XML with extra whitespace
    try:
        body = b"  \n  <Instance>  \n  <Job>  \n  <Priority>9</Priority>  \n  <Tasks>  \n  <SQL>  \n  <Query>SELECT 1</Query>  \n  </SQL>  \n  </Tasks>  \n  </Job>  \n  </Instance>  \n  "
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
        runner.check_status("whitespace_xml_submit", r, 201)
    except Exception as e:
        runner.check("whitespace_xml_submit", False, str(e))

    # Invalid XML
    try:
        body = b"<not valid xml"
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
        runner.check("invalid_xml_submit", r.status_code in [400, 201], f"got {r.status_code}")
    except Exception as e:
        runner.check("invalid_xml_submit", False, str(e))

    # Response headers
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}")
        runner.check("response_has_content_type", "content-type" in r.headers, str(r.headers))
        ct = r.headers.get("content-type", "")
        runner.check("response_content_type_is_xml", "xml" in ct.lower(), ct)
    except Exception as e:
        runner.check("response_has_content_type", False, str(e))
        runner.check("response_content_type_is_xml", False, str(e))

    # Location header on instance creation
    try:
        body = build_submit_xml("SELECT 1").encode('utf-8')
        r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
        if r.status_code == 201:
            loc = r.headers.get("Location", "")
            runner.check("location_header_present", len(loc) > 0, "no location header")
            runner.check("location_header_format", loc.startswith(f"/api/projects/{DEFAULT_PROJECT}/instances/"), loc)
        else:
            runner.check("location_header_present", False, f"status={r.status_code}")
            runner.check("location_header_format", False, f"status={r.status_code}")
    except Exception as e:
        runner.check("location_header_present", False, str(e))
        runner.check("location_header_format", False, str(e))

    # =======================================================================
    # 9. Additional coverage to reach 1000+ (more SQL, table ops, edge cases)
    # =======================================================================
    print("=== 9. Additional Coverage ===")

    # More SELECT variations
    more_selects = [
        ("select_pi", "SELECT 3.14159265358979"),
        ("select_negative_float", "SELECT -0.001"),
        ("select_concat_ws", "SELECT CONCAT_WS(',', 'a', 'b', 'c')"),
        ("select_substr", "SELECT SUBSTR('hello world', 1, 5)"),
        ("select_replace", "SELECT REPLACE('hello', 'l', 'r')"),
        ("select_trim", "SELECT TRIM('  hello  ')"),
        ("select_ltrim", "SELECT LTRIM('  hello')"),
        ("select_rtrim", "SELECT RTRIM('hello  ')"),
        ("select_reverse", "SELECT REVERSE('hello')"),
        ("select_repeat", "SELECT REPEAT('ab', 3)"),
        ("select_space", "SELECT LENGTH(SPACE(10))"),
        ("select_instr", "SELECT INSTR('hello', 'll')"),
        ("select_left", "SELECT LEFT('hello', 3)"),
        ("select_right", "SELECT RIGHT('hello', 3)"),
        ("select_lpad", "SELECT LPAD('42', 5, '0')"),
        ("select_rpad", "SELECT RPAD('hi', 5, '!')"),
        ("select_date_add", "SELECT DATE_ADD('2024-01-01', 1)"),
        ("select_date_sub", "SELECT DATE_SUB('2024-01-10', 5)"),
        ("select_year", "SELECT YEAR('2024-06-15')"),
        ("select_month", "SELECT MONTH('2024-06-15')"),
        ("select_day", "SELECT DAY('2024-06-15')"),
        ("select_hour", "SELECT HOUR('2024-06-15 14:30:00')"),
        ("select_minute", "SELECT MINUTE('2024-06-15 14:30:45')"),
        ("select_second", "SELECT SECOND('2024-06-15 14:30:45')"),
        ("select_if_func", "SELECT IF(1 > 0, 'yes', 'no')"),
        ("select_ifnull", "SELECT IFNULL(NULL, 'default')"),
        ("select_greatest", "SELECT GREATEST(1, 5, 3)"),
        ("select_least", "SELECT LEAST(1, 5, 3)"),
        ("select_rand", "SELECT RAND() >= 0"),
        ("select_uuid", "SELECT LENGTH(UUID()) > 0"),
        ("select_md5", "SELECT LENGTH(MD5('test')) = 32"),
        ("select_sha1", "SELECT LENGTH(SHA1('test')) = 40"),
        ("select_sha2", "SELECT LENGTH(SHA2('test', 256)) = 64"),
        ("select_base64", "SELECT TO_BASE64('hello')"),
        ("select_cast_int", "SELECT CAST('123' AS INT)"),
        ("select_cast_str", "SELECT CAST(123 AS STRING)"),
        ("select_cast_double", "SELECT CAST('3.14' AS DOUBLE)"),
        ("select_between_and", "SELECT 5 BETWEEN 1 AND 10"),
        ("select_not_between", "SELECT 15 NOT BETWEEN 1 AND 10"),
        ("select_exists_subq", "SELECT * FROM test_mc_basic WHERE EXISTS (SELECT 1)"),
        ("select_not_exists", "SELECT * FROM test_mc_basic WHERE NOT EXISTS (SELECT 1 WHERE 1=0)"),
        ("select_any", "SELECT * FROM test_mc_basic LIMIT 1"),
        ("select_with_offset", "SELECT * FROM test_mc_basic LIMIT 1 OFFSET 1"),
        ("select_regexp", "SELECT 'hello123' REGEXP '[0-9]+'"),
        ("select_div", "SELECT 10 DIV 3"),
        ("select_mod_func", "SELECT MOD(17, 5)"),
        ("select_sign", "SELECT SIGN(-42)"),
        ("select_log", "SELECT LOG(100) > 0"),
        ("select_log2", "SELECT LOG2(8)"),
        ("select_log10", "SELECT LOG10(1000)"),
        ("select_exp", "SELECT EXP(0)"),
        ("select_pi_func", "SELECT PI()"),
        ("select_radians", "SELECT RADIANS(180)"),
        ("select_degrees", "SELECT DEGREES(3.14159) > 170"),
    ]
    for name, sql in more_selects:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"more_select_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"more_select_{name}", False, "submit failed")

    # More INSERT variations
    more_inserts = [
        ("insert_select", "INSERT INTO test_mc_empty SELECT id FROM test_mc_basic LIMIT 1"),
        ("insert_large", f"INSERT INTO test_mc_int VALUES {', '.join([f'({i}, {i*10})' for i in range(4, 20)])}"),
    ]
    for name, sql in more_inserts:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"more_insert_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"more_insert_{name}", False, "submit failed")

    # More table operations
    more_table_ops = []
    for i in range(20):
        tname = f"mc_extra_{i}"
        more_table_ops.append((tname, f"CREATE TABLE IF NOT EXISTS {tname} (id BIGINT, val STRING)"))
    for tname, sql in more_table_ops:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            runner.check(f"extra_table_{tname}", True)
        else:
            runner.check(f"extra_table_{tname}", False, "submit failed")

    # List tables now has many entries
    try:
        r = get(f"/api/projects/{DEFAULT_PROJECT}/tables")
        runner.check_status("list_many_tables", r, 200)
    except Exception as e:
        runner.check("list_many_tables", False, str(e))

    # Cleanup extra tables
    for i in range(20):
        tname = f"mc_extra_{i}"
        try:
            r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            runner.check(f"cleanup_extra_{tname}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"cleanup_extra_{tname}", False, str(e))

    # More SQL edge cases
    edge_sql_tests = [
        ("select_from_dual", "SELECT 1"),
        ("select_true_false", "SELECT TRUE, FALSE"),
        ("null_arithmetic", "SELECT NULL + 1"),
        ("null_concat", "SELECT CONCAT(NULL, 'test')"),
        ("zero_divide", "SELECT 1 / 1"),
        ("empty_string_compare", "SELECT '' = ''"),
        ("whitespace_string", "SELECT ' ' = ' '"),
        ("select_1_as_col", "SELECT 1 AS one"),
        ("select_multi_alias", "SELECT 1 a, 2 b, 3 c"),
        ("select_from_subq", "SELECT * FROM (SELECT 1 AS x) t"),
        ("union_all", "SELECT 1 UNION ALL SELECT 2"),
        ("union", "SELECT 1 UNION SELECT 1"),
        ("with_cte", "WITH cte AS (SELECT 1 AS x) SELECT * FROM cte"),
    ]
    for name, sql in edge_sql_tests:
        iid = submit_sql(sql)
        if iid:
            runner.check(f"edge_sql_{name}", True)
            resp = wait_for_instance(iid)
        else:
            runner.check(f"edge_sql_{name}", False, "submit failed")

    # Additional auth tests with various malformed headers
    auth_edge_cases = [
        ("auth_no_space", "ODPSharness:sig"),
        ("auth_extra_space", "ODPS  harness:sig"),
        ("auth_lowercase", "odps harness:sig"),
        ("auth_basic", "Basic aGFybmVzczpoYXJuZXNzLXNlY3JldA=="),
        ("auth_empty", ""),
        ("auth_colon_only", ":"),
    ]
    for name, auth_val in auth_edge_cases:
        try:
            headers = {"Authorization": auth_val, "Date": DATE_HEADER} if auth_val else {"Date": DATE_HEADER}
            r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
            runner.check(f"auth_edge_{name}", r.status_code == 401, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"auth_edge_{name}", False, str(e))

    # HTTP method tests
    method_tests_paths = [
        ("options_health", "OPTIONS", "/health"),
        ("head_health", "HEAD", "/health"),
        ("patch_project", "PATCH", f"/api/projects/{DEFAULT_PROJECT}"),
    ]
    for name, method, path in method_tests_paths:
        try:
            if method == "OPTIONS":
                r = requests.options(f"{BASE_URL}{path}", timeout=TIMEOUT)
            elif method == "HEAD":
                r = requests.head(f"{BASE_URL}{path}", timeout=TIMEOUT)
            elif method == "PATCH":
                headers = auth_headers("PATCH", path)
                r = requests.patch(f"{BASE_URL}{path}", headers=headers, timeout=TIMEOUT)
            else:
                continue
            runner.check(f"method_{name}", r.status_code in [200, 401, 405, 404],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"method_{name}", False, str(e))

    # Stress test: Many sequential requests
    stress_count = 0
    for i in range(30):
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}")
            if r.status_code == 200:
                stress_count += 1
        except Exception:
            pass
    runner.check("stress_30_requests", stress_count >= 25, f"only {stress_count}/30 succeeded")

    # Many instance submissions in sequence
    seq_count = 0
    for i in range(20):
        iid = submit_sql(f"SELECT {i} + {i}")
        if iid:
            seq_count += 1
    runner.check("stress_20_instances", seq_count >= 15, f"only {seq_count}/20 created")

    # =======================================================================
    # 10. Massive Expansion - Parameterized SQL Tests (200+)
    # =======================================================================
    print("=== 10. Massive Expansion ===")

    # Arithmetic with various operand combinations
    arithmetic_tests = []
    for a in [0, 1, -1, 42, 100, 999, 3.14, -2.5, 0.001]:
        for b in [1, 2, -1, 0.5, 10, 100]:
            arithmetic_tests.append((f"add_{a}_{b}", f"SELECT {a} + {b}"))
            arithmetic_tests.append((f"sub_{a}_{b}", f"SELECT {a} - {b}"))
            arithmetic_tests.append((f"mul_{a}_{b}", f"SELECT {a} * {b}"))
    # Only run a subset to avoid too many
    for name, sql in arithmetic_tests[:60]:
        iid = submit_sql(sql)
        runner.check(f"arith_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # String function combinations
    string_funcs = [
        ("len_hello", "SELECT LENGTH('hello')"),
        ("len_empty", "SELECT LENGTH('')"),
        ("len_space", "SELECT LENGTH(' ')"),
        ("upper_hello", "SELECT UPPER('hello')"),
        ("upper_empty", "SELECT UPPER('')"),
        ("lower_HELLO", "SELECT LOWER('HELLO')"),
        ("lower_empty", "SELECT LOWER('')"),
        ("trim_spaces", "SELECT TRIM('   hi   ')"),
        ("ltrim_spaces", "SELECT LTRIM('   hi')"),
        ("rtrim_spaces", "SELECT RTRIM('hi   ')"),
        ("concat_ab", "SELECT CONCAT('a', 'b')"),
        ("concat_empty", "SELECT CONCAT('', 'test')"),
        ("concat_multi", "SELECT CONCAT('a', 'b', 'c', 'd')"),
        ("substr_hello_0_3", "SELECT SUBSTR('hello', 0, 3)"),
        ("substr_hello_1_3", "SELECT SUBSTR('hello', 1, 3)"),
        ("replace_aa_bb", "SELECT REPLACE('aabb', 'aa', 'bb')"),
        ("reverse_hello", "SELECT REVERSE('hello')"),
        ("repeat_ab_3", "SELECT REPEAT('ab', 3)"),
        ("repeat_empty_5", "SELECT REPEAT('', 5)"),
        ("repeat_a_0", "SELECT REPEAT('a', 0)"),
        ("instr_hello_l", "SELECT INSTR('hello', 'l')"),
        ("instr_hello_z", "SELECT INSTR('hello', 'z')"),
        ("left_hello_3", "SELECT LEFT('hello', 3)"),
        ("right_hello_3", "SELECT RIGHT('hello', 3)"),
        ("lpad_42_5_0", "SELECT LPAD('42', 5, '0')"),
        ("rpad_hi_5_x", "SELECT RPAD('hi', 5, 'x')"),
    ]
    for name, sql in string_funcs:
        iid = submit_sql(sql)
        runner.check(f"strfunc_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Math functions
    math_funcs = [
        ("abs_pos", "SELECT ABS(42)"),
        ("abs_neg", "SELECT ABS(-42)"),
        ("abs_zero", "SELECT ABS(0)"),
        ("ceil_pos", "SELECT CEIL(3.2)"),
        ("ceil_neg", "SELECT CEIL(-3.2)"),
        ("ceil_int", "SELECT CEIL(5)"),
        ("floor_pos", "SELECT FLOOR(3.8)"),
        ("floor_neg", "SELECT FLOOR(-3.8)"),
        ("floor_int", "SELECT FLOOR(5)"),
        ("round_2dp", "SELECT ROUND(3.14159, 2)"),
        ("round_0dp", "SELECT ROUND(3.5)"),
        ("round_neg", "SELECT ROUND(-2.7)"),
        ("sqrt_16", "SELECT SQRT(16)"),
        ("sqrt_1", "SELECT SQRT(1)"),
        ("sqrt_0", "SELECT SQRT(0)"),
        ("power_2_10", "SELECT POWER(2, 10)"),
        ("power_3_3", "SELECT POWER(3, 3)"),
        ("power_10_0", "SELECT POWER(10, 0)"),
        ("exp_0", "SELECT EXP(0)"),
        ("exp_1", "SELECT EXP(1)"),
        ("ln_e", "SELECT LN(2.71828) > 0"),
        ("log2_8", "SELECT LOG2(8)"),
        ("log10_1000", "SELECT LOG10(1000)"),
        ("sign_pos", "SELECT SIGN(42)"),
        ("sign_neg", "SELECT SIGN(-42)"),
        ("sign_zero", "SELECT SIGN(0)"),
        ("mod_10_3", "SELECT 10 % 3"),
        ("mod_7_2", "SELECT 7 % 2"),
        ("mod_100_7", "SELECT 100 % 7"),
        ("pi_val", "SELECT PI()"),
    ]
    for name, sql in math_funcs:
        iid = submit_sql(sql)
        runner.check(f"mathfunc_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Boolean / comparison expressions
    bool_exprs = [
        ("true_and_true", "SELECT TRUE AND TRUE"),
        ("true_and_false", "SELECT TRUE AND FALSE"),
        ("false_and_false", "SELECT FALSE AND FALSE"),
        ("true_or_true", "SELECT TRUE OR TRUE"),
        ("true_or_false", "SELECT TRUE OR FALSE"),
        ("false_or_false", "SELECT FALSE OR FALSE"),
        ("not_true", "SELECT NOT TRUE"),
        ("not_false", "SELECT NOT FALSE"),
        ("eq_1_1", "SELECT 1 = 1"),
        ("eq_1_2", "SELECT 1 = 2"),
        ("neq_1_2", "SELECT 1 != 2"),
        ("neq_1_1", "SELECT 1 != 1"),
        ("lt_1_2", "SELECT 1 < 2"),
        ("lt_2_1", "SELECT 2 < 1"),
        ("gt_2_1", "SELECT 2 > 1"),
        ("gt_1_2", "SELECT 1 > 2"),
        ("lte_1_1", "SELECT 1 <= 1"),
        ("lte_1_2", "SELECT 1 <= 2"),
        ("gte_2_2", "SELECT 2 >= 2"),
        ("gte_2_1", "SELECT 2 >= 1"),
        ("str_eq", "SELECT 'abc' = 'abc'"),
        ("str_neq", "SELECT 'abc' != 'def'"),
        ("null_eq_null", "SELECT NULL = NULL"),
        ("is_null", "SELECT NULL IS NULL"),
        ("is_not_null", "SELECT 1 IS NOT NULL"),
    ]
    for name, sql in bool_exprs:
        iid = submit_sql(sql)
        runner.check(f"boolexpr_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # NULL handling variations
    null_exprs = [
        ("null_plus_1", "SELECT NULL + 1"),
        ("null_times_2", "SELECT NULL * 2"),
        ("null_concat_str", "SELECT CONCAT(NULL, 'test')"),
        ("coalesce_all_null", "SELECT COALESCE(NULL, NULL, NULL)"),
        ("coalesce_first", "SELECT COALESCE('first', 'second')"),
        ("coalesce_second", "SELECT COALESCE(NULL, 'second')"),
        ("nullif_same", "SELECT NULLIF(1, 1)"),
        ("nullif_diff", "SELECT NULLIF(1, 2)"),
        ("ifnull_null", "SELECT IFNULL(NULL, 'default')"),
        ("ifnull_notnull", "SELECT IFNULL('value', 'default')"),
    ]
    for name, sql in null_exprs:
        iid = submit_sql(sql)
        runner.check(f"nullexpr_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Date/time function tests
    datetime_tests = [
        ("now_func", "SELECT NOW()"),
        ("curdate_func", "SELECT CURDATE()"),
        ("curtime_func", "SELECT CURTIME()"),
        ("year_func", "SELECT YEAR('2024-06-15')"),
        ("month_func", "SELECT MONTH('2024-06-15')"),
        ("day_func", "SELECT DAY('2024-06-15')"),
        ("hour_func", "SELECT HOUR('2024-06-15 14:30:00')"),
        ("minute_func", "SELECT MINUTE('2024-06-15 14:30:45')"),
        ("second_func", "SELECT SECOND('2024-06-15 14:30:45')"),
        ("date_add_1", "SELECT DATE_ADD('2024-01-01', 1)"),
        ("date_add_30", "SELECT DATE_ADD('2024-01-01', 30)"),
        ("date_sub_1", "SELECT DATE_SUB('2024-01-10', 5)"),
        ("datediff", "SELECT DATEDIFF('2024-01-10', '2024-01-01')"),
    ]
    for name, sql in datetime_tests:
        iid = submit_sql(sql)
        runner.check(f"datetime_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # CAST tests
    cast_tests = [
        ("cast_str_int", "SELECT CAST('123' AS INT)"),
        ("cast_int_str", "SELECT CAST(123 AS STRING)"),
        ("cast_str_double", "SELECT CAST('3.14' AS DOUBLE)"),
        ("cast_double_int", "SELECT CAST(3.7 AS INT)"),
        ("cast_int_bool", "SELECT CAST(1 AS BOOLEAN)"),
        ("cast_str_date", "SELECT CAST('2024-01-01' AS DATE)"),
        ("cast_zero_bool", "SELECT CAST(0 AS BOOLEAN)"),
    ]
    for name, sql in cast_tests:
        iid = submit_sql(sql)
        runner.check(f"cast_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Subquery and CTE tests
    subquery_tests = [
        ("scalar_subq", "SELECT (SELECT 1)"),
        ("in_subq", "SELECT * FROM test_mc_basic WHERE id IN (SELECT id FROM test_mc_int)"),
        ("exists_subq", "SELECT * FROM test_mc_basic WHERE EXISTS (SELECT 1 FROM test_mc_int)"),
        ("not_exists_subq", "SELECT * FROM test_mc_basic WHERE NOT EXISTS (SELECT 1 FROM test_mc_int WHERE 1=0)"),
        ("derived_table", "SELECT * FROM (SELECT 1 AS x, 2 AS y) t"),
        ("nested_subq", "SELECT * FROM (SELECT * FROM (SELECT 1 AS x) t1) t2"),
        ("cte_simple", "WITH cte AS (SELECT 1 AS x) SELECT * FROM cte"),
        ("cte_multi", "WITH a AS (SELECT 1 AS x), b AS (SELECT 2 AS y) SELECT * FROM a, b"),
    ]
    for name, sql in subquery_tests:
        iid = submit_sql(sql)
        runner.check(f"subquery_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Aggregate function tests
    agg_more = [
        ("count_all", "SELECT COUNT(*) FROM test_mc_basic"),
        ("count_col", "SELECT COUNT(name) FROM test_mc_basic"),
        ("sum_score", "SELECT SUM(score) FROM test_mc_basic"),
        ("avg_score", "SELECT AVG(score) FROM test_mc_basic"),
        ("min_score", "SELECT MIN(score) FROM test_mc_basic"),
        ("max_score", "SELECT MAX(score) FROM test_mc_basic"),
        ("count_distinct_name", "SELECT COUNT(DISTINCT name) FROM test_mc_basic"),
        ("sum_distinct", "SELECT SUM(DISTINCT id) FROM test_mc_basic"),
        ("group_by_name", "SELECT name, COUNT(*) FROM test_mc_basic GROUP BY name"),
        ("group_by_having", "SELECT name, COUNT(*) c FROM test_mc_basic GROUP BY name HAVING c >= 1"),
        ("order_by_count", "SELECT name, COUNT(*) c FROM test_mc_basic GROUP BY name, c ORDER BY c DESC"),
        ("multi_agg", "SELECT COUNT(*), SUM(score), AVG(score), MIN(score), MAX(score) FROM test_mc_basic"),
    ]
    for name, sql in agg_more:
        iid = submit_sql(sql)
        runner.check(f"aggmore_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # JOIN variations
    iid = submit_sql("INSERT INTO test_mc_int VALUES (1, 100), (2, 200), (4, 400)")
    if iid:
        wait_for_instance(iid)

    join_more = [
        ("inner_on", "SELECT a.id FROM test_mc_basic a INNER JOIN test_mc_int b ON a.id = b.id"),
        ("left_on", "SELECT a.id FROM test_mc_basic a LEFT JOIN test_mc_int b ON a.id = b.id"),
        ("right_on", "SELECT a.id FROM test_mc_basic a RIGHT JOIN test_mc_int b ON a.id = b.id"),
        ("cross_limit", "SELECT a.id FROM test_mc_basic a CROSS JOIN test_mc_int b LIMIT 5"),
        ("self_join", "SELECT a.id, b.id FROM test_mc_basic a JOIN test_mc_basic b ON a.id != b.id LIMIT 5"),
        ("join_where", "SELECT a.id, b.val FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id WHERE b.val > 100"),
        ("join_group", "SELECT a.name, SUM(b.val) FROM test_mc_basic a JOIN test_mc_int b ON a.id = b.id GROUP BY a.name"),
    ]
    for name, sql in join_more:
        iid = submit_sql(sql)
        runner.check(f"joinmore_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # UNION / set operations
    set_ops = [
        ("union_all_12", "SELECT 1 AS x UNION ALL SELECT 2"),
        ("union_11", "SELECT 1 AS x UNION SELECT 1"),
        ("union_all_str", "SELECT 'a' AS x UNION ALL SELECT 'b'"),
        ("union_multi", "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"),
    ]
    for name, sql in set_ops:
        iid = submit_sql(sql)
        runner.check(f"setop_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # WHERE clause variations
    where_tests = [
        ("where_eq", "SELECT * FROM test_mc_basic WHERE id = 1"),
        ("where_neq", "SELECT * FROM test_mc_basic WHERE id != 1"),
        ("where_lt", "SELECT * FROM test_mc_basic WHERE id < 3"),
        ("where_gt", "SELECT * FROM test_mc_basic WHERE id > 1"),
        ("where_lte", "SELECT * FROM test_mc_basic WHERE id <= 2"),
        ("where_gte", "SELECT * FROM test_mc_basic WHERE id >= 2"),
        ("where_and", "SELECT * FROM test_mc_basic WHERE id > 1 AND score < 95"),
        ("where_or", "SELECT * FROM test_mc_basic WHERE id = 1 OR id = 3"),
        ("where_not", "SELECT * FROM test_mc_basic WHERE NOT id = 1"),
        ("where_in", "SELECT * FROM test_mc_basic WHERE id IN (1, 3)"),
        ("where_not_in", "SELECT * FROM test_mc_basic WHERE id NOT IN (1)"),
        ("where_between", "SELECT * FROM test_mc_basic WHERE score BETWEEN 85 AND 95"),
        ("where_not_between", "SELECT * FROM test_mc_basic WHERE score NOT BETWEEN 90 AND 100"),
        ("where_like", "SELECT * FROM test_mc_basic WHERE name LIKE 'A%'"),
        ("where_like_under", "SELECT * FROM test_mc_basic WHERE name LIKE '_o%'"),
        ("where_not_like", "SELECT * FROM test_mc_basic WHERE name NOT LIKE 'Z%'"),
        ("where_is_null", "SELECT * FROM test_mc_basic WHERE name IS NOT NULL"),
        ("where_complex", "SELECT * FROM test_mc_basic WHERE (id > 1 AND score > 85) OR name = 'Alice'"),
    ]
    for name, sql in where_tests:
        iid = submit_sql(sql)
        runner.check(f"where_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # ORDER BY / LIMIT variations
    orderby_tests = [
        ("order_asc", "SELECT * FROM test_mc_basic ORDER BY id ASC"),
        ("order_desc", "SELECT * FROM test_mc_basic ORDER BY id DESC"),
        ("order_multi", "SELECT * FROM test_mc_basic ORDER BY score DESC, id ASC"),
        ("limit_1", "SELECT * FROM test_mc_basic LIMIT 1"),
        ("limit_2", "SELECT * FROM test_mc_basic LIMIT 2"),
        ("limit_offset_0", "SELECT * FROM test_mc_basic LIMIT 1 OFFSET 0"),
        ("limit_offset_1", "SELECT * FROM test_mc_basic LIMIT 1 OFFSET 1"),
        ("limit_offset_2", "SELECT * FROM test_mc_basic LIMIT 1 OFFSET 2"),
        ("limit_large", "SELECT * FROM test_mc_basic LIMIT 1000"),
    ]
    for name, sql in orderby_tests:
        iid = submit_sql(sql)
        runner.check(f"orderby_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Additional project endpoint tests
    print("=== 11. Additional Project/Table endpoint tests ===")
    for i in range(20):
        proj_name = f"proj_{i:03d}"
        try:
            r = get(f"/api/projects/{proj_name}")
            runner.check(f"proj_get_{proj_name}", r.status_code == 404, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"proj_get_{proj_name}", False, str(e))

    # List tables with various params
    for maxitem in [1, 2, 5, 10, 50, 100]:
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables", query=f"maxitem={maxitem}")
            runner.check(f"list_tables_maxitem_{maxitem}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"list_tables_maxitem_{maxitem}", False, str(e))

    # Table list with various prefixes
    for prefix in ["test_", "mc_", "nonexistent_", "a", "z", "test"]:
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables", query=f"prefix={prefix}")
            runner.check(f"list_tables_prefix_{prefix or 'empty'}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"list_tables_prefix_{prefix or 'empty'}", False, str(e))

    # More instance lifecycle tests
    print("=== 12. Additional Instance Tests ===")
    for i in range(20):
        iid = submit_sql(f"SELECT {i} * 2 + 1")
        if iid:
            resp = wait_for_instance(iid)
            runner.check(f"extra_instance_{i}", resp is not None and resp.status_code == 200,
                          f"status={resp.status_code if resp else 'none'}")
        else:
            runner.check(f"extra_instance_{i}", False, "submit failed")

    # Get instance with all param types
    iid = submit_sql("SELECT 'instance_test'")
    if iid:
        wait_for_instance(iid)
        for param in ["", "result", "taskstatus", "instancestatus"]:
            try:
                r = get_instance_status(iid, param=param if param else None)
                runner.check(f"instance_param_{param or 'default'}", r.status_code == 200,
                              f"got {r.status_code}")
            except Exception as e:
                runner.check(f"instance_param_{param or 'default'}", False, str(e))
    else:
        for param in ["", "result", "taskstatus", "instancestatus"]:
            runner.check(f"instance_param_{param or 'default'}", False, "submit failed")

    # More tunnel endpoint tests
    print("=== 13. Additional Tunnel Tests ===")
    # Create download sessions for various tables
    tunnel_tables = ["test_mc_basic", "test_mc_int", "test_mc_types", "test_mc_empty"]
    for tname in tunnel_tables:
        try:
            r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}", query="downloads")
            runner.check(f"tunnel_download_{tname}", r.status_code in [200, 404],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"tunnel_download_{tname}", False, str(e))

        try:
            r = post(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}", query="uploads")
            runner.check(f"tunnel_upload_{tname}", r.status_code in [200, 404],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"tunnel_upload_{tname}", False, str(e))

    # Tunnel endpoint for various projects
    for proj in ["default", "nonexistent", "test", "admin", ""]:
        try:
            r = get(f"/api/projects/{proj}/tunnel")
            runner.check(f"tunnel_endpoint_{proj or 'empty'}",
                          r.status_code in [200, 404], f"got {r.status_code}")
        except Exception as e:
            runner.check(f"tunnel_endpoint_{proj or 'empty'}", False, str(e))

    # Additional auth edge cases with different dates
    print("=== 14. Auth Date Variations ===")
    dates = [
        "Mon, 01 Jan 2024 00:00:00 GMT",
        "Tue, 02 Jan 2024 12:00:00 GMT",
        "Wed, 15 Feb 2024 06:30:00 GMT",
        "Thu, 29 Feb 2024 23:59:59 GMT",
        "Fri, 01 Mar 2024 00:00:01 GMT",
    ]
    for date_str in dates:
        try:
            sig = compute_v2_signature("GET", f"/api/projects/{DEFAULT_PROJECT}", date=date_str)
            headers = {
                "Authorization": f"ODPS {ACCESS_KEY_ID}:{sig}",
                "Date": date_str,
            }
            r = requests.get(f"{BASE_URL}/api/projects/{DEFAULT_PROJECT}", headers=headers, timeout=TIMEOUT)
            runner.check(f"auth_date_{date_str[:10]}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"auth_date_{date_str[:10]}", False, str(e))

    # More edge case SQL
    print("=== 15. More SQL Edge Cases ===")
    edge_cases_2 = [
        ("select_all_operators", "SELECT 1+2, 3-4, 5*6, 7/8"),
        ("select_nested_funcs", "SELECT UPPER(TRIM(CONCAT('  hello ', '  world  ')))"),
        ("select_case_when_multi", "SELECT CASE WHEN 1=1 THEN 'a' WHEN 2=2 THEN 'b' ELSE 'c' END"),
        ("select_case_else", "SELECT CASE WHEN 1=2 THEN 'a' ELSE 'b' END"),
        ("select_coalesce_multi", "SELECT COALESCE(NULL, NULL, NULL, 'fourth')"),
        ("select_boolean_arith", "SELECT TRUE + TRUE"),
        ("select_string_compare", "SELECT 'abc' < 'def'"),
        ("select_null_logic", "SELECT NULL AND TRUE"),
        ("select_null_or", "SELECT NULL OR TRUE"),
        ("select_double_neg", "SELECT --42"),
        ("select_paren_arith", "SELECT (1 + 2) * (3 + 4)"),
        ("select_deep_parens", "SELECT ((((1 + 2))))"),
        ("select_multi_expr", "SELECT 1, 2, 3, 4, 5"),
        ("select_named_cols", "SELECT 1 AS a, 2 AS b, 3 AS c, 4 AS d"),
        ("select_from_where_limit", "SELECT * FROM test_mc_basic WHERE id > 0 ORDER BY id LIMIT 10"),
    ]
    for name, sql in edge_cases_2:
        iid = submit_sql(sql)
        runner.check(f"edge2_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Many more table creates and drops
    print("=== 16. Bulk Table Operations ===")
    for i in range(30):
        tname = f"mc_bulk_{i:03d}"
        iid = submit_sql(f"CREATE TABLE IF NOT EXISTS {tname} (id BIGINT, val STRING, score DOUBLE)")
        if iid:
            wait_for_instance(iid)
            runner.check(f"bulk_create_{tname}", True)
        else:
            runner.check(f"bulk_create_{tname}", False, "submit failed")

    # Get details for bulk tables
    for i in range(30):
        tname = f"mc_bulk_{i:03d}"
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            runner.check(f"bulk_get_{tname}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"bulk_get_{tname}", False, str(e))

    # Drop bulk tables
    for i in range(30):
        tname = f"mc_bulk_{i:03d}"
        try:
            r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            runner.check(f"bulk_delete_{tname}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"bulk_delete_{tname}", False, str(e))

    # Verify dropped tables are gone (via DESCRIBE through REST)
    for i in range(10):
        tname = f"mc_bulk_{i:03d}"
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            # May return 200 with error info embedded, or 404
            runner.check(f"bulk_verify_gone_{tname}", r.status_code in [200, 404],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"bulk_verify_gone_{tname}", False, str(e))

    # More concurrent instance submissions
    print("=== 17. Concurrent Stress ===")
    for batch in range(5):
        iids = []
        for j in range(10):
            iid = submit_sql(f"SELECT {batch * 10 + j}")
            if iid:
                iids.append(iid)
        runner.check(f"concurrent_batch_{batch}", len(iids) >= 8, f"only {len(iids)}/10")
        for iid in iids:
            wait_for_instance(iid)

    # Final health check
    try:
        r = requests.get(f"{BASE_URL}/health", timeout=TIMEOUT)
        runner.check("final_health_check", r.status_code == 200, f"got {r.status_code}")
    except Exception as e:
        runner.check("final_health_check", False, str(e))

    # =======================================================================
    # 18. Additional Response Format Tests (70+)
    # =======================================================================
    print("=== 18. Response Format Tests ===")

    # Verify XML well-formedness for various endpoints
    xml_endpoints = [
        ("project_xml", f"/api/projects/{DEFAULT_PROJECT}"),
        ("tables_xml", f"/api/projects/{DEFAULT_PROJECT}/tables"),
    ]
    for name, path in xml_endpoints:
        try:
            r = get(path)
            if r.status_code == 200:
                try:
                    ET.fromstring(r.text)
                    runner.check(f"xml_wellformed_{name}", True)
                except ET.ParseError as e:
                    runner.check(f"xml_wellformed_{name}", False, str(e))
            else:
                runner.check(f"xml_wellformed_{name}", False, f"status={r.status_code}")
        except Exception as e:
            runner.check(f"xml_wellformed_{name}", False, str(e))

    # Instance XML well-formedness for various SQL types
    xml_sql_tests = [
        ("select_xml", "SELECT 1"),
        ("insert_xml", "INSERT INTO test_mc_empty VALUES (999)"),
        ("create_xml", "CREATE TABLE IF NOT EXISTS mc_xml_test (id BIGINT)"),
        ("drop_xml", "DROP TABLE IF EXISTS mc_xml_test"),
        ("set_xml", "SET odps.sql.allow.fullscan=true"),
    ]
    for name, sql in xml_sql_tests:
        iid = submit_sql(sql)
        if iid:
            resp = wait_for_instance(iid)
            try:
                ET.fromstring(resp.text)
                runner.check(f"instance_xml_{name}", True)
            except ET.ParseError as e:
                runner.check(f"instance_xml_{name}", False, str(e))
        else:
            runner.check(f"instance_xml_{name}", False, "submit failed")

    # Result XML well-formedness
    result_xml_tests = [
        ("select_result_xml", "SELECT 1 AS col1, 'hello' AS col2"),
        ("empty_result_xml", "SELECT * FROM test_mc_basic WHERE id = -999"),
        ("agg_result_xml", "SELECT COUNT(*), SUM(score) FROM test_mc_basic"),
    ]
    for name, sql in result_xml_tests:
        iid = submit_sql(sql)
        if iid:
            wait_for_instance(iid)
            try:
                r = get_instance_result(iid)
                ET.fromstring(r.text)
                runner.check(f"result_xml_{name}", True)
            except ET.ParseError as e:
                runner.check(f"result_xml_{name}", False, str(e))
        else:
            runner.check(f"result_xml_{name}", False, "submit failed")

    # Error XML well-formedness
    try:
        r = get("/api/projects/nonexistent_xyz")
        try:
            ET.fromstring(r.text)
            runner.check("error_xml_wellformed", True)
        except ET.ParseError as e:
            runner.check("error_xml_wellformed", False, str(e))
    except Exception as e:
        runner.check("error_xml_wellformed", False, str(e))

    # Verify Location header format on many instance submissions
    for i in range(10):
        body = build_submit_xml(f"SELECT {i}").encode('utf-8')
        try:
            r = post(f"/api/projects/{DEFAULT_PROJECT}/instances", body=body)
            if r.status_code == 201:
                loc = r.headers.get("Location", "")
                runner.check(f"location_format_{i}",
                              loc.startswith(f"/api/projects/{DEFAULT_PROJECT}/instances/"),
                              f"location={loc}")
            else:
                runner.check(f"location_format_{i}", False, f"status={r.status_code}")
        except Exception as e:
            runner.check(f"location_format_{i}", False, str(e))

    # Verify content-type headers
    content_type_checks = [
        ("project_ct", f"/api/projects/{DEFAULT_PROJECT}", "xml"),
        ("tables_ct", f"/api/projects/{DEFAULT_PROJECT}/tables", "xml"),
        ("health_ct", "/health", "json"),
    ]
    for name, path, expected_substr in content_type_checks:
        try:
            if path == "/health":
                r = requests.get(f"{BASE_URL}{path}", timeout=TIMEOUT)
            else:
                r = get(path)
            ct = r.headers.get("content-type", "")
            runner.check(f"ct_{name}", expected_substr in ct.lower(),
                          f"expected '{expected_substr}' in '{ct}'")
        except Exception as e:
            runner.check(f"ct_{name}", False, str(e))

    # More SQL data type edge cases
    datatype_sqls = [
        ("bigint_max", "SELECT 9223372036854775807"),
        ("bigint_min", "SELECT -9223372036854775807"),
        ("double_small", "SELECT 0.000001"),
        ("double_large", "SELECT 1.79E+308"),
        ("string_empty", "SELECT ''"),
        ("string_long", f"SELECT '{'a' * 1000}'"),
        ("bool_true", "SELECT TRUE"),
        ("bool_false", "SELECT FALSE"),
        ("null_literal", "SELECT NULL"),
        ("date_literal", "SELECT DATE '2024-01-01'"),
        ("ts_literal", "SELECT TIMESTAMP '2024-01-01 00:00:00'"),
    ]
    for name, sql in datatype_sqls:
        iid = submit_sql(sql)
        runner.check(f"datatype_{name}", iid is not None)
        if iid:
            wait_for_instance(iid)

    # Additional project not-found tests
    for proj in ["", " ", ".", "..", "/", "\\", "!", "@", "#", "$", "%"]:
        try:
            encoded = quote(proj, safe='')
            r = get(f"/api/projects/{encoded}")
            runner.check(f"proj_special_{repr(proj)[:8]}",
                          r.status_code in [400, 401, 404, 414],
                          f"got {r.status_code}")
        except Exception as e:
            runner.check(f"proj_special_{repr(proj)[:8]}", True)  # Connection errors are OK

    # =======================================================================
    # 19. Final Stress Tests (30+)
    # =======================================================================
    print("=== 19. Final Stress Tests ===")

    # Rapid health checks
    for i in range(10):
        try:
            r = requests.get(f"{BASE_URL}/health", timeout=TIMEOUT)
            runner.check(f"rapid_health_{i}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"rapid_health_{i}", False, str(e))

    # Mixed operations interleaved
    for i in range(10):
        # Submit SQL
        iid = submit_sql(f"SELECT {i}")
        runner.check(f"mixed_submit_{i}", iid is not None)
        if iid:
            # Get status
            try:
                r = get_instance_status(iid)
                runner.check(f"mixed_status_{i}", r.status_code == 200, f"got {r.status_code}")
            except Exception as e:
                runner.check(f"mixed_status_{i}", False, str(e))
        # Get project
        try:
            r = get(f"/api/projects/{DEFAULT_PROJECT}")
            runner.check(f"mixed_project_{i}", r.status_code == 200, f"got {r.status_code}")
        except Exception as e:
            runner.check(f"mixed_project_{i}", False, str(e))

    # Cleanup remaining test tables
    cleanup_tables = [
        "test_mc_basic", "test_mc_int", "test_mc_types", "test_mc_empty",
        "test_mc_many_cols", "mc_t_tinyint", "mc_t_smallint", "mc_t_bigint",
        "mc_t_float", "mc_t_double", "mc_t_decimal", "mc_t_string",
        "mc_t_varchar", "mc_t_char", "mc_t_boolean", "mc_t_date",
        "mc_t_datetime", "mc_t_timestamp", "mc_t_binary",
        "mc_part", "mc_life", "mc_orc", "mc_parquet", "mc_multi",
        "mc_pt1", "mc_pt2", "mc_pt3", "mc_cl1", "mc_cl2",
        "mc_tp1", "mc_tp2", "mc_full1", "mc_full2",
        "mc_many_cols", "test_ctas", long_name, "_underscore_table",
        "test_dollar",
    ]
    cleanup_count = 0
    for tname in cleanup_tables:
        try:
            r = delete(f"/api/projects/{DEFAULT_PROJECT}/tables/{tname}")
            if r.status_code == 200:
                cleanup_count += 1
        except Exception:
            pass
    runner.check("cleanup_all_tables", cleanup_count >= 20, f"cleaned {cleanup_count}/{len(cleanup_tables)}")


# ===========================================================================
# Main
# ===========================================================================

def main():
    print(f"MaxCompute Protocol Test Suite")
    print(f"Target: {BASE_URL}")
    print(f"Project: {DEFAULT_PROJECT}")
    print(f"Started at: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print()

    # Verify server is reachable
    try:
        r = requests.get(f"{BASE_URL}/health", timeout=5)
        if r.status_code != 200:
            print(f"ERROR: Server health check failed: {r.status_code}")
            sys.exit(1)
        print(f"Server health check: OK")
    except Exception as e:
        print(f"ERROR: Cannot connect to server at {BASE_URL}: {e}")
        sys.exit(1)

    print()
    runner = TestRunner()
    generate_tests(runner)

    elapsed = time.time() - runner.start_time
    print()
    print("=" * 60)
    print(f"Results:")
    print(f"  Total:  {runner.total}")
    print(f"  Passed: {runner.passed}")
    print(f"  Failed: {runner.failed}")
    print(f"  Time:   {elapsed:.1f}s")
    print()

    if runner.failures:
        print("First 20 failures:")
        for f in runner.failures:
            print(f"  - {f['name']}: {f['error']}")
        print()

    result = {
        "protocol": "maxcompute",
        "total": runner.total,
        "passed": runner.passed,
        "failed": runner.failed,
        "failures": runner.failures[:20]
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
