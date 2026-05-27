#!/usr/bin/env python3
"""
Test multiple concurrent MySQL connections to verify per-connection session state
"""
import pymysql
import threading
import time
import sys

def test_connection(conn_id, db_name, errors):
    """Each connection creates its own database and works in it"""
    try:
        conn = pymysql.connect(
            host='127.0.0.1',
            port=9030,
            user='root',
            connect_timeout=10
        )
        cur = conn.cursor()

        # Create and use own database
        cur.execute(f'CREATE DATABASE IF NOT EXISTS {db_name}')
        cur.execute(f'USE {db_name}')

        # Create table
        cur.execute(f'CREATE TABLE IF NOT EXISTS test_table (id INT, value VARCHAR(100))')

        # Insert data specific to this connection
        cur.execute(f"INSERT INTO test_table VALUES (1, 'data_from_{conn_id}')")
        conn.commit()

        # Verify we can only see our own data
        cur.execute('SELECT value FROM test_table WHERE id = 1')
        result = cur.fetchone()

        if result and result[0] == f'data_from_{conn_id}':
            print(f"Connection {conn_id}: ✓ Working correctly")
        else:
            errors.append(f"Connection {conn_id}: Got wrong data: {result}")

        # Cleanup
        cur.execute(f'DROP DATABASE {db_name}')
        conn.commit()

        cur.close()
        conn.close()

    except Exception as e:
        errors.append(f"Connection {conn_id}: {e}")

def main():
    print("Testing multiple concurrent MySQL connections...")
    print("=" * 60)

    errors = []
    threads = []

    # Start 5 concurrent connections
    for i in range(5):
        t = threading.Thread(
            target=test_connection,
            args=(i, f"test_db_{i}", errors)
        )
        threads.append(t)
        t.start()

    # Wait for all threads
    for t in threads:
        t.join()

    print("=" * 60)
    if errors:
        print(f"FAILED: {len(errors)} errors occurred:")
        for err in errors:
            print(f"  - {err}")
        return 1
    else:
        print("SUCCESS: All connections worked independently!")
        return 0

if __name__ == '__main__':
    sys.exit(main())
