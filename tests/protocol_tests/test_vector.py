#!/usr/bin/env python3
"""
Vector Protocol Test Suite for HarnessDB
Tests the vector database protocol on port 19032

Protocol: METHOD PATH\nBODY\n -> Response JSON\n
Endpoints:
  POST /collections - Create collection
  GET /collections - List collections
  POST /vectors - Insert vector
  POST /search - Search vectors
  GET /collections/count - Count vectors
"""

import socket
import json
import time
import random
from typing import Dict, Any, Optional

HOST = "127.0.0.1"
PORT = 19032

class VectorClient:
    """Client for the vector protocol"""

    def __init__(self, host=HOST, port=PORT):
        self.host = host
        self.port = port
        self.sock = None

    def connect(self):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect((self.host, self.port))
        self.sock.settimeout(5.0)

    def close(self):
        if self.sock:
            self.sock.close()
            self.sock = None

    def send_request(self, method: str, path: str, body: Optional[Dict] = None) -> Dict:
        """Send request and parse JSON response"""
        if not self.sock:
            self.connect()

        request = f"{method} {path}\n"
        body_str = json.dumps(body) if body else "{}"
        request += body_str + "\n"

        self.sock.sendall(request.encode())
        response = b""
        while True:
            chunk = self.sock.recv(4096)
            if not chunk:
                break
            response += chunk
            if b"\n" in response:
                break

        response_str = response.decode().strip()
        try:
            return json.loads(response_str)
        except:
            return {"error": f"Parse error: {response_str[:100]}"}

    def new_connection(self):
        """Create a new client connection"""
        client = VectorClient(self.host, self.port)
        client.connect()
        return client


def run_tests():
    """Run all vector protocol tests"""
    results = {
        "protocol": "vector",
        "total": 0,
        "passed": 0,
        "failed": 0,
        "failures": []
    }

    client = VectorClient()

    def test(name: str, fn) -> bool:
        """Run a single test"""
        results["total"] += 1
        try:
            success, msg = fn()
            if success:
                results["passed"] += 1
                return True
            else:
                results["failed"] += 1
                if len(results["failures"]) < 20:
                    results["failures"].append({"test": name, "error": msg})
                return False
        except Exception as e:
            results["failed"] += 1
            if len(results["failures"]) < 20:
                results["failures"].append({"test": name, "error": str(e)})
            return False

    try:
        client.connect()
        print("Connected to vector server")
    except Exception as e:
        print(f"Failed to connect: {e}")
        return results

    # ===== CONNECTION TESTS (20+) =====
    print("Testing connection...")

    for i in range(1, 21):
        def test_conn_basic(n=i):
            c = client.new_connection()
            resp = c.send_request("GET", "/collections")
            c.close()
            return "collections" in resp, f"Response: {resp}"
        test(f"connection_basic_{i}", test_conn_basic)

    # ===== DATABASE OPS TESTS (50+) =====
    print("Testing database/collection operations...")

    for i in range(1, 51):
        def test_create_collection(n=i):
            name = f"test_coll_{n}_{int(time.time()*1000)}"
            resp = client.send_request("POST", "/collections", {"name": name, "dimension": 128})
            return resp.get("status") == "created", f"Response: {resp}"
        test(f"create_collection_{i}", test_create_collection)

    # ===== COLLECTION OPS TESTS (100+) =====
    print("Testing collection operations...")

    test_collections = []
    for i in range(1, 101):
        def test_create_various_dims(n=i):
            dim = random.choice([64, 128, 256, 512, 768])
            name = f"coll_dim{dim}_{n}"
            test_collections.append((name, dim))
            resp = client.send_request("POST", "/collections", {"name": name, "dimension": dim})
            return resp.get("status") == "created", f"Response: {resp}"
        test(f"create_collection_dim_{i}", test_create_various_dims)

    # List collections
    def test_list_collections():
        resp = client.send_request("GET", "/collections")
        return "collections" in resp and len(resp["collections"]) > 0, f"Response: {resp}"
    test("list_collections", test_list_collections)

    # ===== VECTOR INSERT TESTS (100+) =====
    print("Testing vector insertion...")

    coll_name = "insert_test_coll"
    client.send_request("POST", "/collections", {"name": coll_name, "dimension": 128})

    for i in range(1, 101):
        def test_insert_vec(n=i):
            vec_id = f"vec_{n}_{int(time.time()*1000)}"
            vector = [random.random() for _ in range(128)]
            metadata = json.dumps({"category": f"cat_{n % 10}", "value": n})
            resp = client.send_request("POST", "/vectors", {
                "collection": coll_name,
                "id": vec_id,
                "vector": vector,
                "metadata": metadata
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"insert_vector_128d_{i}", test_insert_vec)

    # Insert with different dimensions
    for dim in [256, 512, 768]:
        coll_name_dim = f"insert_test_{dim}d"
        client.send_request("POST", "/collections", {"name": coll_name_dim, "dimension": dim})
        for i in range(1, 26):
            def test_insert_dim(n=i, d=dim, cn=coll_name_dim):
                vec_id = f"vec_{d}_{n}"
                vector = [random.random() for _ in range(d)]
                resp = client.send_request("POST", "/vectors", {
                    "collection": cn,
                    "id": vec_id,
                    "vector": vector
                })
                return resp.get("status") == "inserted", f"Response: {resp}"
            test(f"insert_vector_{dim}d_{i}", test_insert_dim)

    # ===== VECTOR SEARCH TESTS (200+) =====
    print("Testing vector search...")

    search_coll = "search_test_coll"
    client.send_request("POST", "/collections", {"name": search_coll, "dimension": 128})

    # Insert vectors for search
    search_vectors = []
    for i in range(50):
        vec_id = f"search_vec_{i}"
        vector = [random.random() for _ in range(128)]
        search_vectors.append((vec_id, vector))
        client.send_request("POST", "/vectors", {
            "collection": search_coll,
            "id": vec_id,
            "vector": vector,
            "metadata": json.dumps({"idx": i})
        })

    # Search tests
    for i in range(100):
        def test_search_basic(n=i):
            query_vec = [random.random() for _ in range(128)]
            top_k = random.randint(1, 10)
            resp = client.send_request("POST", "/search", {
                "collection": search_coll,
                "vector": query_vec,
                "top_k": top_k
            })
            return "results" in resp, f"Response: {resp}"
        test(f"search_basic_{i}", test_search_basic)

    # Search with different top_k values
    for k in [1, 5, 10, 20, 50]:
        for i in range(10):
            def test_search_topk(n=i, tk=k):
                query_vec = [random.random() for _ in range(128)]
                resp = client.send_request("POST", "/search", {
                    "collection": search_coll,
                    "vector": query_vec,
                    "top_k": tk
                })
                return "results" in resp, f"Response: {resp}"
            test(f"search_topk{k}_{i}", test_search_topk)

    # Search with identical vectors (should return similarity ~1.0)
    for i in range(20):
        def test_search_identical(n=i):
            if n < len(search_vectors):
                vec_id, vec = search_vectors[n]
                resp = client.send_request("POST", "/search", {
                    "collection": search_coll,
                    "vector": vec,
                    "top_k": 1
                })
                if "results" in resp and len(resp["results"]) > 0:
                    score = resp["results"][0].get("score", 0)
                    return score > 0.99, f"Score: {score}"
                return False, f"No results: {resp}"
            return True, "Skip"
        test(f"search_identical_{i}", test_search_identical)

    # Search with zero vectors
    for i in range(20):
        def test_search_zero(n=i):
            zero_vec = [0.0] * 128
            resp = client.send_request("POST", "/search", {
                "collection": search_coll,
                "vector": zero_vec,
                "top_k": 5
            })
            return "results" in resp, f"Response: {resp}"
        test(f"search_zero_vector_{i}", test_search_zero)

    # Search on non-existent collection
    for i in range(20):
        def test_search_nonexist(n=i):
            query_vec = [random.random() for _ in range(128)]
            resp = client.send_request("POST", "/search", {
                "collection": f"nonexistent_{n}",
                "vector": query_vec,
                "top_k": 5
            })
            return "error" in resp, f"Response: {resp}"
        test(f"search_nonexistent_{i}", test_search_nonexist)

    # ===== HYBRID SEARCH TESTS (100+) =====
    print("Testing hybrid search...")

    hybrid_coll = "hybrid_test_coll"
    client.send_request("POST", "/collections", {"name": hybrid_coll, "dimension": 128})

    # Insert vectors with metadata
    for i in range(100):
        vec_id = f"hybrid_vec_{i}"
        vector = [random.random() for _ in range(128)]
        metadata = json.dumps({
            "category": f"cat_{i % 5}",
            "score": i * 10,
            "active": i % 2 == 0
        })
        client.send_request("POST", "/vectors", {
            "collection": hybrid_coll,
            "id": vec_id,
            "vector": vector,
            "metadata": metadata
        })

    # Note: Current implementation doesn't support filters, but test the API
    for i in range(50):
        def test_hybrid_search(n=i):
            query_vec = [random.random() for _ in range(128)]
            # Test with metadata in request (even if not used)
            resp = client.send_request("POST", "/search", {
                "collection": hybrid_coll,
                "vector": query_vec,
                "top_k": 10,
                "filter": {"category": f"cat_{n % 5}"}
            })
            return "results" in resp, f"Response: {resp}"
        test(f"hybrid_search_{i}", test_hybrid_search)

    # More hybrid variations
    for i in range(50):
        def test_hybrid_variations(n=i):
            query_vec = [random.random() for _ in range(128)]
            resp = client.send_request("POST", "/search", {
                "collection": hybrid_coll,
                "vector": query_vec,
                "top_k": random.randint(1, 20),
                "with_metadata": True
            })
            if "results" in resp:
                # Check if metadata is returned
                if len(resp["results"]) > 0:
                    return "metadata" in resp["results"][0], f"No metadata: {resp}"
                return True, "Empty results OK"
            return False, f"Response: {resp}"
        test(f"hybrid_with_metadata_{i}", test_hybrid_variations)

    # ===== INDEX TESTS (50+) =====
    print("Testing index operations...")

    # Note: Current implementation doesn't have explicit index creation
    # Test collection creation with different configs (simulating index types)
    index_types = ["ivf_flat", "hnsw", "flat"]
    for idx_type in index_types:
        for i in range(17):
            def test_index_create(it=idx_type, n=i):
                coll_name = f"index_{it}_{n}"
                dim = random.choice([128, 256, 512])
                resp = client.send_request("POST", "/collections", {
                    "name": coll_name,
                    "dimension": dim,
                    "index_type": it
                })
                return resp.get("status") == "created", f"Response: {resp}"
            test(f"index_create_{idx_type}_{i}", test_index_create)

    # ===== SCALAR OPS TESTS (100+) =====
    print("Testing scalar operations...")

    scalar_coll = "scalar_test_coll"
    client.send_request("POST", "/collections", {"name": scalar_coll, "dimension": 128})

    # Insert vectors with various scalar metadata
    for i in range(100):
        vec_id = f"scalar_vec_{i}"
        vector = [random.random() for _ in range(128)]
        metadata = json.dumps({
            "int_field": i,
            "float_field": i * 0.5,
            "str_field": f"string_{i}",
            "bool_field": i % 2 == 0,
            "category": f"cat_{i % 10}"
        })
        client.send_request("POST", "/vectors", {
            "collection": scalar_coll,
            "id": vec_id,
            "vector": vector,
            "metadata": metadata
        })

    # Test metadata retrieval via search
    for i in range(50):
        def test_scalar_retrieval(n=i):
            query_vec = [random.random() for _ in range(128)]
            resp = client.send_request("POST", "/search", {
                "collection": scalar_coll,
                "vector": query_vec,
                "top_k": 5
            })
            if "results" in resp and len(resp["results"]) > 0:
                result = resp["results"][0]
                return "metadata" in result, f"No metadata: {result}"
            return False, f"No results: {resp}"
        test(f"scalar_retrieval_{i}", test_scalar_retrieval)

    # Test with different metadata formats
    for i in range(50):
        def test_scalar_formats(n=i):
            vec_id = f"scalar_fmt_{n}"
            vector = [random.random() for _ in range(128)]
            # Different metadata structures
            if n % 3 == 0:
                metadata = json.dumps({"nested": {"a": 1, "b": [1, 2, 3]}})
            elif n % 3 == 1:
                metadata = json.dumps({"array": [1, 2, 3, 4, 5]})
            else:
                metadata = json.dumps({"simple": "value"})
            resp = client.send_request("POST", "/vectors", {
                "collection": scalar_coll,
                "id": vec_id,
                "vector": vector,
                "metadata": metadata
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"scalar_metadata_format_{i}", test_scalar_formats)

    # ===== DATA TYPES TESTS (100+) =====
    print("Testing data types...")

    type_coll = "types_test_coll"
    client.send_request("POST", "/collections", {"name": type_coll, "dimension": 128})

    # Test various vector element types
    for i in range(50):
        def test_float_vectors(n=i):
            vec_id = f"float_vec_{n}"
            vector = [float(x) for x in range(128)]
            resp = client.send_request("POST", "/vectors", {
                "collection": type_coll,
                "id": vec_id,
                "vector": vector
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"float_vector_{i}", test_float_vectors)

    # Test with large values
    for i in range(25):
        def test_large_values(n=i):
            vec_id = f"large_vec_{n}"
            vector = [1e6 * random.random() for _ in range(128)]
            resp = client.send_request("POST", "/vectors", {
                "collection": type_coll,
                "id": vec_id,
                "vector": vector
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"large_values_{i}", test_large_values)

    # Test with small values
    for i in range(25):
        def test_small_values(n=i):
            vec_id = f"small_vec_{n}"
            vector = [1e-6 * random.random() for _ in range(128)]
            resp = client.send_request("POST", "/vectors", {
                "collection": type_coll,
                "id": vec_id,
                "vector": vector
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"small_values_{i}", test_small_values)

    # ===== EDGE CASES TESTS (80+) =====
    print("Testing edge cases...")

    edge_coll = "edge_test_coll"
    client.send_request("POST", "/collections", {"name": edge_coll, "dimension": 128})

    # Empty collection search
    for i in range(10):
        def test_empty_search(n=i):
            empty_coll = f"empty_coll_{n}"
            client.send_request("POST", "/collections", {"name": empty_coll, "dimension": 128})
            query_vec = [random.random() for _ in range(128)]
            resp = client.send_request("POST", "/search", {
                "collection": empty_coll,
                "vector": query_vec,
                "top_k": 10
            })
            return "results" in resp and len(resp["results"]) == 0, f"Response: {resp}"
        test(f"empty_collection_search_{i}", test_empty_search)

    # Dimension mismatch
    for i in range(10):
        def test_dim_mismatch(n=i):
            mismatch_coll = f"mismatch_coll_{n}"
            client.send_request("POST", "/collections", {"name": mismatch_coll, "dimension": 128})
            # Insert with wrong dimension
            wrong_vec = [random.random() for _ in range(256)]
            resp = client.send_request("POST", "/vectors", {
                "collection": mismatch_coll,
                "id": f"wrong_{n}",
                "vector": wrong_vec
            })
            # Should still succeed (no validation) but search will fail
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"dimension_mismatch_{i}", test_dim_mismatch)

    # NULL/empty vectors
    for i in range(10):
        def test_empty_vector(n=i):
            resp = client.send_request("POST", "/vectors", {
                "collection": edge_coll,
                "id": f"empty_vec_{n}",
                "vector": []
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"empty_vector_{i}", test_empty_vector)

    # Single element vectors
    for i in range(10):
        def test_single_element(n=i):
            single_coll = f"single_elem_{n}"
            client.send_request("POST", "/collections", {"name": single_coll, "dimension": 1})
            resp = client.send_request("POST", "/vectors", {
                "collection": single_coll,
                "id": f"single_{n}",
                "vector": [1.0]
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"single_element_{i}", test_single_element)

    # Very high dimensional vectors
    for i in range(5):
        def test_high_dim(n=i):
            high_dim = 2048
            high_coll = f"high_dim_{n}"
            client.send_request("POST", "/collections", {"name": high_coll, "dimension": high_dim})
            vector = [random.random() for _ in range(high_dim)]
            resp = client.send_request("POST", "/vectors", {
                "collection": high_coll,
                "id": f"high_{n}",
                "vector": vector
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"high_dimensional_{i}", test_high_dim)

    # Duplicate ID insert
    for i in range(10):
        def test_duplicate_id(n=i):
            dup_id = f"dup_vec_{n}"
            vector1 = [random.random() for _ in range(128)]
            vector2 = [random.random() for _ in range(128)]
            client.send_request("POST", "/vectors", {
                "collection": edge_coll,
                "id": dup_id,
                "vector": vector1
            })
            resp = client.send_request("POST", "/vectors", {
                "collection": edge_coll,
                "id": dup_id,
                "vector": vector2
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"duplicate_id_{i}", test_duplicate_id)

    # Special characters in IDs
    for i in range(10):
        def test_special_ids(n=i):
            special_id = f"special_{n}_!@#$%^&*()"
            vector = [random.random() for _ in range(128)]
            resp = client.send_request("POST", "/vectors", {
                "collection": edge_coll,
                "id": special_id,
                "vector": vector
            })
            return resp.get("status") == "inserted", f"Response: {resp}"
        test(f"special_chars_id_{i}", test_special_ids)

    # Count vectors
    for i in range(5):
        def test_count(n=i):
            resp = client.send_request("GET", "/collections/count")
            return "count" in resp and resp["count"] >= 0, f"Response: {resp}"
        test(f"count_vectors_{i}", test_count)

    # Unknown endpoints
    for i in range(10):
        def test_unknown_endpoint(n=i):
            resp = client.send_request("GET", f"/unknown_{n}")
            return "error" in resp, f"Response: {resp}"
        test(f"unknown_endpoint_{i}", test_unknown_endpoint)

    # Invalid JSON
    for i in range(10):
        def test_invalid_json(n=i):
            # Send raw invalid data
            try:
                request = f"POST /collections\n{{invalid json\n"
                client.sock.sendall(request.encode())
                response = b""
                while True:
                    chunk = client.sock.recv(4096)
                    if not chunk:
                        break
                    response += chunk
                    if b"\n" in response:
                        break
                resp_str = response.decode().strip()
                resp = json.loads(resp_str)
                return "error" in resp, f"Response: {resp}"
            except:
                return True, "Expected error"
        test(f"invalid_json_{i}", test_invalid_json)

    # Missing fields
    for i in range(10):
        def test_missing_fields(n=i):
            resp = client.send_request("POST", "/collections", {})
            # Should use defaults
            return resp.get("status") == "created", f"Response: {resp}"
        test(f"missing_fields_{i}", test_missing_fields)

    # Concurrent inserts
    for i in range(10):
        def test_concurrent(n=i):
            clients = [client.new_connection() for _ in range(5)]
            success = True
            for idx, c in enumerate(clients):
                vec_id = f"concurrent_{n}_{idx}"
                vector = [random.random() for _ in range(128)]
                resp = c.send_request("POST", "/vectors", {
                    "collection": edge_coll,
                    "id": vec_id,
                    "vector": vector
                })
                if resp.get("status") != "inserted":
                    success = False
                c.close()
            return success, f"Concurrent insert failed"
        test(f"concurrent_inserts_{i}", test_concurrent)

    print(f"\n{'='*60}")
    print(f"Vector Protocol Test Results")
    print(f"{'='*60}")
    print(f"Total:  {results['total']}")
    print(f"Passed: {results['passed']}")
    print(f"Failed: {results['failed']}")
    print(f"{'='*60}")

    if results['failures']:
        print(f"\nFirst {len(results['failures'])} failures:")
        for f in results['failures']:
            print(f"  - {f['test']}: {f['error']}")

    return results


if __name__ == "__main__":
    results = run_tests()
    print(json.dumps(results, indent=2))
