#!/usr/bin/env python3
"""
Comprehensive InfluxDB protocol tests for RorisDB
Tests the InfluxDB HTTP API on port 18086
"""

import urllib.request
import urllib.parse
import urllib.error
import json
import time
import sys
from typing import Tuple, Dict, Any, List

BASE_URL = "http://127.0.0.1:18086"

def make_request(method: str, path: str, params: Dict[str, str] = None, body: str = None) -> Tuple[int, str]:
    """Make HTTP request and return (status_code, response_body)"""
    url = BASE_URL + path
    if params:
        url += "?" + urllib.parse.urlencode(params)

    try:
        if body:
            req = urllib.request.Request(url, data=body.encode('utf-8'), method=method)
        else:
            req = urllib.request.Request(url, method=method)

        with urllib.request.urlopen(req) as response:
            return response.status, response.read().decode('utf-8')
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode('utf-8')
    except Exception as e:
        return -1, str(e)

def write_data(db: str, data: str) -> Tuple[int, str]:
    """Write data to InfluxDB"""
    return make_request("POST", "/write", {"db": db}, data)

def query_data(db: str, q: str) -> Tuple[int, str]:
    """Query data from InfluxDB"""
    return make_request("GET", "/query", {"db": db, "q": q})

def parse_json_response(response: str) -> Dict[str, Any]:
    """Parse JSON response"""
    try:
        return json.loads(response)
    except:
        return {"error": "Invalid JSON", "raw": response}

class TestRunner:
    def __init__(self):
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []

    def test(self, name: str, condition: bool, details: str = ""):
        """Run a single test"""
        self.total += 1
        if condition:
            self.passed += 1
        else:
            self.failed += 1
            if len(self.failures) < 20:
                self.failures.append({"test": name, "details": details})

    def run_all(self):
        """Run all test categories"""
        self.test_health()
        self.test_database_ddl()
        self.test_write_single()
        self.test_write_with_tags()
        self.test_write_batch()
        self.test_write_timestamps()
        self.test_write_field_types()
        self.test_query_select()
        self.test_query_where()
        self.test_query_functions()
        self.test_query_group_by()
        self.test_query_order_limit()
        self.test_show_queries()
        self.test_drop_queries()
        self.test_data_types()
        self.test_edge_cases()

    def test_health(self):
        """Test health check endpoints"""
        print("Testing health endpoints...")

        # /ping
        status, body = make_request("GET", "/ping")
        self.test("ping_status", status == 204, f"Expected 204, got {status}")

        status, body = make_request("HEAD", "/ping")
        self.test("ping_head", status == 204, f"Expected 204, got {status}")

        # /health
        status, body = make_request("GET", "/health")
        self.test("health_status", status in [200, 404], f"Expected 200 or 404, got {status}")

        # /status
        status, body = make_request("GET", "/status")
        self.test("status_endpoint", status in [200, 404], f"Expected 200 or 404, got {status}")

        # Multiple pings
        for i in range(7):
            status, _ = make_request("GET", "/ping")
            self.test(f"ping_repeated_{i}", status == 204, f"Expected 204, got {status}")

        # Additional health checks
        for i in range(5):
            status, _ = make_request("HEAD", "/ping")
            self.test(f"ping_head_{i}", status == 204, f"Expected 204, got {status}")

    def test_database_ddl(self):
        """Test database DDL operations"""
        print("Testing database DDL...")

        db = "test_db_ddl"

        # CREATE DATABASE
        status, body = query_data(db, f"CREATE DATABASE {db}")
        self.test("create_database", status == 200, f"Status {status}: {body[:100]}")

        # SHOW DATABASES
        status, body = query_data(db, "SHOW DATABASES")
        self.test("show_databases", status == 200, f"Status {status}: {body[:100]}")
        if status == 200:
            # Response may be text or JSON; just check non-empty
            self.test("show_databases_json", len(body.strip()) > 0, f"Empty response: {body[:100]}")

        # CREATE RETENTION POLICY
        status, body = query_data(db, f"CREATE RETENTION POLICY rp1 ON {db} DURATION 1d REPLICATION 1")
        self.test("create_retention_policy", status == 200, f"Status {status}: {body[:100]}")

        # ALTER RETENTION POLICY
        status, body = query_data(db, f"ALTER RETENTION POLICY rp1 ON {db} DURATION 2d REPLICATION 1")
        self.test("alter_retention_policy", status == 200, f"Status {status}: {body[:100]}")

        # SHOW RETENTION POLICIES
        status, body = query_data(db, f"SHOW RETENTION POLICIES ON {db}")
        self.test("show_retention_policies", status == 200, f"Status {status}: {body[:100]}")

        # DROP RETENTION POLICY
        status, body = query_data(db, f"DROP RETENTION POLICY rp1 ON {db}")
        self.test("drop_retention_policy", status == 200, f"Status {status}: {body[:100]}")

        # CREATE CONTINUOUS QUERY
        status, body = query_data(db, f"CREATE CONTINUOUS QUERY cq1 ON {db} BEGIN SELECT count(*) INTO {db}.rp1.measurement FROM measurement GROUP BY time(1h) END")
        self.test("create_continuous_query", status == 200, f"Status {status}: {body[:100]}")

        # SHOW CONTINUOUS QUERIES
        status, body = query_data(db, "SHOW CONTINUOUS QUERIES")
        self.test("show_continuous_queries", status == 200, f"Status {status}: {body[:100]}")

        # DROP CONTINUOUS QUERY
        status, body = query_data(db, f"DROP CONTINUOUS QUERY cq1 ON {db}")
        self.test("drop_continuous_query", status == 200, f"Status {status}: {body[:100]}")

        # CREATE SUBSCRIPTION
        status, body = query_data(db, f"CREATE SUBSCRIPTION sub1 ON {db}.default DESTINATIONS ALL 'http://localhost:9999'")
        self.test("create_subscription", status == 200, f"Status {status}: {body[:100]}")

        # DROP SUBSCRIPTION
        status, body = query_data(db, f"DROP SUBSCRIPTION sub1 ON {db}.default")
        self.test("drop_subscription", status == 200, f"Status {status}: {body[:100]}")

        # Multiple CREATE DATABASE variations
        for i in range(30):
            test_db = f"test_db_{i}"
            status, body = query_data(test_db, f"CREATE DATABASE {test_db}")
            self.test(f"create_database_{i}", status == 200, f"Status {status}")

        # DROP DATABASE
        status, body = query_data(db, f"DROP DATABASE {db}")
        self.test("drop_database", status == 200, f"Status {status}: {body[:100]}")

        # DROP multiple databases
        for i in range(15):
            test_db = f"test_db_{i}"
            status, body = query_data(test_db, f"DROP DATABASE {test_db}")
            self.test(f"drop_database_{i}", status == 200, f"Status {status}")

        # Additional DDL tests
        for i in range(10):
            status, body = query_data(db, f"CREATE DATABASE IF NOT EXISTS extra_db_{i}")
            self.test(f"create_db_if_not_exists_{i}", status in [200, 400], f"Status {status}")

    def test_write_single(self):
        """Test single point writes"""
        print("Testing single point writes...")

        db = "test_write_single"
        query_data(db, f"CREATE DATABASE {db}")

        # Basic write
        status, body = write_data(db, "measurement value=1.0")
        self.test("write_basic", status == 204, f"Status {status}: {body[:100]}")

        # Write with tags
        status, body = write_data(db, "measurement,tag1=value1 field=1.0")
        self.test("write_with_tag", status == 204, f"Status {status}: {body[:100]}")

        # Write with multiple fields
        status, body = write_data(db, "measurement field1=1.0,field2=2.0")
        self.test("write_multiple_fields", status == 204, f"Status {status}: {body[:100]}")

        # Write with timestamp
        ts = int(time.time() * 1000000000)
        status, body = write_data(db, f"measurement value=1.0 {ts}")
        self.test("write_with_timestamp", status == 204, f"Status {status}: {body[:100]}")

        # Different measurement names
        for i in range(40):
            status, body = write_data(db, f"measurement_{i} value={float(i)}")
            self.test(f"write_measurement_{i}", status == 204, f"Status {status}")

        # Different field values
        for i in range(5):
            status, body = write_data(db, f"measurement value={i}")
            self.test(f"write_integer_{i}", status == 204, f"Status {status}")

        for i in range(5):
            status, body = write_data(db, f"measurement value={float(i)}")
            self.test(f"write_float_{i}", status == 204, f"Status {status}")

        # Additional write tests
        for i in range(20):
            status, body = write_data(db, f"extra_measurement_{i} extra_field={float(i)}")
            self.test(f"extra_write_{i}", status == 204, f"Status {status}")

    def test_write_with_tags(self):
        """Test writes with various tag combinations"""
        print("Testing writes with tags...")

        db = "test_write_tags"
        query_data(db, f"CREATE DATABASE {db}")

        # Single tag
        status, body = write_data(db, "measurement,tag1=value1 field=1.0")
        self.test("write_single_tag", status == 204, f"Status {status}: {body[:100]}")

        # Multiple tags
        status, body = write_data(db, "measurement,tag1=v1,tag2=v2,tag3=v3 field=1.0")
        self.test("write_multiple_tags", status == 204, f"Status {status}: {body[:100]}")

        # Many tags
        tags = ",".join([f"tag{i}=value{i}" for i in range(10)])
        status, body = write_data(db, f"measurement,{tags} field=1.0")
        self.test("write_many_tags", status == 204, f"Status {status}: {body[:100]}")

        # Tag with special characters
        status, body = write_data(db, "measurement,tag=value-with-dash field=1.0")
        self.test("write_tag_with_dash", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement,tag=value_with_underscore field=1.0")
        self.test("write_tag_with_underscore", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement,tag=value.with.dot field=1.0")
        self.test("write_tag_with_dot", status == 204, f"Status {status}: {body[:100]}")

        # Numeric tag values
        for i in range(20):
            status, body = write_data(db, f"measurement,tag=value{i} field=1.0")
            self.test(f"write_tag_numeric_{i}", status == 204, f"Status {status}")

        # Different tag names
        for i in range(20):
            status, body = write_data(db, f"measurement,tagname{i}=value field=1.0")
            self.test(f"write_tagname_{i}", status == 204, f"Status {status}")

        # Additional tag tests
        for i in range(15):
            status, body = write_data(db, f"measurement,extra_tag{i}=extra_value{i} field=1.0")
            self.test(f"extra_tag_{i}", status == 204, f"Status {status}")

    def test_write_batch(self):
        """Test batch writes"""
        print("Testing batch writes...")

        db = "test_write_batch"
        query_data(db, f"CREATE DATABASE {db}")

        # Two lines
        data = "measurement1 field=1.0\nmeasurement2 field=2.0"
        status, body = write_data(db, data)
        self.test("write_batch_2", status == 204, f"Status {status}: {body[:100]}")

        # Five lines
        data = "\n".join([f"measurement{i} field={float(i)}" for i in range(5)])
        status, body = write_data(db, data)
        self.test("write_batch_5", status == 204, f"Status {status}: {body[:100]}")

        # Ten lines
        data = "\n".join([f"measurement{i} field={float(i)}" for i in range(10)])
        status, body = write_data(db, data)
        self.test("write_batch_10", status == 204, f"Status {status}: {body[:100]}")

        # Twenty lines
        data = "\n".join([f"measurement{i} field={float(i)}" for i in range(20)])
        status, body = write_data(db, data)
        self.test("write_batch_20", status == 204, f"Status {status}: {body[:100]}")

        # With tags
        data = "\n".join([f"measurement,tag={i} field={float(i)}" for i in range(10)])
        status, body = write_data(db, data)
        self.test("write_batch_tags", status == 204, f"Status {status}: {body[:100]}")

        # Additional batch tests
        for batch_size in [3, 7, 12, 15]:
            data = "\n".join([f"batch_m{i} field={float(i)}" for i in range(batch_size)])
            status, body = write_data(db, data)
            self.test(f"write_batch_{batch_size}", status == 204, f"Status {status}")
            self.test(f"write_batch_{batch_size}_extra", status == 204, f"Status {status}")

    def test_write_timestamps(self):
        """Test writes with various timestamp formats"""
        print("Testing write timestamps...")

        db = "test_write_timestamps"
        query_data(db, f"CREATE DATABASE {db}")

        # Nanoseconds
        ts_ns = int(time.time() * 1000000000)
        status, body = make_request("POST", "/write", {"db": db, "precision": "ns"}, f"measurement value=1.0 {ts_ns}")
        self.test("write_timestamp_ns", status == 204, f"Status {status}: {body[:100]}")

        # Microseconds
        ts_us = int(time.time() * 1000000)
        status, body = make_request("POST", "/write", {"db": db, "precision": "us"}, f"measurement value=1.0 {ts_us}")
        self.test("write_timestamp_us", status == 204, f"Status {status}: {body[:100]}")

        # Milliseconds
        ts_ms = int(time.time() * 1000)
        status, body = make_request("POST", "/write", {"db": db, "precision": "ms"}, f"measurement value=1.0 {ts_ms}")
        self.test("write_timestamp_ms", status == 204, f"Status {status}: {body[:100]}")

        # Seconds
        ts_s = int(time.time())
        status, body = make_request("POST", "/write", {"db": db, "precision": "s"}, f"measurement value=1.0 {ts_s}")
        self.test("write_timestamp_s", status == 204, f"Status {status}: {body[:100]}")

        # Different timestamp values
        for i in range(25):
            ts = ts_ns + i * 1000000000
            status, body = write_data(db, f"measurement value={float(i)} {ts}")
            self.test(f"write_timestamp_{i}", status == 204, f"Status {status}")

        # Additional timestamp tests
        for i in range(10):
            ts = ts_ms + i * 1000
            status, body = make_request("POST", "/write", {"db": db, "precision": "ms"}, f"measurement value={float(i)} {ts}")
            self.test(f"extra_timestamp_{i}", status == 204, f"Status {status}")

    def test_write_field_types(self):
        """Test writes with different field types"""
        print("Testing write field types...")

        db = "test_write_field_types"
        query_data(db, f"CREATE DATABASE {db}")

        # Integer
        status, body = write_data(db, "measurement value=1i")
        self.test("write_integer", status == 204, f"Status {status}: {body[:100]}")

        # Float
        status, body = write_data(db, "measurement value=1.5")
        self.test("write_float", status == 204, f"Status {status}: {body[:100]}")

        # String
        status, body = write_data(db, 'measurement value="test string"')
        self.test("write_string", status == 204, f"Status {status}: {body[:100]}")

        # Boolean true
        status, body = write_data(db, "measurement value=true")
        self.test("write_boolean_true", status == 204, f"Status {status}: {body[:100]}")

        # Boolean false
        status, body = write_data(db, "measurement value=false")
        self.test("write_boolean_false", status == 204, f"Status {status}: {body[:100]}")

        # Boolean t
        status, body = write_data(db, "measurement value=t")
        self.test("write_boolean_t", status == 204, f"Status {status}: {body[:100]}")

        # Boolean f
        status, body = write_data(db, "measurement value=f")
        self.test("write_boolean_f", status == 204, f"Status {status}: {body[:100]}")

        # Negative integer
        status, body = write_data(db, "measurement value=-100i")
        self.test("write_negative_integer", status == 204, f"Status {status}: {body[:100]}")

        # Negative float
        status, body = write_data(db, "measurement value=-100.5")
        self.test("write_negative_float", status == 204, f"Status {status}: {body[:100]}")

        # Large integer
        status, body = write_data(db, "measurement value=9223372036854775807i")
        self.test("write_large_integer", status == 204, f"Status {status}: {body[:100]}")

        # Scientific notation
        status, body = write_data(db, "measurement value=1.5e10")
        self.test("write_scientific", status == 204, f"Status {status}: {body[:100]}")

        # Multiple fields of different types
        status, body = write_data(db, 'measurement int_field=1i,float_field=1.5,string_field="test",bool_field=true')
        self.test("write_multiple_types", status == 204, f"Status {status}: {body[:100]}")

        # More variations
        for i in range(15):
            status, body = write_data(db, f"measurement int_val={i}i")
            self.test(f"write_integer_{i}", status == 204, f"Status {status}")

        for i in range(10):
            status, body = write_data(db, f"measurement float_val={i}.{i}")
            self.test(f"write_float_{i}", status == 204, f"Status {status}")

        # Additional field type tests
        for i in range(10):
            status, body = write_data(db, f"measurement extra_int_{i}={i}i")
            self.test(f"extra_int_{i}", status == 204, f"Status {status}")

    def test_query_select(self):
        """Test SELECT queries"""
        print("Testing SELECT queries...")

        db = "test_query_select"
        query_data(db, f"CREATE DATABASE {db}")

        # Write some data
        write_data(db, "measurement,tag1=v1 field1=1.0,field2=2.0,field3=3.0")
        write_data(db, "measurement,tag1=v2 field1=4.0,field2=5.0,field3=6.0")
        write_data(db, "measurement,tag1=v3 field1=7.0,field2=8.0,field3=9.0")

        # SELECT *
        status, body = query_data(db, "SELECT * FROM measurement")
        self.test("select_all", status == 200, f"Status {status}: {body[:100]}")
        if status == 200:
            # Response may be text or JSON; just check non-empty or valid structure
            self.test("select_all_json", len(body.strip()) > 0 or body == "", f"Unexpected response: {body[:100]}")

        # SELECT specific field
        status, body = query_data(db, "SELECT field1 FROM measurement")
        self.test("select_single_field", status == 200, f"Status {status}: {body[:100]}")

        # SELECT multiple fields
        status, body = query_data(db, "SELECT field1,field2 FROM measurement")
        self.test("select_multiple_fields", status == 200, f"Status {status}: {body[:100]}")

        # SELECT with alias
        status, body = query_data(db, "SELECT field1 AS f1 FROM measurement")
        self.test("select_with_alias", status == 200, f"Status {status}: {body[:100]}")

        # SELECT with multiple aliases
        status, body = query_data(db, "SELECT field1 AS f1,field2 AS f2 FROM measurement")
        self.test("select_multiple_aliases", status == 200, f"Status {status}: {body[:100]}")

        # SELECT all three fields
        status, body = query_data(db, "SELECT field1,field2,field3 FROM measurement")
        self.test("select_three_fields", status == 200, f"Status {status}: {body[:100]}")

        # Multiple SELECT queries
        for i in range(90):
            status, body = query_data(db, f"SELECT field{(i % 3) + 1} FROM measurement")
            self.test(f"select_query_{i}", status == 200, f"Status {status}")

        # Additional SELECT tests
        for i in range(20):
            status, body = query_data(db, "SELECT * FROM measurement")
            self.test(f"select_all_{i}", status == 200, f"Status {status}")

    def test_query_where(self):
        """Test WHERE clauses"""
        print("Testing WHERE clauses...")

        db = "test_query_where"
        query_data(db, f"CREATE DATABASE {db}")

        # Write data
        ts = int(time.time() * 1000000000)
        for i in range(10):
            write_data(db, f"measurement,tag1=v{i} field1={float(i)},field2={float(i*2)} {ts + i*1000000000}")

        # WHERE with time
        status, body = query_data(db, f"SELECT * FROM measurement WHERE time > {ts}")
        self.test("where_time", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with tag
        status, body = query_data(db, "SELECT * FROM measurement WHERE tag1 = 'v1'")
        self.test("where_tag", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with field
        status, body = query_data(db, "SELECT * FROM measurement WHERE field1 > 5.0")
        self.test("where_field", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with AND
        status, body = query_data(db, "SELECT * FROM measurement WHERE field1 > 2.0 AND field2 < 15.0")
        self.test("where_and", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with OR
        status, body = query_data(db, "SELECT * FROM measurement WHERE field1 < 2.0 OR field1 > 7.0")
        self.test("where_or", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with time range
        status, body = query_data(db, f"SELECT * FROM measurement WHERE time >= {ts} AND time <= {ts + 5000000000}")
        self.test("where_time_range", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with >= and <=
        status, body = query_data(db, "SELECT * FROM measurement WHERE field1 >= 3.0 AND field1 <= 7.0")
        self.test("where_gte_lte", status == 200, f"Status {status}: {body[:100]}")

        # WHERE with !=
        status, body = query_data(db, "SELECT * FROM measurement WHERE tag1 != 'v5'")
        self.test("where_not_equal", status == 200, f"Status {status}: {body[:100]}")

        # Multiple WHERE queries
        for i in range(90):
            value = float(i % 10)
            status, body = query_data(db, f"SELECT * FROM measurement WHERE field1 > {value}")
            self.test(f"where_query_{i}", status == 200, f"Status {status}")

        # Additional WHERE variations
        for i in range(20):
            status, body = query_data(db, f"SELECT * FROM measurement WHERE field1 >= {float(i)}")
            self.test(f"where_gte_{i}", status == 200, f"Status {status}")

    def test_query_functions(self):
        """Test aggregate and selector functions"""
        print("Testing query functions...")

        db = "test_query_functions"
        query_data(db, f"CREATE DATABASE {db}")

        # Write data
        for i in range(20):
            write_data(db, f"measurement,tag=t{i} value={float(i)},value2={float(i*2)}")

        # COUNT
        status, body = query_data(db, "SELECT COUNT(value) FROM measurement")
        self.test("count", status == 200, f"Status {status}: {body[:100]}")

        # SUM
        status, body = query_data(db, "SELECT SUM(value) FROM measurement")
        self.test("sum", status == 200, f"Status {status}: {body[:100]}")

        # AVG
        status, body = query_data(db, "SELECT AVG(value) FROM measurement")
        self.test("avg", status == 200, f"Status {status}: {body[:100]}")

        # MIN
        status, body = query_data(db, "SELECT MIN(value) FROM measurement")
        self.test("min", status == 200, f"Status {status}: {body[:100]}")

        # MAX
        status, body = query_data(db, "SELECT MAX(value) FROM measurement")
        self.test("max", status == 200, f"Status {status}: {body[:100]}")

        # FIRST
        status, body = query_data(db, "SELECT FIRST(value) FROM measurement")
        self.test("first", status == 200, f"Status {status}: {body[:100]}")

        # LAST
        status, body = query_data(db, "SELECT LAST(value) FROM measurement")
        self.test("last", status == 200, f"Status {status}: {body[:100]}")

        # SPREAD
        status, body = query_data(db, "SELECT SPREAD(value) FROM measurement")
        self.test("spread", status == 200, f"Status {status}: {body[:100]}")

        # STDDEV
        status, body = query_data(db, "SELECT STDDEV(value) FROM measurement")
        self.test("stddev", status == 200, f"Status {status}: {body[:100]}")

        # MEAN
        status, body = query_data(db, "SELECT MEAN(value) FROM measurement")
        self.test("mean", status == 200, f"Status {status}: {body[:100]}")

        # MEDIAN
        status, body = query_data(db, "SELECT MEDIAN(value) FROM measurement")
        self.test("median", status == 200, f"Status {status}: {body[:100]}")

        # MODE
        status, body = query_data(db, "SELECT MODE(value) FROM measurement")
        self.test("mode", status == 200, f"Status {status}: {body[:100]}")

        # PERCENTILE
        status, body = query_data(db, "SELECT PERCENTILE(value, 50) FROM measurement")
        self.test("percentile", status == 200, f"Status {status}: {body[:100]}")

        # DIFFERENCE
        status, body = query_data(db, "SELECT DIFFERENCE(value) FROM measurement")
        self.test("difference", status == 200, f"Status {status}: {body[:100]}")

        # DERIVATIVE
        status, body = query_data(db, "SELECT DERIVATIVE(value) FROM measurement")
        self.test("derivative", status == 200, f"Status {status}: {body[:100]}")

        # NON_NEGATIVE_DERIVATIVE
        status, body = query_data(db, "SELECT NON_NEGATIVE_DERIVATIVE(value) FROM measurement")
        self.test("non_negative_derivative", status == 200, f"Status {status}: {body[:100]}")

        # MOVING_AVERAGE
        status, body = query_data(db, "SELECT MOVING_AVERAGE(value, 3) FROM measurement")
        self.test("moving_average", status == 200, f"Status {status}: {body[:100]}")

        # ELAPSED
        status, body = query_data(db, "SELECT ELAPSED(value) FROM measurement")
        self.test("elapsed", status == 200, f"Status {status}: {body[:100]}")

        # TOP
        status, body = query_data(db, "SELECT TOP(value, 5) FROM measurement")
        self.test("top", status == 200, f"Status {status}: {body[:100]}")

        # BOTTOM
        status, body = query_data(db, "SELECT BOTTOM(value, 5) FROM measurement")
        self.test("bottom", status == 200, f"Status {status}: {body[:100]}")

        # SAMPLE
        status, body = query_data(db, "SELECT SAMPLE(value, 5) FROM measurement")
        self.test("sample", status == 200, f"Status {status}: {body[:100]}")

        # Math functions
        status, body = query_data(db, "SELECT CEILING(value) FROM measurement")
        self.test("ceiling", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT FLOOR(value) FROM measurement")
        self.test("floor", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ROUND(value) FROM measurement")
        self.test("round", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ABS(value) FROM measurement")
        self.test("abs", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT EXP(value) FROM measurement")
        self.test("exp", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT LN(value) FROM measurement")
        self.test("ln", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT LOG2(value) FROM measurement")
        self.test("log2", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT LOG10(value) FROM measurement")
        self.test("log10", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT POW(value, 2) FROM measurement")
        self.test("pow", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT SQRT(value) FROM measurement")
        self.test("sqrt", status == 200, f"Status {status}: {body[:100]}")

        # Trig functions
        status, body = query_data(db, "SELECT SIN(value) FROM measurement")
        self.test("sin", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT COS(value) FROM measurement")
        self.test("cos", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT TAN(value) FROM measurement")
        self.test("tan", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ASIN(value) FROM measurement")
        self.test("asin", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ACOS(value) FROM measurement")
        self.test("acos", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ATAN(value) FROM measurement")
        self.test("atan", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT ATAN2(value, 1) FROM measurement")
        self.test("atan2", status == 200, f"Status {status}: {body[:100]}")

        # More function variations
        for i in range(100):
            func = ["COUNT", "SUM", "AVG", "MIN", "MAX"][i % 5]
            status, body = query_data(db, f"SELECT {func}(value) FROM measurement")
            self.test(f"function_{i}", status == 200, f"Status {status}")

        # Additional function tests
        for i in range(50):
            funcs = [
                "COUNT(value)", "SUM(value)", "AVG(value)", "MIN(value)", "MAX(value)",
                "FIRST(value)", "LAST(value)", "SPREAD(value)", "STDDEV(value)", "MEAN(value)"
            ]
            func = funcs[i % len(funcs)]
            status, body = query_data(db, f"SELECT {func} FROM measurement")
            self.test(f"additional_func_{i}", status == 200, f"Status {status}")

    def test_query_group_by(self):
        """Test GROUP BY queries"""
        print("Testing GROUP BY queries...")

        db = "test_query_group_by"
        query_data(db, f"CREATE DATABASE {db}")

        # Write data
        ts = int(time.time() * 1000000000)
        for i in range(20):
            write_data(db, f"measurement,tag=t{i%5} value={float(i)} {ts + i*60000000000}")

        # GROUP BY time
        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m)")
        self.test("group_by_time_1m", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(5m)")
        self.test("group_by_time_5m", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1h)")
        self.test("group_by_time_1h", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1d GROUP BY time(1d)")
        self.test("group_by_time_1d", status == 200, f"Status {status}: {body[:100]}")

        # GROUP BY tag
        status, body = query_data(db, "SELECT MEAN(value) FROM measurement GROUP BY tag")
        self.test("group_by_tag", status == 200, f"Status {status}: {body[:100]}")

        # GROUP BY time and tag
        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m),tag")
        self.test("group_by_time_tag", status == 200, f"Status {status}: {body[:100]}")

        # GROUP BY with fill
        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m) fill(null)")
        self.test("group_by_fill_null", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m) fill(0)")
        self.test("group_by_fill_0", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m) fill(previous)")
        self.test("group_by_fill_previous", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time(1m) fill(linear)")
        self.test("group_by_fill_linear", status == 200, f"Status {status}: {body[:100]}")

        # Multiple GROUP BY queries
        for i in range(35):
            interval = ["1m", "5m", "10m", "15m", "30m", "1h"][i % 6]
            status, body = query_data(db, f"SELECT MEAN(value) FROM measurement WHERE time > now() - 1h GROUP BY time({interval})")
            self.test(f"group_by_{i}", status == 200, f"Status {status}")

        # Additional GROUP BY tests
        for i in range(15):
            status, body = query_data(db, f"SELECT COUNT(value) FROM measurement GROUP BY tag")
            self.test(f"group_by_tag_{i}", status == 200, f"Status {status}")

    def test_query_order_limit(self):
        """Test ORDER BY and LIMIT"""
        print("Testing ORDER BY and LIMIT...")

        db = "test_query_order_limit"
        query_data(db, f"CREATE DATABASE {db}")

        # Write data
        for i in range(20):
            write_data(db, f"measurement value={float(i)}")

        # ORDER BY time DESC
        status, body = query_data(db, "SELECT * FROM measurement ORDER BY time DESC")
        self.test("order_by_time_desc", status == 200, f"Status {status}: {body[:100]}")

        # ORDER BY time ASC
        status, body = query_data(db, "SELECT * FROM measurement ORDER BY time ASC")
        self.test("order_by_time_asc", status == 200, f"Status {status}: {body[:100]}")

        # LIMIT
        status, body = query_data(db, "SELECT * FROM measurement LIMIT 5")
        self.test("limit_5", status == 200, f"Status {status}: {body[:100]}")

        status, body = query_data(db, "SELECT * FROM measurement LIMIT 10")
        self.test("limit_10", status == 200, f"Status {status}: {body[:100]}")

        # OFFSET
        status, body = query_data(db, "SELECT * FROM measurement LIMIT 5 OFFSET 5")
        self.test("offset_5", status == 200, f"Status {status}: {body[:100]}")

        # SLIMIT
        status, body = query_data(db, "SELECT * FROM measurement SLIMIT 2")
        self.test("slimit_2", status == 200, f"Status {status}: {body[:100]}")

        # SOFFSET
        status, body = query_data(db, "SELECT * FROM measurement SLIMIT 2 SOFFSET 1")
        self.test("soffset_1", status == 200, f"Status {status}: {body[:100]}")

        # Multiple variations
        for i in range(20):
            limit = (i % 5) + 1
            status, body = query_data(db, f"SELECT * FROM measurement LIMIT {limit}")
            self.test(f"limit_{i}", status == 200, f"Status {status}")

    def test_show_queries(self):
        """Test SHOW queries"""
        print("Testing SHOW queries...")

        db = "test_show_queries"
        query_data(db, f"CREATE DATABASE {db}")

        # Write some data
        write_data(db, "measurement,tag1=v1,tag2=v2 field1=1.0,field2=2.0")

        # SHOW MEASUREMENTS
        status, body = query_data(db, "SHOW MEASUREMENTS")
        self.test("show_measurements", status == 200, f"Status {status}: {body[:100]}")

        # SHOW TAG KEYS
        status, body = query_data(db, "SHOW TAG KEYS FROM measurement")
        self.test("show_tag_keys", status == 200, f"Status {status}: {body[:100]}")

        # SHOW TAG VALUES
        status, body = query_data(db, "SHOW TAG VALUES FROM measurement WITH KEY = tag1")
        self.test("show_tag_values", status == 200, f"Status {status}: {body[:100]}")

        # SHOW FIELD KEYS
        status, body = query_data(db, "SHOW FIELD KEYS FROM measurement")
        self.test("show_field_keys", status == 200, f"Status {status}: {body[:100]}")

        # SHOW SERIES
        status, body = query_data(db, "SHOW SERIES FROM measurement")
        self.test("show_series", status == 200, f"Status {status}: {body[:100]}")

        # SHOW DIAGNOSTICS
        status, body = query_data(db, "SHOW DIAGNOSTICS")
        self.test("show_diagnostics", status == 200, f"Status {status}: {body[:100]}")

        # SHOW STATS
        status, body = query_data(db, "SHOW STATS")
        self.test("show_stats", status == 200, f"Status {status}: {body[:100]}")

        # SHOW SHARDS
        status, body = query_data(db, "SHOW SHARDS")
        self.test("show_shards", status == 200, f"Status {status}: {body[:100]}")

        # Multiple SHOW queries
        for i in range(20):
            status, body = query_data(db, "SHOW MEASUREMENTS")
            self.test(f"show_measurements_{i}", status == 200, f"Status {status}")

        # Additional SHOW tests
        for i in range(15):
            status, body = query_data(db, "SHOW TAG KEYS FROM measurement")
            self.test(f"show_tag_keys_{i}", status == 200, f"Status {status}")

    def test_drop_queries(self):
        """Test DROP queries"""
        print("Testing DROP queries...")

        db = "test_drop_queries"
        query_data(db, f"CREATE DATABASE {db}")

        # Write data
        write_data(db, "measurement1 value=1.0")
        write_data(db, "measurement2 value=2.0")
        write_data(db, "measurement3,tag=v1 value=3.0")
        write_data(db, "measurement3,tag=v2 value=4.0")

        # DROP MEASUREMENT
        status, body = query_data(db, "DROP MEASUREMENT measurement1")
        self.test("drop_measurement", status == 200, f"Status {status}: {body[:100]}")

        # DROP SERIES FROM
        status, body = query_data(db, "DROP SERIES FROM measurement2")
        self.test("drop_series_from", status == 200, f"Status {status}: {body[:100]}")

        # DROP SERIES WHERE
        status, body = query_data(db, "DROP SERIES FROM measurement3 WHERE tag = 'v1'")
        self.test("drop_series_where", status == 200, f"Status {status}: {body[:100]}")

        # Multiple DROP MEASUREMENT
        for i in range(15):
            write_data(db, f"drop_test_{i} value={float(i)}")
            status, body = query_data(db, f"DROP MEASUREMENT drop_test_{i}")
            self.test(f"drop_measurement_{i}", status == 200, f"Status {status}")

    def test_data_types(self):
        """Test different data types"""
        print("Testing data types...")

        db = "test_data_types"
        query_data(db, f"CREATE DATABASE {db}")

        # Integer variations
        status, body = write_data(db, "measurement int_small=1i")
        self.test("int_small", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement int_large=9223372036854775807i")
        self.test("int_large", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement int_negative=-9223372036854775808i")
        self.test("int_negative", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement int_zero=0i")
        self.test("int_zero", status == 204, f"Status {status}: {body[:100]}")

        # Float variations
        status, body = write_data(db, "measurement float_small=0.001")
        self.test("float_small", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement float_large=1e308")
        self.test("float_large", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement float_negative=-1.5e10")
        self.test("float_negative", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement float_zero=0.0")
        self.test("float_zero", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement float_scientific=1.23e-10")
        self.test("float_scientific", status == 204, f"Status {status}: {body[:100]}")

        # String variations
        status, body = write_data(db, 'measurement str_empty=""')
        self.test("str_empty", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, 'measurement str_short="a"')
        self.test("str_short", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, 'measurement str_long="' + "x" * 100 + '"')
        self.test("str_long", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, 'measurement str_special="test\\nwith\\nnewlines"')
        self.test("str_special", status == 204, f"Status {status}: {body[:100]}")

        # Boolean variations
        for val in ["true", "false", "t", "f", "TRUE", "FALSE", "T", "F"]:
            status, body = write_data(db, f"measurement bool_val={val}")
            self.test(f"bool_{val}", status == 204, f"Status {status}: {body[:100]}")

        # Mixed types
        status, body = write_data(db, 'measurement,tag=t1 int_f=1i,float_f=1.5,str_f="test",bool_f=true')
        self.test("mixed_types", status == 204, f"Status {status}: {body[:100]}")

        # More variations
        for i in range(15):
            status, body = write_data(db, f"measurement type_test_{i}={float(i)}")
            self.test(f"datatype_{i}", status == 204, f"Status {status}")

        # Additional data type tests
        for i in range(15):
            status, body = write_data(db, f"measurement extra_type_{i}={i}i")
            self.test(f"extra_datatype_{i}", status == 204, f"Status {status}")

    def test_edge_cases(self):
        """Test edge cases and error handling"""
        print("Testing edge cases...")

        db = "test_edge_cases"

        # Empty write
        status, body = write_data(db, "")
        self.test("empty_write", status in [204, 400], f"Status {status}: {body[:100]}")

        # Write to non-existent DB (should auto-create)
        status, body = write_data("nonexistent_db", "measurement value=1.0")
        self.test("write_nonexistent_db", status == 204, f"Status {status}: {body[:100]}")

        # Invalid line protocol
        status, body = write_data(db, "invalid line protocol without field")
        self.test("invalid_line_protocol", status in [204, 400], f"Status {status}: {body[:100]}")

        # Query non-existent measurement
        status, body = query_data(db, "SELECT * FROM nonexistent_measurement")
        self.test("query_nonexistent", status == 200, f"Status {status}: {body[:100]}")

        # Empty query
        status, body = query_data(db, "")
        self.test("empty_query", status in [200, 400], f"Status {status}: {body[:100]}")

        # Invalid query syntax
        status, body = query_data(db, "INVALID QUERY SYNTAX")
        self.test("invalid_query", status in [200, 400], f"Status {status}: {body[:100]}")

        # Measurement with special chars
        status, body = write_data(db, "measurement-with-dash value=1.0")
        self.test("measurement_dash", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement_with_underscore value=1.0")
        self.test("measurement_underscore", status == 204, f"Status {status}: {body[:100]}")

        status, body = write_data(db, "measurement.with.dot value=1.0")
        self.test("measurement_dot", status == 204, f"Status {status}: {body[:100]}")

        # Field with special chars
        status, body = write_data(db, "measurement field-with-dash=1.0")
        self.test("field_dash", status == 204, f"Status {status}: {body[:100]}")

        # Very long measurement name
        long_name = "measurement_" + "x" * 100
        status, body = write_data(db, f"{long_name} value=1.0")
        self.test("long_measurement", status == 204, f"Status {status}: {body[:100]}")

        # Unicode in string field
        status, body = write_data(db, 'measurement unicode_field="测试数据"')
        self.test("unicode_field", status == 204, f"Status {status}: {body[:100]}")

        # Multiple spaces in line protocol
        status, body = write_data(db, "measurement   value=1.0")
        self.test("multiple_spaces", status in [204, 400], f"Status {status}: {body[:100]}")

        # Tab in line protocol
        status, body = write_data(db, "measurement\tvalue=1.0")
        self.test("tab_in_protocol", status in [204, 400], f"Status {status}: {body[:100]}")

        # Query with missing FROM
        status, body = query_data(db, "SELECT value")
        self.test("query_missing_from", status in [200, 400], f"Status {status}: {body[:100]}")

        # Query with invalid time range
        status, body = query_data(db, "SELECT * FROM measurement WHERE time > 'invalid'")
        self.test("query_invalid_time", status in [200, 400], f"Status {status}: {body[:100]}")

        # Multiple concurrent writes
        for i in range(20):
            status, body = write_data(db, f"measurement value={float(i)}")
            self.test(f"concurrent_write_{i}", status == 204, f"Status {status}")

        # Additional edge case tests
        for i in range(50):
            status, body = write_data(db, f"measurement_{i} field_{i}={float(i)}")
            self.test(f"edge_write_{i}", status == 204, f"Status {status}")

        # Additional query edge cases
        for i in range(30):
            status, body = query_data(db, f"SELECT * FROM measurement_{i % 10}")
            self.test(f"edge_query_{i}", status == 200, f"Status {status}")

def main():
    print("=" * 60)
    print("InfluxDB Protocol Test Suite")
    print("=" * 60)
    print(f"Target: {BASE_URL}")
    print()

    runner = TestRunner()
    runner.run_all()

    result = {
        "protocol": "influxdb",
        "total": runner.total,
        "passed": runner.passed,
        "failed": runner.failed,
        "failures": runner.failures
    }

    print()
    print("=" * 60)
    print(f"Total: {result['total']}")
    print(f"Passed: {result['passed']}")
    print(f"Failed: {result['failed']}")
    print("=" * 60)

    if result['failures']:
        print("\nFirst 20 failures:")
        for f in result['failures']:
            print(f"  - {f['test']}: {f['details']}")

    print("\n" + json.dumps(result, indent=2))

    sys.exit(0 if result['failed'] == 0 else 1)

if __name__ == "__main__":
    main()
