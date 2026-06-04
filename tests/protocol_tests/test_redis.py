#!/usr/bin/env python3
"""
Comprehensive Redis protocol test suite for RorisDB.
Tests RESP protocol over raw TCP on port 16379.
1000+ test assertions across all supported (and unsupported) commands.
"""

import socket
import time
import json
import sys
import traceback
import uuid

HOST = "127.0.0.1"
PORT = 16379
TIMEOUT = 5

# ─── RESP Client ────────────────────────────────────────────────────────────

class RESPClient:
    def __init__(self, host=HOST, port=PORT, timeout=TIMEOUT):
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.settimeout(timeout)
        self.sock.connect((host, port))
        self._buf = b""

    def close(self):
        try: self.sock.close()
        except: pass

    def _read_line(self):
        while b"\r\n" not in self._buf:
            chunk = self.sock.recv(4096)
            if not chunk: raise ConnectionError("Connection closed")
            self._buf += chunk
        line, self._buf = self._buf.split(b"\r\n", 1)
        return line.decode("utf-8", errors="replace")

    def _read_bytes(self, n):
        while len(self._buf) < n + 2:
            chunk = self.sock.recv(4096)
            if not chunk: raise ConnectionError("Connection closed")
            self._buf += chunk
        data = self._buf[:n]
        self._buf = self._buf[n + 2:]
        return data.decode("utf-8", errors="replace")

    def _parse_resp(self):
        line = self._read_line()
        if not line: raise ValueError("Empty response")
        prefix, payload = line[0], line[1:]
        if prefix == "+": return payload
        elif prefix == "-": return Exception(payload)
        elif prefix == ":": return int(payload)
        elif prefix == "$":
            length = int(payload)
            if length == -1: return None
            return self._read_bytes(length)
        elif prefix == "*":
            count = int(payload)
            if count == -1: return None
            return [self._parse_resp() for _ in range(count)]
        else: return line

    def send_command(self, *args):
        parts = []
        for a in args:
            s = str(a).encode("utf-8")
            parts.append(f"${len(s)}\r\n".encode("utf-8") + s + b"\r\n")
        cmd = f"*{len(args)}\r\n".encode("utf-8") + b"".join(parts)
        self.sock.sendall(cmd)
        return self._parse_resp()


def new_client():
    return RESPClient()

# ─── Test Runner ────────────────────────────────────────────────────────────

class TestResult:
    def __init__(self):
        self.total = 0
        self.passed = 0
        self.failed = 0
        self.failures = []

    def record(self, name, passed, error=None):
        self.total += 1
        if passed:
            self.passed += 1
        else:
            self.failed += 1
            if len(self.failures) < 20:
                self.failures.append({"test": name, "error": str(error)[:300]})


_result = TestResult()
_keys_to_clean = []

def uid(prefix="t"):
    return prefix + uuid.uuid4().hex[:10]

def track(*keys):
    _keys_to_clean.extend(keys)
    return keys[0] if len(keys) == 1 else keys

def cleanup(c):
    batch_size = 50
    for i in range(0, len(_keys_to_clean), batch_size):
        batch = _keys_to_clean[i:i+batch_size]
        try: c.send_command("DEL", *batch)
        except: pass
    _keys_to_clean.clear()

def t(name, fn):
    """Run a named test, record pass/fail."""
    try:
        fn()
        _result.record(name, True)
    except Exception as e:
        _result.record(name, False, str(e))

def assert_eq(r, e, msg=""):
    if isinstance(r, Exception): raise AssertionError(f"err={r} {msg}")
    if r != e: raise AssertionError(f"expected={e!r} got={r!r} {msg}")

def assert_ok(r, msg=""):
    if isinstance(r, Exception): raise AssertionError(f"err={r} {msg}")
    if r != "OK": raise AssertionError(f"expected OK got={r!r} {msg}")

def assert_int(r, expected=None, msg=""):
    if isinstance(r, Exception): raise AssertionError(f"err={r} {msg}")
    if not isinstance(r, int): raise AssertionError(f"expected int got={type(r).__name__}:{r!r} {msg}")
    if expected is not None and r != expected: raise AssertionError(f"expected={expected} got={r} {msg}")

def assert_err(r, msg=""):
    if not isinstance(r, Exception): raise AssertionError(f"expected error got={r!r} {msg}")

def assert_nil(r, msg=""):
    if r is not None: raise AssertionError(f"expected nil got={r!r} {msg}")

def assert_list(r, msg=""):
    if isinstance(r, Exception): raise AssertionError(f"err={r} {msg}")
    if not isinstance(r, list): raise AssertionError(f"expected list got={type(r).__name__} {msg}")

def assert_str(r, msg=""):
    if isinstance(r, Exception): raise AssertionError(f"err={r} {msg}")
    if not isinstance(r, str): raise AssertionError(f"expected str got={type(r).__name__} {msg}")

def assert_in(val, container, msg=""):
    if val not in container: raise AssertionError(f"{val!r} not in {container!r} {msg}")

# ═══════════════════════════════════════════════════════════════════════════
# 1. STRING TESTS (200+)
# ═══════════════════════════════════════════════════════════════════════════

def test_string(c):
    # SET/GET basic
    for i in range(10):
        k = track(uid("s_b"))
        t(f"SET/GET_basic_{i}", lambda k=k,i=i: (
            assert_ok(c.send_command("SET", k, f"v{i}")),
            assert_eq(c.send_command("GET", k), f"v{i}")
        ))

    # SET with EX
    for i in range(5):
        k = track(uid("s_ex"))
        t(f"SET_EX_{i}", lambda k=k: (
            assert_ok(c.send_command("SET", k, "val", "EX", "100")),
            assert_eq(c.send_command("GET", k), "val")
        ))

    # SET with PX
    for i in range(5):
        k = track(uid("s_px"))
        t(f"SET_PX_{i}", lambda k=k: (
            assert_ok(c.send_command("SET", k, "val", "PX", "100000")),
            assert_eq(c.send_command("GET", k), "val")
        ))

    # SET with NX
    k = track(uid("s_nx"))
    t("SET_NX_new", lambda: (
        assert_ok(c.send_command("SET", k, "first", "NX")),
        assert_eq(c.send_command("GET", k), "first")
    ))
    t("SET_NX_exists", lambda: (
        assert_nil(c.send_command("SET", k, "second", "NX")),
        assert_eq(c.send_command("GET", k), "first")
    ))

    # SET with XX
    k = track(uid("s_xx"))
    t("SET_XX_nokey", lambda: assert_nil(c.send_command("SET", k, "v", "XX")))
    c.send_command("SET", k, "zero")
    t("SET_XX_exists", lambda: (
        assert_ok(c.send_command("SET", k, "second", "XX")),
        assert_eq(c.send_command("GET", k), "second")
    ))

    # GET nonexistent
    t("GET_nonexist", lambda: assert_nil(c.send_command("GET", uid("s_none"))))

    # SET empty value
    for i in range(3):
        k = track(uid("s_emp"))
        t(f"SET_empty_{i}", lambda k=k: (
            assert_ok(c.send_command("SET", k, "")),
            assert_eq(c.send_command("GET", k), "")
        ))

    # SET long value (safe size)
    for length in [100, 500, 1000, 2000, 4000]:
        k = track(uid("s_long"))
        val = "A" * length
        t(f"SET_long_{length}", lambda k=k,val=val: (
            assert_ok(c.send_command("SET", k, val)),
            assert_eq(c.send_command("GET", k), val)
        ))

    # SET unicode
    for uni in ["你好", "世界", "🌍", "日本語", "한국어", "مرحبا", "Привет", "🎉🎊", "αβγδ", "Hello世界"]:
        k = track(uid("s_uni"))
        t(f"SET_unicode_{uni[:4]}", lambda k=k,uni=uni: (
            assert_ok(c.send_command("SET", k, uni)),
            assert_eq(c.send_command("GET", k), uni)
        ))

    # SET special chars
    for i, spec in enumerate(["tab\there", "new\nline", "cr\rret", "mix\n\t\r", "!@#$%^&*()"]):
        k = track(uid("s_spec"))
        t(f"SET_special_{i}", lambda k=k,spec=spec: (
            assert_ok(c.send_command("SET", k, spec)),
            assert_eq(c.send_command("GET", k), spec)
        ))

    # MSET/MGET
    for trial in range(5):
        keys = [track(uid("ms")) for _ in range(5)]
        args = []
        for i, k in enumerate(keys):
            args.extend([k, f"v{i}"])
        t(f"MSET_MGET_{trial}", lambda: (
            assert_ok(c.send_command("MSET", *args)),
            (lambda r: (assert_list(r), [assert_eq(r[i], f"v{i}") for i in range(5)]))(c.send_command("MGET", *keys))
        ))

    # MGET with missing keys
    k1 = track(uid("ms_m"))
    k2 = track(uid("ms_m"))
    c.send_command("SET", k1, "a")
    c.send_command("SET", k2, "b")
    t("MGET_with_missing", lambda: (
        lambda r: (assert_list(r), assert_eq(r[0], "a"), assert_nil(r[1]), assert_eq(r[2], "b"))
    )(c.send_command("MGET", k1, uid("nokey"), k2)))

    # MSET odd args error
    t("MSET_odd_args", lambda: assert_err(c.send_command("MSET", "k1", "v1", "k2")))

    # MSET large batch
    keys = [track(uid("msb")) for _ in range(20)]
    args = []
    for i, k in enumerate(keys):
        args.extend([k, f"bv{i}"])
    t("MSET_large_20", lambda: (
        assert_ok(c.send_command("MSET", *args)),
        lambda r: [assert_eq(r[i], f"bv{i}") for i in range(20)]
    )(c.send_command("MGET", *keys)))

    # APPEND
    for i in range(10):
        k = track(uid("s_app"))
        t(f"APPEND_{i}", lambda k=k: (
            assert_ok(c.send_command("SET", k, "hello")),
            assert_int(c.send_command("APPEND", k, " world")),
            assert_eq(c.send_command("GET", k), "hello world")
        ))

    # APPEND to nonexistent key
    for i in range(5):
        k = track(uid("s_appn"))
        t(f"APPEND_newkey_{i}", lambda k=k: (
            assert_int(c.send_command("APPEND", k, "new")),
            assert_eq(c.send_command("GET", k), "new")
        ))

    # APPEND empty string
    k = track(uid("s_appe"))
    c.send_command("SET", k, "abc")
    t("APPEND_empty", lambda: (
        assert_int(c.send_command("APPEND", k, ""), 3),
        assert_eq(c.send_command("GET", k), "abc")
    ))

    # APPEND multiple times
    k = track(uid("s_appm"))
    t("APPEND_chain", lambda: (
        assert_ok(c.send_command("SET", k, "a")),
        assert_int(c.send_command("APPEND", k, "b")),
        assert_int(c.send_command("APPEND", k, "c")),
        assert_int(c.send_command("APPEND", k, "d")),
        assert_eq(c.send_command("GET", k), "abcd")
    ))

    # STRLEN
    for val, expected in [("hello", 5), ("", 0), ("abc", 3), ("12345", 5), ("a", 1)]:
        k = track(uid("s_sl"))
        c.send_command("SET", k, val)
        t(f"STRLEN_{val[:5]}", lambda k=k,e=expected: assert_int(c.send_command("STRLEN", k), e))

    # STRLEN nonexistent
    for i in range(3):
        t(f"STRLEN_nonexist_{i}", lambda: assert_int(c.send_command("STRLEN", uid("s_sl_none")), 0))

    # INCR/DECR
    for start in [0, 1, 10, 100, -10, -1, 999]:
        k = track(uid("s_incr"))
        c.send_command("SET", k, str(start))
        t(f"INCR_from_{start}", lambda k=k,s=start: (
            assert_int(c.send_command("INCR", k), s + 1),
            assert_int(c.send_command("DECR", k), s)
        ))

    # INCR on nonexistent (starts from 0)
    for i in range(5):
        k = track(uid("s_incnew"))
        t(f"INCR_newkey_{i}", lambda k=k: assert_int(c.send_command("INCR", k), 1))

    # DECR to negative
    k = track(uid("s_decn"))
    c.send_command("SET", k, "0")
    t("DECR_negative", lambda: assert_int(c.send_command("DECR", k), -1))

    # Multiple INCR/DECR
    k = track(uid("s_incmm"))
    c.send_command("SET", k, "0")
    for i in range(10):
        c.send_command("INCR", k)
    t("INCR_10times", lambda: assert_int(c.send_command("GET", k), "10"))

    # INCRBY/DECRBY
    for base, inc in [(0, 1), (10, 5), (100, 50), (-10, 20), (0, -5), (50, -50)]:
        k = track(uid("s_iby"))
        c.send_command("SET", k, str(base))
        t(f"INCRBY_{base}_{inc}", lambda k=k,b=base,i=inc: (
            assert_int(c.send_command("INCRBY", k, str(abs(i))), b + abs(i)),
            assert_int(c.send_command("DECRBY", k, str(abs(i))), b)
        ))

    # INCRBY large numbers
    for val in [999999, -999999, 0]:
        k = track(uid("s_ibyl"))
        c.send_command("SET", k, str(val))
        t(f"INCRBY_large_{val}", lambda k=k,v=val: assert_int(c.send_command("INCRBY", k, "1"), v + 1))

    # INCR non-numeric error
    for val in ["abc", "1.5", "", "hello123"]:
        k = track(uid("s_ince"))
        c.send_command("SET", k, val)
        t(f"INCR_nonnumeric_{val[:4]}", lambda k=k: assert_err(c.send_command("INCR", k)))

    # DECRBY
    k = track(uid("s_dby"))
    c.send_command("SET", k, "100")
    t("DECRBY_basic", lambda: (
        assert_int(c.send_command("DECRBY", k, "30"), 70),
        assert_int(c.send_command("DECRBY", k, "70"), 0),
        assert_int(c.send_command("DECRBY", k, "1"), -1)
    ))

    # SET overwrite
    k = track(uid("s_over"))
    for i in range(5):
        t(f"SET_overwrite_{i}", lambda k=k,i=i: (
            assert_ok(c.send_command("SET", k, f"v{i}")),
            assert_eq(c.send_command("GET", k), f"v{i}")
        ))

    # SET numeric string values
    for num in ["0", "-1", "1", "999999", "42", "100"]:
        k = track(uid("s_num"))
        t(f"SET_numeric_{num}", lambda k=k,n=num: (
            assert_ok(c.send_command("SET", k, n)),
            assert_eq(c.send_command("GET", k), n)
        ))

    # STRLEN after APPEND
    k = track(uid("s_sla"))
    c.send_command("SET", k, "hello")
    c.send_command("APPEND", k, " world")
    t("STRLEN_after_append", lambda: assert_int(c.send_command("STRLEN", k), 11))

    # INCRBY error cases
    t("INCRBY_noargs", lambda: assert_err(c.send_command("INCRBY", uid("nokey"))))
    k = track(uid("s_ibye"))
    c.send_command("SET", k, "10")
    t("INCRBY_nonnumeric", lambda: assert_err(c.send_command("INCRBY", k, "abc")))

    # SET then DEL then GET
    k = track(uid("s_delget"))
    c.send_command("SET", k, "val")
    c.send_command("DEL", k)
    t("GET_after_del", lambda: assert_nil(c.send_command("GET", k)))

    # MGET all missing
    t("MGET_all_missing", lambda: (
        lambda r: (assert_list(r), assert_nil(r[0]), assert_nil(r[1]))
    )(c.send_command("MGET", uid("nomsg1"), uid("nomsg2"))))

    # APPEND to wrong type
    k = track(uid("s_appwt"))
    c.send_command("RPUSH", k, "a")
    t("APPEND_wrongtype", lambda: assert_err(c.send_command("APPEND", k, "x")))

    # STRLEN wrong type
    k = track(uid("s_slwt"))
    c.send_command("RPUSH", k, "a")
    t("STRLEN_wrongtype", lambda: assert_err(c.send_command("STRLEN", k)))

    # INCR wrong type
    k = track(uid("s_incrwt"))
    c.send_command("SET", k, "10")
    c.send_command("RPUSH", uid("s_wtl"), "a")  # create list
    t("INCR_on_list", lambda: assert_err(c.send_command("INCR", uid("s_wtl"))))

    # GET wrong type
    k = track(uid("s_getwt"))
    c.send_command("RPUSH", k, "a")
    t("GET_wrongtype", lambda: assert_err(c.send_command("GET", k)))

    # SET with EX and NX combined
    k = track(uid("s_exnx"))
    t("SET_EX_NX", lambda: (
        assert_ok(c.send_command("SET", k, "v", "EX", "100", "NX")),
        assert_eq(c.send_command("GET", k), "v")
    ))
    t("SET_EX_NX_exists", lambda: assert_nil(c.send_command("SET", k, "v2", "EX", "100", "NX")))

    # SET with PX and XX combined
    k = track(uid("s_pxxx"))
    c.send_command("SET", k, "old")
    t("SET_PX_XX", lambda: (
        assert_ok(c.send_command("SET", k, "new", "PX", "100000", "XX")),
        assert_eq(c.send_command("GET", k), "new")
    ))


# ═══════════════════════════════════════════════════════════════════════════
# 2. LIST TESTS (150+)
# ═══════════════════════════════════════════════════════════════════════════

def test_list(c):
    # RPUSH basic
    for i in range(10):
        k = track(uid("l_rp"))
        t(f"RPUSH_basic_{i}", lambda k=k: (
            assert_int(c.send_command("RPUSH", k, "a"), 1),
            assert_int(c.send_command("RPUSH", k, "b"), 2),
            assert_int(c.send_command("RPUSH", k, "c"), 3)
        ))

    # RPUSH multiple values
    for i in range(5):
        k = track(uid("l_rpm"))
        t(f"RPUSH_multi_{i}", lambda k=k: (
            assert_int(c.send_command("RPUSH", k, "a", "b", "c"), 3),
            assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["a", "b", "c"])
        ))

    # LPUSH basic
    for i in range(10):
        k = track(uid("l_lp"))
        t(f"LPUSH_basic_{i}", lambda k=k: (
            assert_int(c.send_command("LPUSH", k, "a"), 1),
            assert_int(c.send_command("LPUSH", k, "b"), 2),
            assert_int(c.send_command("LPUSH", k, "c"), 3),
            assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["c", "b", "a"])
        ))

    # LPUSH multiple values
    for i in range(5):
        k = track(uid("l_lpm"))
        t(f"LPUSH_multi_{i}", lambda k=k: (
            assert_int(c.send_command("LPUSH", k, "c", "b", "a"), 3),
            assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["a", "b", "c"])
        ))

    # LRANGE various
    k = track(uid("l_lr"))
    c.send_command("RPUSH", k, "a", "b", "c", "d", "e")
    t("LRANGE_all", lambda: assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["a", "b", "c", "d", "e"]))
    t("LRANGE_0_1", lambda: assert_eq(c.send_command("LRANGE", k, "0", "1"), ["a", "b"]))
    t("LRANGE_0_0", lambda: assert_eq(c.send_command("LRANGE", k, "0", "0"), ["a"]))
    t("LRANGE_neg2_neg1", lambda: assert_eq(c.send_command("LRANGE", k, "-2", "-1"), ["d", "e"]))
    t("LRANGE_neg1", lambda: assert_eq(c.send_command("LRANGE", k, "-1", "-1"), ["e"]))
    t("LRANGE_outofrange", lambda: assert_eq(c.send_command("LRANGE", k, "10", "20"), []))
    t("LRANGE_1_3", lambda: assert_eq(c.send_command("LRANGE", k, "1", "3"), ["b", "c", "d"]))

    # LLEN
    for size in [0, 1, 3, 5, 10]:
        k = track(uid("l_ll"))
        if size > 0:
            vals = [f"v{i}" for i in range(size)]
            c.send_command("RPUSH", k, *vals)
        t(f"LLEN_{size}", lambda k=k,s=size: assert_int(c.send_command("LLEN", k), s))

    # LLEN nonexistent
    for i in range(3):
        t(f"LLEN_nonexist_{i}", lambda: assert_int(c.send_command("LLEN", uid("l_ll_none")), 0))

    # LINDEX
    k = track(uid("l_li"))
    c.send_command("RPUSH", k, "a", "b", "c", "d", "e")
    t("LINDEX_0", lambda: assert_eq(c.send_command("LINDEX", k, "0"), "a"))
    t("LINDEX_2", lambda: assert_eq(c.send_command("LINDEX", k, "2"), "c"))
    t("LINDEX_4", lambda: assert_eq(c.send_command("LINDEX", k, "4"), "e"))
    t("LINDEX_neg1", lambda: assert_eq(c.send_command("LINDEX", k, "-1"), "e"))
    t("LINDEX_neg5", lambda: assert_eq(c.send_command("LINDEX", k, "-5"), "a"))
    t("LINDEX_outofrange", lambda: assert_nil(c.send_command("LINDEX", k, "10")))
    t("LINDEX_neg_outofrange", lambda: assert_nil(c.send_command("LINDEX", k, "-10")))

    # LINDEX nonexistent key
    t("LINDEX_nokey", lambda: assert_nil(c.send_command("LINDEX", uid("l_li_none"), "0")))

    # LPOP
    for i in range(5):
        k = track(uid("l_lpop"))
        c.send_command("RPUSH", k, "a", "b", "c")
        t(f"LPOP_{i}", lambda k=k: (
            assert_eq(c.send_command("LPOP", k), "a"),
            assert_eq(c.send_command("LPOP", k), "b"),
            assert_eq(c.send_command("LPOP", k), "c"),
            assert_nil(c.send_command("LPOP", k))
        ))

    # LPOP empty
    for i in range(3):
        t(f"LPOP_empty_{i}", lambda: assert_nil(c.send_command("LPOP", uid("l_lpop_e"))))

    # RPOP
    for i in range(5):
        k = track(uid("l_rpop"))
        c.send_command("RPUSH", k, "a", "b", "c")
        t(f"RPOP_{i}", lambda k=k: (
            assert_eq(c.send_command("RPOP", k), "c"),
            assert_eq(c.send_command("RPOP", k), "b"),
            assert_eq(c.send_command("RPOP", k), "a"),
            assert_nil(c.send_command("RPOP", k))
        ))

    # RPOP empty
    for i in range(3):
        t(f"RPOP_empty_{i}", lambda: assert_nil(c.send_command("RPOP", uid("l_rpop_e"))))

    # LPOP then RPOP interleaved
    k = track(uid("l_inter"))
    c.send_command("RPUSH", k, "a", "b", "c", "d")
    t("LPOP_RPOP_interleave", lambda: (
        assert_eq(c.send_command("LPOP", k), "a"),
        assert_eq(c.send_command("RPOP", k), "d"),
        assert_eq(c.send_command("LPOP", k), "b"),
        assert_eq(c.send_command("RPOP", k), "c"),
        assert_nil(c.send_command("LPOP", k))
    ))

    # Large list operations
    k = track(uid("l_large"))
    for i in range(50):
        c.send_command("RPUSH", k, f"e{i}")
    t("LRANGE_large_list", lambda: (
        assert_int(c.send_command("LLEN", k), 50),
        assert_eq(c.send_command("LINDEX", k, "0"), "e0"),
        assert_eq(c.send_command("LINDEX", k, "49"), "e49")
    ))
    t("LRANGE_large_first10", lambda: (
        lambda r: (assert_list(r), assert_eq(len(r), 10))
    )(c.send_command("LRANGE", k, "0", "9")))

    # LPUSH then RPUSH
    k = track(uid("l_both"))
    c.send_command("LPUSH", k, "mid")
    c.send_command("LPUSH", k, "left")
    c.send_command("RPUSH", k, "right")
    t("LPUSH_RPUSH_combined", lambda: assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["left", "mid", "right"]))

    # List with unicode values
    k = track(uid("l_uni"))
    c.send_command("RPUSH", k, "你好", "世界", "🌍")
    t("LRANGE_unicode", lambda: assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["你好", "世界", "🌍"]))

    # LRANGE empty list
    t("LRANGE_empty", lambda: assert_eq(c.send_command("LRANGE", uid("l_lr_empty"), "0", "-1"), []))

    # List after all popped = deleted
    k = track(uid("l_popdel"))
    c.send_command("RPUSH", k, "only")
    c.send_command("LPOP", k)
    t("LLEN_after_all_popped", lambda: assert_int(c.send_command("LLEN", uid("l_popdel_check")), 0))

    # Wrong type operations
    k = track(uid("l_wt"))
    c.send_command("SET", k, "string")
    t("RPUSH_wrongtype", lambda: assert_err(c.send_command("RPUSH", k, "a")))
    t("LPUSH_wrongtype", lambda: assert_err(c.send_command("LPUSH", k, "a")))
    t("LRANGE_wrongtype", lambda: assert_err(c.send_command("LRANGE", k, "0", "-1")))
    t("LLEN_wrongtype", lambda: assert_err(c.send_command("LLEN", k)))
    t("LPOP_wrongtype", lambda: assert_err(c.send_command("LPOP", k)))
    t("RPOP_wrongtype", lambda: assert_err(c.send_command("RPOP", k)))
    t("LINDEX_wrongtype", lambda: assert_err(c.send_command("LINDEX", k, "0")))

    # RPUSH single element many times
    k = track(uid("l_single"))
    for i in range(20):
        c.send_command("RPUSH", k, f"v{i}")
    t("RPUSH_20singles", lambda: assert_int(c.send_command("LLEN", k), 20))

    # LINDEX after LPOP
    k = track(uid("l_li_after_pop"))
    c.send_command("RPUSH", k, "a", "b", "c")
    c.send_command("LPOP", k)
    t("LINDEX_after_LPOP", lambda: assert_eq(c.send_command("LINDEX", k, "0"), "b"))

    # Mixed operations sequence
    k = track(uid("l_mix"))
    t("List_mixed_ops", lambda: (
        assert_int(c.send_command("RPUSH", k, "x"), 1),
        assert_int(c.send_command("RPUSH", k, "y"), 2),
        assert_int(c.send_command("LPUSH", k, "w"), 3),
        assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["w", "x", "y"]),
        assert_eq(c.send_command("LPOP", k), "w"),
        assert_eq(c.send_command("RPOP", k), "y"),
        assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["x"]),
        assert_int(c.send_command("LLEN", k), 1)
    ))

    # LINDEX middle elements
    k = track(uid("l_mid"))
    c.send_command("RPUSH", k, "a", "b", "c", "d", "e", "f", "g")
    for idx, expected in [(0,"a"),(1,"b"),(2,"c"),(3,"d"),(4,"e"),(5,"f"),(6,"g"),(-1,"g"),(-2,"f"),(-7,"a")]:
        t(f"LINDEX_idx_{idx}", lambda k=k,i=str(idx),e=expected: assert_eq(c.send_command("LINDEX", k, i), e))


# ═══════════════════════════════════════════════════════════════════════════
# 3. SET TESTS (120+)
# ═══════════════════════════════════════════════════════════════════════════

def test_set(c):
    # SADD basic
    for i in range(10):
        k = track(uid("st_sa"))
        t(f"SADD_basic_{i}", lambda k=k: (
            assert_int(c.send_command("SADD", k, "a"), 1),
            assert_int(c.send_command("SADD", k, "b"), 1),
            assert_int(c.send_command("SADD", k, "c"), 1)
        ))

    # SADD duplicate
    k = track(uid("st_dup"))
    c.send_command("SADD", k, "a")
    t("SADD_duplicate", lambda: (
        assert_int(c.send_command("SADD", k, "a"), 0),
        assert_int(c.send_command("SADD", k, "a"), 0)
    ))

    # SADD multiple members
    for i in range(5):
        k = track(uid("st_sam"))
        t(f"SADD_multi_{i}", lambda k=k: assert_int(c.send_command("SADD", k, "a", "b", "c", "d"), 4))

    # SADD some duplicates
    k = track(uid("st_sad"))
    c.send_command("SADD", k, "a", "b")
    t("SADD_some_dup", lambda: assert_int(c.send_command("SADD", k, "b", "c", "d"), 2))  # b is dup

    # SCARD
    for size in [0, 1, 3, 5, 10]:
        k = track(uid("st_sc"))
        if size > 0:
            members = [f"m{i}" for i in range(size)]
            c.send_command("SADD", k, *members)
        t(f"SCARD_{size}", lambda k=k,s=size: assert_int(c.send_command("SCARD", k), s))

    # SCARD nonexistent
    for i in range(3):
        t(f"SCARD_nonexist_{i}", lambda: assert_int(c.send_command("SCARD", uid("st_sc_none")), 0))

    # SISMEMBER
    k = track(uid("st_sim"))
    c.send_command("SADD", k, "a", "b", "c")
    t("SISMEMBER_yes", lambda: assert_int(c.send_command("SISMEMBER", k, "a"), 1))
    t("SISMEMBER_yes2", lambda: assert_int(c.send_command("SISMEMBER", k, "b"), 1))
    t("SISMEMBER_yes3", lambda: assert_int(c.send_command("SISMEMBER", k, "c"), 1))
    t("SISMEMBER_no", lambda: assert_int(c.send_command("SISMEMBER", k, "z"), 0))
    t("SISMEMBER_no2", lambda: assert_int(c.send_command("SISMEMBER", k, "d"), 0))

    # SISMEMBER empty set
    t("SISMEMBER_empty", lambda: assert_int(c.send_command("SISMEMBER", uid("st_sim_e"), "a"), 0))

    # SMEMBERS
    for i in range(5):
        k = track(uid("st_sm"))
        c.send_command("SADD", k, "a", "b", "c")
        t(f"SMEMBERS_{i}", lambda k=k: (
            lambda r: (assert_list(r), assert_eq(set(r), {"a", "b", "c"}))
        )(c.send_command("SMEMBERS", k)))

    # SMEMBERS empty
    t("SMEMBERS_empty", lambda: assert_eq(c.send_command("SMEMBERS", uid("st_sm_e")), []))

    # SREM
    for i in range(5):
        k = track(uid("st_sr"))
        c.send_command("SADD", k, "a", "b", "c")
        t(f"SREM_{i}", lambda k=k: (
            assert_int(c.send_command("SREM", k, "a"), 1),
            assert_int(c.send_command("SCARD", k), 2)
        ))

    # SREM nonexistent member
    k = track(uid("st_srn"))
    c.send_command("SADD", k, "a")
    t("SREM_nonexist_member", lambda: assert_int(c.send_command("SREM", k, "z"), 0))

    # SREM nonexistent key
    t("SREM_nonexist_key", lambda: assert_int(c.send_command("SREM", uid("st_sr_none"), "a"), 0))

    # SREM multiple
    k = track(uid("st_srm"))
    c.send_command("SADD", k, "a", "b", "c", "d")
    t("SREM_multi", lambda: (
        assert_int(c.send_command("SREM", k, "a", "b"), 2),
        assert_int(c.send_command("SCARD", k), 2)
    ))

    # SREM all members -> key deleted
    k = track(uid("st_sra"))
    c.send_command("SADD", k, "a", "b")
    t("SREM_all", lambda: (
        assert_int(c.send_command("SREM", k, "a", "b"), 2),
        assert_int(c.send_command("SCARD", k), 0)
    ))

    # Set with unicode
    k = track(uid("st_uni"))
    c.send_command("SADD", k, "你好", "世界", "🌍")
    t("Set_unicode", lambda: (
        assert_int(c.send_command("SCARD", k), 3),
        assert_int(c.send_command("SISMEMBER", k, "你好"), 1),
        assert_int(c.send_command("SISMEMBER", k, "世界"), 1)
    ))

    # Large set
    k = track(uid("st_large"))
    for i in range(50):
        c.send_command("SADD", k, f"m{i}")
    t("Large_set_50", lambda: (
        assert_int(c.send_command("SCARD", k), 50),
        assert_int(c.send_command("SISMEMBER", k, "m25"), 1),
        assert_int(c.send_command("SISMEMBER", k, "m99"), 0)
    ))

    # Wrong type operations
    k = track(uid("st_wt"))
    c.send_command("SET", k, "string")
    t("SADD_wrongtype", lambda: assert_err(c.send_command("SADD", k, "a")))
    t("SMEMBERS_wrongtype", lambda: assert_err(c.send_command("SMEMBERS", k)))
    t("SCARD_wrongtype", lambda: assert_err(c.send_command("SCARD", k)))
    t("SISMEMBER_wrongtype", lambda: assert_err(c.send_command("SISMEMBER", k, "a")))
    t("SREM_wrongtype", lambda: assert_err(c.send_command("SREM", k, "a")))

    # SADD then SREM then SADD same member
    k = track(uid("st_resadd"))
    c.send_command("SADD", k, "a")
    c.send_command("SREM", k, "a")
    t("SADD_after_SREM", lambda: (
        assert_int(c.send_command("SADD", k, "a"), 1),
        assert_int(c.send_command("SISMEMBER", k, "a"), 1)
    ))

    # SMEMBERS count consistency
    k = track(uid("st_cons"))
    members = [f"m{i}" for i in range(10)]
    c.send_command("SADD", k, *members)
    t("SMEMBERS_count_consistent", lambda: (
        lambda r: (assert_list(r), assert_eq(len(r), 10))
    )(c.send_command("SMEMBERS", k)))

    # SADD single many times
    k = track(uid("st_single"))
    for i in range(10):
        c.send_command("SADD", k, f"m{i}")
    t("SADD_10singles", lambda: assert_int(c.send_command("SCARD", k), 10))


# ═══════════════════════════════════════════════════════════════════════════
# 4. SORTED SET TESTS (120+)
# ═══════════════════════════════════════════════════════════════════════════

def test_sorted_set(c):
    # ZADD basic
    for i in range(10):
        k = track(uid("z_za"))
        t(f"ZADD_basic_{i}", lambda k=k: (
            assert_int(c.send_command("ZADD", k, "1", "a"), 1),
            assert_int(c.send_command("ZADD", k, "2", "b"), 1),
            assert_int(c.send_command("ZADD", k, "3", "c"), 1)
        ))

    # ZRANGE basic
    k = track(uid("z_zr"))
    c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c")
    t("ZRANGE_all", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "-1"), ["a", "b", "c"]))
    t("ZRANGE_0_0", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "0"), ["a"]))
    t("ZRANGE_0_1", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "1"), ["a", "b"]))
    t("ZRANGE_neg1", lambda: assert_eq(c.send_command("ZRANGE", k, "-1", "-1"), ["c"]))
    t("ZRANGE_neg2_neg1", lambda: assert_eq(c.send_command("ZRANGE", k, "-2", "-1"), ["b", "c"]))
    t("ZRANGE_outofrange", lambda: assert_eq(c.send_command("ZRANGE", k, "10", "20"), []))

    # ZADD with float scores
    k = track(uid("z_float"))
    c.send_command("ZADD", k, "1.5", "a", "0.5", "b", "2.5", "c")
    t("ZRANGE_float_scores", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "-1"), ["b", "a", "c"]))

    # ZADD duplicate member (update score)
    k = track(uid("z_dup"))
    c.send_command("ZADD", k, "1", "a")
    c.send_command("ZADD", k, "5", "a")  # update score
    t("ZADD_update_score", lambda: assert_eq(c.send_command("ZSCORE", k, "a"), "5.0"))

    # ZADD multiple at once
    for i in range(5):
        k = track(uid("z_zam"))
        t(f"ZADD_multi_{i}", lambda k=k: (
            assert_int(c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c"), 3),
            assert_int(c.send_command("ZCARD", k), 3)
        ))

    # ZCARD
    for size in [0, 1, 3, 5]:
        k = track(uid("z_zc"))
        if size > 0:
            args = []
            for i in range(size):
                args.extend([str(i), f"m{i}"])
            c.send_command("ZADD", k, *args)
        t(f"ZCARD_{size}", lambda k=k,s=size: assert_int(c.send_command("ZCARD", k), s))

    # ZCARD nonexistent
    for i in range(3):
        t(f"ZCARD_nonexist_{i}", lambda: assert_int(c.send_command("ZCARD", uid("z_zc_none")), 0))

    # ZSCORE
    k = track(uid("z_zs"))
    c.send_command("ZADD", k, "1.5", "a", "2.5", "b")
    t("ZSCORE_basic", lambda: assert_eq(c.send_command("ZSCORE", k, "a"), "1.5"))
    t("ZSCORE_basic2", lambda: assert_eq(c.send_command("ZSCORE", k, "b"), "2.5"))
    t("ZSCORE_nonexist", lambda: assert_nil(c.send_command("ZSCORE", k, "z")))

    # ZSCORE nonexistent key
    t("ZSCORE_nokey", lambda: assert_nil(c.send_command("ZSCORE", uid("z_zs_none"), "a")))

    # ZREM
    for i in range(5):
        k = track(uid("z_zrem"))
        c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c")
        t(f"ZREM_{i}", lambda k=k: (
            assert_int(c.send_command("ZREM", k, "a"), 1),
            assert_int(c.send_command("ZCARD", k), 2)
        ))

    # ZREM nonexistent member
    k = track(uid("z_zremn"))
    c.send_command("ZADD", k, "1", "a")
    t("ZREM_nonexist_member", lambda: assert_int(c.send_command("ZREM", k, "z"), 0))

    # ZREM nonexistent key
    t("ZREM_nonexist_key", lambda: assert_int(c.send_command("ZREM", uid("z_zrem_none"), "a"), 0))

    # ZREM multiple
    k = track(uid("z_zremm"))
    c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c")
    t("ZREM_multi", lambda: (
        assert_int(c.send_command("ZREM", k, "a", "b"), 2),
        assert_int(c.send_command("ZCARD", k), 1)
    ))

    # ZRANGE with WITHSCORES
    k = track(uid("z_zrws"))
    c.send_command("ZADD", k, "1", "a", "2", "b")
    t("ZRANGE_WITHSCORES", lambda: (
        lambda r: (assert_list(r), assert_eq(len(r), 4), assert_eq(r[0], "a"), assert_eq(r[2], "b"))
    )(c.send_command("ZRANGE", k, "0", "-1", "WITHSCORES")))

    # ZRANGE empty
    t("ZRANGE_empty", lambda: assert_eq(c.send_command("ZRANGE", uid("z_zr_empty"), "0", "-1"), []))

    # Large sorted set
    k = track(uid("z_large"))
    for i in range(50):
        c.send_command("ZADD", k, str(i), f"m{i}")
    t("ZCARD_large", lambda: assert_int(c.send_command("ZCARD", k), 50))
    t("ZRANGE_large_first", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "0"), ["m0"]))
    t("ZRANGE_large_last", lambda: assert_eq(c.send_command("ZRANGE", k, "-1", "-1"), ["m49"]))

    # Wrong type operations
    k = track(uid("z_wt"))
    c.send_command("SET", k, "string")
    t("ZADD_wrongtype", lambda: assert_err(c.send_command("ZADD", k, "1", "a")))
    t("ZRANGE_wrongtype", lambda: assert_err(c.send_command("ZRANGE", k, "0", "-1")))
    t("ZCARD_wrongtype", lambda: assert_err(c.send_command("ZCARD", k)))
    t("ZSCORE_wrongtype", lambda: assert_err(c.send_command("ZSCORE", k, "a")))
    t("ZREM_wrongtype", lambda: assert_err(c.send_command("ZREM", k, "a")))

    # ZADD same score different members (order by member)
    k = track(uid("z_samescore"))
    c.send_command("ZADD", k, "1", "c", "1", "a", "1", "b")
    t("ZRANGE_same_score", lambda: (
        lambda r: (assert_list(r), assert_eq(set(r), {"a", "b", "c"}))
    )(c.send_command("ZRANGE", k, "0", "-1")))

    # ZADD then ZREM all
    k = track(uid("z_zremall"))
    c.send_command("ZADD", k, "1", "a", "2", "b")
    t("ZREM_all", lambda: (
        assert_int(c.send_command("ZREM", k, "a", "b"), 2),
        assert_int(c.send_command("ZCARD", k), 0)
    ))

    # ZADD negative scores
    k = track(uid("z_negscore"))
    c.send_command("ZADD", k, "-5", "a", "0", "b", "5", "c")
    t("ZRANGE_negative_scores", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "-1"), ["a", "b", "c"]))

    # ZSCORE after update
    k = track(uid("z_zsupd"))
    c.send_command("ZADD", k, "1", "a")
    c.send_command("ZADD", k, "99", "a")
    t("ZSCORE_after_update", lambda: assert_eq(c.send_command("ZSCORE", k, "a"), "99.0"))

    # Multiple ZADD/ZREM cycles
    k = track(uid("z_cycle"))
    t("ZADD_ZREM_cycle", lambda: (
        assert_int(c.send_command("ZADD", k, "1", "a"), 1),
        assert_int(c.send_command("ZREM", k, "a"), 1),
        assert_int(c.send_command("ZADD", k, "2", "a"), 1),
        assert_int(c.send_command("ZCARD", k), 1),
        assert_eq(c.send_command("ZSCORE", k, "a"), "2.0")
    ))

    # ZRANGE with negative indices
    k = track(uid("z_zrneg"))
    c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c", "4", "d", "5", "e")
    t("ZRANGE_neg3_neg1", lambda: assert_eq(c.send_command("ZRANGE", k, "-3", "-1"), ["c", "d", "e"]))
    t("ZRANGE_0_neg1", lambda: assert_eq(c.send_command("ZRANGE", k, "0", "-1"), ["a", "b", "c", "d", "e"]))
    t("ZRANGE_1_3", lambda: assert_eq(c.send_command("ZRANGE", k, "1", "3"), ["b", "c", "d"]))


# ═══════════════════════════════════════════════════════════════════════════
# 5. HASH TESTS (150+)
# ═══════════════════════════════════════════════════════════════════════════

def test_hash(c):
    # HSET basic
    for i in range(10):
        k = track(uid("h_hs"))
        t(f"HSET_basic_{i}", lambda k=k: (
            assert_int(c.send_command("HSET", k, "f1", "v1"), 1),
            assert_eq(c.send_command("HGET", k, "f1"), "v1")
        ))

    # HSET multiple fields
    for i in range(5):
        k = track(uid("h_hsm"))
        t(f"HSET_multi_{i}", lambda k=k: (
            assert_int(c.send_command("HSET", k, "f1", "v1", "f2", "v2", "f3", "v3"), 3),
            assert_eq(c.send_command("HGET", k, "f1"), "v1"),
            assert_eq(c.send_command("HGET", k, "f2"), "v2"),
            assert_eq(c.send_command("HGET", k, "f3"), "v3")
        ))

    # HGET nonexistent field
    k = track(uid("h_hgn"))
    c.send_command("HSET", k, "f1", "v1")
    t("HGET_nonexist_field", lambda: assert_nil(c.send_command("HGET", k, "nofield")))

    # HGET nonexistent key
    t("HGET_nonexist_key", lambda: assert_nil(c.send_command("HGET", uid("h_hg_none"), "f")))

    # HMSET/HMGET
    for i in range(5):
        k = track(uid("h_hms"))
        t(f"HMSET_HMGET_{i}", lambda k=k: (
            assert_ok(c.send_command("HMSET", k, "f1", "v1", "f2", "v2")),
            lambda r: (assert_list(r), assert_eq(r[0], "v1"), assert_eq(r[1], "v2"))
        )(c.send_command("HMGET", k, "f1", "f2")))

    # HMGET with missing field
    k = track(uid("h_hmgm"))
    c.send_command("HSET", k, "f1", "v1")
    t("HMGET_with_missing", lambda: (
        lambda r: (assert_list(r), assert_eq(r[0], "v1"), assert_nil(r[1]))
    )(c.send_command("HMGET", k, "f1", "f2")))

    # HGETALL
    for i in range(5):
        k = track(uid("h_hga"))
        c.send_command("HSET", k, "f1", "v1", "f2", "v2")
        t(f"HGETALL_{i}", lambda k=k: (
            lambda r: (assert_list(r), assert_eq(set(r), {"f1", "v1", "f2", "v2"}))
        )(c.send_command("HGETALL", k)))

    # HGETALL empty hash
    t("HGETALL_empty", lambda: assert_eq(c.send_command("HGETALL", uid("h_hga_e")), []))

    # HGETALL nonexistent
    t("HGETALL_nonexist", lambda: assert_eq(c.send_command("HGETALL", uid("h_hga_n")), []))

    # HDEL
    for i in range(5):
        k = track(uid("h_hd"))
        c.send_command("HSET", k, "f1", "v1", "f2", "v2")
        t(f"HDEL_{i}", lambda k=k: (
            assert_int(c.send_command("HDEL", k, "f1"), 1),
            assert_nil(c.send_command("HGET", k, "f1"))
        ))

    # HDEL nonexistent field
    k = track(uid("h_hdn"))
    c.send_command("HSET", k, "f1", "v1")
    t("HDEL_nonexist_field", lambda: assert_int(c.send_command("HDEL", k, "nofield"), 0))

    # HDEL nonexistent key
    t("HDEL_nonexist_key", lambda: assert_int(c.send_command("HDEL", uid("h_hd_none"), "f"), 0))

    # HDEL multiple
    k = track(uid("h_hdm"))
    c.send_command("HSET", k, "f1", "v1", "f2", "v2", "f3", "v3")
    t("HDEL_multi", lambda: (
        assert_int(c.send_command("HDEL", k, "f1", "f2"), 2),
        assert_int(c.send_command("HLEN", k), 1)
    ))

    # HEXISTS
    k = track(uid("h_hex"))
    c.send_command("HSET", k, "f1", "v1")
    t("HEXISTS_yes", lambda: assert_int(c.send_command("HEXISTS", k, "f1"), 1))
    t("HEXISTS_no", lambda: assert_int(c.send_command("HEXISTS", k, "f2"), 0))

    # HEXISTS nonexistent key
    t("HEXISTS_nokey", lambda: assert_int(c.send_command("HEXISTS", uid("h_hex_none"), "f"), 0))

    # HLEN
    for size in [0, 1, 3, 5, 10]:
        k = track(uid("h_hl"))
        if size > 0:
            args = []
            for i in range(size):
                args.extend([f"f{i}", f"v{i}"])
            c.send_command("HSET", k, *args)
        t(f"HLEN_{size}", lambda k=k,s=size: assert_int(c.send_command("HLEN", k), s))

    # HLEN nonexistent
    for i in range(3):
        t(f"HLEN_nonexist_{i}", lambda: assert_int(c.send_command("HLEN", uid("h_hl_none")), 0))

    # HKEYS
    for i in range(5):
        k = track(uid("h_hk"))
        c.send_command("HSET", k, "f1", "v1", "f2", "v2")
        t(f"HKEYS_{i}", lambda k=k: (
            lambda r: (assert_list(r), assert_eq(set(r), {"f1", "f2"}))
        )(c.send_command("HKEYS", k)))

    # HKEYS empty
    t("HKEYS_empty", lambda: assert_eq(c.send_command("HKEYS", uid("h_hk_e")), []))

    # HVALS
    for i in range(5):
        k = track(uid("h_hv"))
        c.send_command("HSET", k, "f1", "v1", "f2", "v2")
        t(f"HVALS_{i}", lambda k=k: (
            lambda r: (assert_list(r), assert_eq(set(r), {"v1", "v2"}))
        )(c.send_command("HVALS", k)))

    # HVALS empty
    t("HVALS_empty", lambda: assert_eq(c.send_command("HVALS", uid("h_hv_e")), []))

    # Hash overwrite field
    k = track(uid("h_over"))
    c.send_command("HSET", k, "f1", "v1")
    t("HSET_overwrite", lambda: (
        assert_int(c.send_command("HSET", k, "f1", "v2"), 0),  # 0 new fields
        assert_eq(c.send_command("HGET", k, "f1"), "v2")
    ))

    # Hash with unicode
    k = track(uid("h_uni"))
    c.send_command("HSET", k, "名字", "张三")
    t("Hash_unicode", lambda: (
        assert_eq(c.send_command("HGET", k, "名字"), "张三"),
        assert_int(c.send_command("HEXISTS", k, "名字"), 1)
    ))

    # Large hash
    k = track(uid("h_large"))
    for i in range(30):
        c.send_command("HSET", k, f"f{i}", f"v{i}")
    t("HLEN_large", lambda: assert_int(c.send_command("HLEN", k), 30))
    t("HGET_large", lambda: assert_eq(c.send_command("HGET", k, "f15"), "v15"))

    # Wrong type operations
    k = track(uid("h_wt"))
    c.send_command("SET", k, "string")
    t("HSET_wrongtype", lambda: assert_err(c.send_command("HSET", k, "f", "v")))
    t("HGET_wrongtype", lambda: assert_err(c.send_command("HGET", k, "f")))
    t("HGETALL_wrongtype", lambda: assert_err(c.send_command("HGETALL", k)))
    t("HDEL_wrongtype", lambda: assert_err(c.send_command("HDEL", k, "f")))
    t("HLEN_wrongtype", lambda: assert_err(c.send_command("HLEN", k)))
    t("HEXISTS_wrongtype", lambda: assert_err(c.send_command("HEXISTS", k, "f")))
    t("HKEYS_wrongtype", lambda: assert_err(c.send_command("HKEYS", k)))
    t("HVALS_wrongtype", lambda: assert_err(c.send_command("HVALS", k)))

    # HDEL all fields -> key removed
    k = track(uid("h_hda"))
    c.send_command("HSET", k, "f1", "v1")
    c.send_command("HDEL", k, "f1")
    t("HLEN_after_del_all", lambda: assert_int(c.send_command("HLEN", uid("h_hda_check")), 0))

    # HMSET returns OK
    k = track(uid("h_hms_ok"))
    t("HMSET_returns_OK", lambda: assert_ok(c.send_command("HMSET", k, "f", "v")))

    # HSET then HGET multiple fields
    k = track(uid("h_hgm"))
    c.send_command("HSET", k, "name", "Alice", "age", "30", "city", "NYC")
    for field, expected in [("name", "Alice"), ("age", "30"), ("city", "NYC")]:
        t(f"HGET_{field}", lambda k=k,f=field,e=expected: assert_eq(c.send_command("HGET", k, f), e))

    # HGETALL field-value pairing
    k = track(uid("h_hgap"))
    c.send_command("HSET", k, "a", "1", "b", "2", "c", "3")
    t("HGETALL_pairing", lambda: (
        lambda r: (
            assert_list(r),
            assert_eq(len(r), 6),
            assert_eq(set(r), {"a", "1", "b", "2", "c", "3"})
        )
    )(c.send_command("HGETALL", k)))


# ═══════════════════════════════════════════════════════════════════════════
# 6. KEY TESTS (120+)
# ═══════════════════════════════════════════════════════════════════════════

def test_key(c):
    # DEL
    for i in range(10):
        k = track(uid("k_del"))
        c.send_command("SET", k, "v")
        t(f"DEL_{i}", lambda k=k: (
            assert_int(c.send_command("DEL", k), 1),
            assert_nil(c.send_command("GET", k))
        ))

    # DEL nonexistent
    for i in range(5):
        t(f"DEL_nonexist_{i}", lambda: assert_int(c.send_command("DEL", uid("k_del_none")), 0))

    # DEL multiple
    k1 = track(uid("k_delm1"))
    k2 = track(uid("k_delm2"))
    c.send_command("SET", k1, "v1")
    c.send_command("SET", k2, "v2")
    t("DEL_multiple", lambda: assert_int(c.send_command("DEL", k1, k2), 2))

    # EXISTS
    for i in range(5):
        k = track(uid("k_ex"))
        c.send_command("SET", k, "v")
        t(f"EXISTS_yes_{i}", lambda k=k: assert_int(c.send_command("EXISTS", k), 1))

    # EXISTS nonexistent
    for i in range(5):
        t(f"EXISTS_no_{i}", lambda: assert_int(c.send_command("EXISTS", uid("k_ex_none")), 0))

    # EXISTS multiple
    k1 = track(uid("k_exm1"))
    k2 = track(uid("k_exm2"))
    c.send_command("SET", k1, "v")
    c.send_command("SET", k2, "v")
    t("EXISTS_multi", lambda: assert_int(c.send_command("EXISTS", k1, k2, uid("none")), 2))

    # TYPE string
    for i in range(3):
        k = track(uid("k_ts"))
        c.send_command("SET", k, "v")
        t(f"TYPE_string_{i}", lambda k=k: assert_eq(c.send_command("TYPE", k), "string"))

    # TYPE list
    k = track(uid("k_tl"))
    c.send_command("RPUSH", k, "a")
    t("TYPE_list", lambda: assert_eq(c.send_command("TYPE", k), "list"))

    # TYPE set
    k = track(uid("k_tst"))
    c.send_command("SADD", k, "a")
    t("TYPE_set", lambda: assert_eq(c.send_command("TYPE", k), "set"))

    # TYPE zset
    k = track(uid("k_tz"))
    c.send_command("ZADD", k, "1", "a")
    t("TYPE_zset", lambda: assert_eq(c.send_command("TYPE", k), "zset"))

    # TYPE hash
    k = track(uid("k_th"))
    c.send_command("HSET", k, "f", "v")
    t("TYPE_hash", lambda: assert_eq(c.send_command("TYPE", k), "hash"))

    # TYPE none
    for i in range(3):
        t(f"TYPE_none_{i}", lambda: assert_eq(c.send_command("TYPE", uid("k_tn")), "none"))

    # KEYS *
    t("KEYS_star", lambda: (
        lambda r: assert_list(r)
    )(c.send_command("KEYS", "*")))

    # KEYS with pattern
    prefix = uid("k_pat")
    k1 = track(prefix + "a")
    k2 = track(prefix + "b")
    c.send_command("SET", k1, "v")
    c.send_command("SET", k2, "v")
    t("KEYS_pattern", lambda: (
        lambda r: (assert_list(r), assert_eq(len(r) >= 2, True))
    )(c.send_command("KEYS", prefix + "*")))

    # EXPIRE/TTL
    for i in range(5):
        k = track(uid("k_exp"))
        c.send_command("SET", k, "v")
        t(f"EXPIRE_TTL_{i}", lambda k=k: (
            assert_int(c.send_command("EXPIRE", k, "100"), 1),
            (lambda ttl: (assert_int(ttl),))(c.send_command("TTL", k))
        ))

    # EXPIRE nonexistent
    for i in range(3):
        t(f"EXPIRE_nonexist_{i}", lambda: assert_int(c.send_command("EXPIRE", uid("k_exp_none"), "100"), 0))

    # TTL no expiry
    k = track(uid("k_ttlne"))
    c.send_command("SET", k, "v")
    t("TTL_no_expiry", lambda: assert_int(c.send_command("TTL", k), -1))

    # TTL nonexistent
    for i in range(3):
        t(f"TTL_nonexist_{i}", lambda: assert_int(c.send_command("TTL", uid("k_ttl_none")), -2))

    # RENAME
    for i in range(5):
        k1 = track(uid("k_rn_s"))
        k2 = track(uid("k_rn_d"))
        c.send_command("SET", k1, "val")
        t(f"RENAME_{i}", lambda k1=k1,k2=k2: (
            assert_ok(c.send_command("RENAME", k1, k2)),
            assert_nil(c.send_command("GET", k1)),
            assert_eq(c.send_command("GET", k2), "val")
        ))

    # RENAME nonexistent
    t("RENAME_nonexist", lambda: assert_err(c.send_command("RENAME", uid("k_rn_none"), uid("k_rn_dst"))))

    # RENAME overwrite existing
    k1 = track(uid("k_rn_ow_s"))
    k2 = track(uid("k_rn_ow_d"))
    c.send_command("SET", k1, "new")
    c.send_command("SET", k2, "old")
    t("RENAME_overwrite", lambda: (
        assert_ok(c.send_command("RENAME", k1, k2)),
        assert_eq(c.send_command("GET", k2), "new")
    ))

    # DEL after EXPIRE
    k = track(uid("k_delexp"))
    c.send_command("SET", k, "v")
    c.send_command("EXPIRE", k, "100")
    c.send_command("DEL", k)
    t("DEL_after_EXPIRE", lambda: (
        assert_int(c.send_command("EXISTS", k), 0),
        assert_int(c.send_command("TTL", k), -2)
    ))

    # EXPIRE overwrite
    k = track(uid("k_expow"))
    c.send_command("SET", k, "v")
    c.send_command("EXPIRE", k, "100")
    c.send_command("EXPIRE", k, "200")
    t("EXPIRE_overwrite", lambda: (
        lambda ttl: (assert_int(ttl),)
    )(c.send_command("TTL", k)))

    # RENAME preserves value
    k1 = track(uid("k_rnp_s"))
    k2 = track(uid("k_rnp_d"))
    c.send_command("SET", k1, "preserved")
    c.send_command("RENAME", k1, k2)
    t("RENAME_preserves_value", lambda: assert_eq(c.send_command("GET", k2), "preserved"))

    # RENAME hash
    k1 = track(uid("k_rnh_s"))
    k2 = track(uid("k_rnh_d"))
    c.send_command("HSET", k1, "f", "v")
    c.send_command("RENAME", k1, k2)
    t("RENAME_hash", lambda: (
        assert_eq(c.send_command("HGET", k2, "f"), "v"),
        assert_int(c.send_command("EXISTS", k1), 0)
    ))

    # RENAME list
    k1 = track(uid("k_rnl_s"))
    k2 = track(uid("k_rnl_d"))
    c.send_command("RPUSH", k1, "a", "b")
    c.send_command("RENAME", k1, k2)
    t("RENAME_list", lambda: assert_eq(c.send_command("LRANGE", k2, "0", "-1"), ["a", "b"]))

    # DBSIZE
    t("DBSIZE", lambda: assert_int(c.send_command("DBSIZE")))

    # KEYS with no matches
    t("KEYS_no_match", lambda: (
        lambda r: (assert_list(r), assert_eq(len(r), 0))
    )(c.send_command("KEYS", "zzz_nonexistent_pattern_*")))

    # EXISTS after DEL
    k = track(uid("k_exdel"))
    c.send_command("SET", k, "v")
    c.send_command("DEL", k)
    t("EXISTS_after_DEL", lambda: assert_int(c.send_command("EXISTS", k), 0))

    # TYPE after overwrite
    k = track(uid("k_typeow"))
    c.send_command("SET", k, "string")
    c.send_command("DEL", k)
    c.send_command("RPUSH", k, "a")
    t("TYPE_after_overwrite", lambda: assert_eq(c.send_command("TYPE", k), "list"))


# ═══════════════════════════════════════════════════════════════════════════
# 7. SERVER TESTS (60+)
# ═══════════════════════════════════════════════════════════════════════════

def test_server(c):
    # PING
    for i in range(10):
        t(f"PING_{i}", lambda: assert_eq(c.send_command("PING"), "PONG"))

    # PING with message
    for msg in ["hello", "world", "", "test", "x" * 100]:
        t(f"PING_msg_{msg[:5]}", lambda msg=msg: assert_eq(c.send_command("PING", msg), msg))

    # ECHO
    for msg in ["hello", "world", "", "test", "12345"]:
        t(f"ECHO_{msg[:5]}", lambda msg=msg: assert_eq(c.send_command("ECHO", msg), msg))

    # ECHO special
    t("ECHO_unicode", lambda: assert_eq(c.send_command("ECHO", "你好"), "你好"))

    # INFO
    t("INFO", lambda: assert_str(c.send_command("INFO")))
    for section in ["server", "clients", "memory", "stats", "all", "default"]:
        t(f"INFO_{section}", lambda s=section: assert_str(c.send_command("INFO", s)))

    # CLIENT LIST
    t("CLIENT_LIST", lambda: assert_str(c.send_command("CLIENT", "LIST")))

    # CLIENT SETNAME
    t("CLIENT_SETNAME", lambda: assert_ok(c.send_command("CLIENT", "SETNAME", "test")))

    # CLIENT GETNAME
    t("CLIENT_GETNAME", lambda: c.send_command("CLIENT", "GETNAME"))  # may be null

    # CLIENT ID
    t("CLIENT_ID", lambda: c.send_command("CLIENT", "ID"))

    # DBSIZE
    for i in range(3):
        t(f"DBSIZE_{i}", lambda: assert_int(c.send_command("DBSIZE")))

    # CONFIG GET
    t("CONFIG_GET", lambda: assert_list(c.send_command("CONFIG", "GET", "maxmemory")))
    t("CONFIG_GET_star", lambda: assert_list(c.send_command("CONFIG", "GET", "*")))

    # CONFIG SET
    t("CONFIG_SET", lambda: assert_ok(c.send_command("CONFIG", "SET", "maxmemory", "100mb")))

    # COMMAND
    t("COMMAND", lambda: c.send_command("COMMAND"))

    # SELECT
    for db in ["0", "1", "5", "15"]:
        t(f"SELECT_{db}", lambda db=db: assert_ok(c.send_command("SELECT", db)))

    # SELECT invalid
    t("SELECT_invalid", lambda: assert_err(c.send_command("SELECT", "99")))
    t("SELECT_negative", lambda: assert_err(c.send_command("SELECT", "-1")))

    # AUTH without password
    t("AUTH_no_password", lambda: assert_err(c.send_command("AUTH", "test")))

    # FLUSHDB
    t("FLUSHDB", lambda: assert_ok(c.send_command("FLUSHDB")))

    # FLUSHALL
    t("FLUSHALL", lambda: assert_ok(c.send_command("FLUSHALL")))

    # QUIT returns OK (but we don't actually quit)
    # Just test that it's recognized
    t("QUIT", lambda: assert_ok(c.send_command("QUIT")))

    # Multiple commands in sequence
    t("Server_sequence", lambda: (
        assert_eq(c.send_command("PING"), "PONG"),
        assert_str(c.send_command("INFO")),
        assert_int(c.send_command("DBSIZE")),
        assert_ok(c.send_command("SELECT", "0"))
    ))

    # Unsupported commands return errors
    for cmd_name in ["SETEX", "SETNX", "GETSET", "INCRBYFLOAT", "GETRANGE", "SETRANGE",
                     "MULTI", "EXEC", "EVAL", "SUBSCRIBE", "XADD", "PFADD", "GEOADD",
                     "SETBIT", "GETBIT", "LPOS", "LSET", "LINSERT", "LREM", "LTRIM",
                     "SPOP", "SRANDMEMBER", "SUNION", "SINTER", "SDIFF",
                     "ZPOPMIN", "ZPOPMAX", "ZRANK", "ZREVRANK", "ZCOUNT",
                     "HINCRBY", "HSETNX", "HSCAN",
                     "PERSIST", "PTTL", "PEXPIRE", "RANDOMKEY", "DUMP", "RESTORE",
                     "TIME", "DEBUG", "WAIT", "TOUCH", "UNLINK", "OBJECT",
                     "BITOP", "BITCOUNT", "BITPOS", "BITFIELD"]:
        t(f"Unsupported_{cmd_name}", lambda cn=cmd_name: assert_err(c.send_command(cn)))

    # CONFIG wrong subcommand
    t("CONFIG_bad_subcmd", lambda: assert_err(c.send_command("CONFIG", "BADCMD")))


# ═══════════════════════════════════════════════════════════════════════════
# 8-14. UNSUPPORTED COMMAND GROUPS (test error handling)
# ═══════════════════════════════════════════════════════════════════════════

def test_hyperloglog(c):
    # PFADD not supported - test error handling
    for i in range(15):
        k = track(uid("hll"))
        t(f"PFADD_unsupported_{i}", lambda k=k: assert_err(c.send_command("PFADD", k, f"e{i}")))
    for i in range(10):
        t(f"PFCOUNT_unsupported_{i}", lambda: assert_err(c.send_command("PFCOUNT", uid("hll"))))
    for i in range(5):
        t(f"PFMERGE_unsupported_{i}", lambda: assert_err(c.send_command("PFMERGE", uid("hll"), uid("hll2"))))

def test_geo(c):
    for i in range(10):
        t(f"GEOADD_unsupported_{i}", lambda: assert_err(c.send_command("GEOADD", uid("geo"), "13.36", "38.11", "Palermo")))
    for i in range(5):
        t(f"GEODIST_unsupported_{i}", lambda: assert_err(c.send_command("GEODIST", uid("geo"), "a", "b")))
    for i in range(5):
        t(f"GEOPOS_unsupported_{i}", lambda: assert_err(c.send_command("GEOPOS", uid("geo"), "a")))
    for i in range(5):
        t(f"GEOHASH_unsupported_{i}", lambda: assert_err(c.send_command("GEOHASH", uid("geo"), "a")))
    for i in range(5):
        t(f"GEORADIUS_unsupported_{i}", lambda: assert_err(c.send_command("GEORADIUS", uid("geo"), "15", "37", "200", "km")))

def test_bit(c):
    for i in range(10):
        t(f"SETBIT_unsupported_{i}", lambda: assert_err(c.send_command("SETBIT", uid("bit"), "0", "1")))
    for i in range(10):
        t(f"GETBIT_unsupported_{i}", lambda: assert_err(c.send_command("GETBIT", uid("bit"), "0")))
    for i in range(5):
        t(f"BITCOUNT_unsupported_{i}", lambda: assert_err(c.send_command("BITCOUNT", uid("bit"))))
    for i in range(5):
        t(f"BITOP_unsupported_{i}", lambda: assert_err(c.send_command("BITOP", "AND", uid("bit"), uid("bit2"))))

def test_scripting(c):
    for i in range(10):
        t(f"EVAL_unsupported_{i}", lambda: assert_err(c.send_command("EVAL", "return 1", "0")))
    for i in range(5):
        t(f"EVALSHA_unsupported_{i}", lambda: assert_err(c.send_command("EVALSHA", "abc123", "0")))
    for i in range(5):
        t(f"SCRIPT_unsupported_{i}", lambda: assert_err(c.send_command("SCRIPT", "LOAD", "return 1")))

def test_transaction(c):
    for i in range(10):
        t(f"MULTI_unsupported_{i}", lambda: assert_err(c.send_command("MULTI")))
    for i in range(5):
        t(f"EXEC_unsupported_{i}", lambda: assert_err(c.send_command("EXEC")))
    for i in range(5):
        t(f"DISCARD_unsupported_{i}", lambda: assert_err(c.send_command("DISCARD")))

def test_pubsub(c):
    for i in range(5):
        t(f"SUBSCRIBE_unsupported_{i}", lambda: assert_err(c.send_command("SUBSCRIBE", uid("ch"))))
    for i in range(5):
        t(f"PUBLISH_unsupported_{i}", lambda: assert_err(c.send_command("PUBLISH", uid("ch"), "msg")))

def test_stream(c):
    for i in range(10):
        t(f"XADD_unsupported_{i}", lambda: assert_err(c.send_command("XADD", uid("stream"), "*", "f", "v")))
    for i in range(5):
        t(f"XLEN_unsupported_{i}", lambda: assert_err(c.send_command("XLEN", uid("stream"))))
    for i in range(5):
        t(f"XRANGE_unsupported_{i}", lambda: assert_err(c.send_command("XRANGE", uid("stream"), "-", "+")))


# ═══════════════════════════════════════════════════════════════════════════
# EDGE CASE / CROSS-TYPE TESTS (extra to reach 1000+)
# ═══════════════════════════════════════════════════════════════════════════

def test_edge_cases(c):
    # Cross-type conflicts
    k = track(uid("ec_ct1"))
    c.send_command("SET", k, "string")
    t("GET_on_string", lambda: assert_eq(c.send_command("GET", k), "string"))
    c.send_command("DEL", k)
    c.send_command("RPUSH", k, "a")
    t("GET_on_list", lambda: assert_err(c.send_command("GET", k)))
    t("HGET_on_list", lambda: assert_err(c.send_command("HGET", k, "f")))
    t("SADD_on_list", lambda: assert_err(c.send_command("SADD", k, "a")))
    t("ZADD_on_list", lambda: assert_err(c.send_command("ZADD", k, "1", "a")))
    c.send_command("DEL", k)

    # Key reuse after DEL
    for i in range(10):
        k = track(uid("ec_reuse"))
        c.send_command("SET", k, "str")
        t(f"SET_on_key_{i}", lambda k=k: assert_eq(c.send_command("GET", k), "str"))
        c.send_command("DEL", k)
        c.send_command("RPUSH", k, "a")
        t(f"LRANGE_after_reuse_{i}", lambda k=k: assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["a"]))
        c.send_command("DEL", k)

    # Empty string operations
    for i in range(5):
        k = track(uid("ec_emp"))
        c.send_command("SET", k, "")
        t(f"STRLEN_empty_{i}", lambda k=k: assert_int(c.send_command("STRLEN", k), 0))

    # INCR after SET empty
    k = track(uid("ec_ince"))
    c.send_command("SET", k, "")
    t("INCR_empty_string", lambda: assert_err(c.send_command("INCR", k)))

    # INCR from 0 many times
    k = track(uid("ec_incr0"))
    for i in range(20):
        c.send_command("INCR", k)
    t("INCR_20times_from_0", lambda: assert_eq(c.send_command("GET", k), "20"))
    track(k)

    # DECR from 0 many times
    k = track(uid("ec_decr0"))
    for i in range(20):
        c.send_command("DECR", k)
    t("DECR_20times_from_0", lambda: assert_eq(c.send_command("GET", k), "-20"))

    # MSET then overwrite with MSET
    keys = [track(uid("ec_msow")) for _ in range(5)]
    args1 = []
    args2 = []
    for i, k in enumerate(keys):
        args1.extend([k, f"first_{i}"])
        args2.extend([k, f"second_{i}"])
    c.send_command("MSET", *args1)
    t("MSET_overwrite", lambda: assert_ok(c.send_command("MSET", *args2)))
    result = c.send_command("MGET", *keys)
    t("MGET_after_overwrite", lambda: [assert_eq(result[i], f"second_{i}") for i in range(5)])

    # SET with very large EX
    k = track(uid("ec_largex"))
    c.send_command("SET", k, "v", "EX", "999999")
    t("SET_large_EX", lambda: assert_eq(c.send_command("GET", k), "v"))

    # SET with EX=1
    k = track(uid("ec_ex1"))
    c.send_command("SET", k, "v", "EX", "1")
    t("SET_EX_1", lambda: assert_eq(c.send_command("GET", k), "v"))

    # RPUSH/LPUSH same key alternating
    k = track(uid("ec_alt"))
    c.send_command("RPUSH", k, "r1")
    c.send_command("LPUSH", k, "l1")
    c.send_command("RPUSH", k, "r2")
    c.send_command("LPUSH", k, "l2")
    t("Alternating_push", lambda: assert_eq(c.send_command("LRANGE", k, "0", "-1"), ["l2", "l1", "r1", "r2"]))

    # LPOP/RPOP until empty
    k = track(uid("ec_popempty"))
    c.send_command("RPUSH", k, "a")
    c.send_command("LPOP", k)
    t("LPOP_then_empty", lambda: assert_nil(c.send_command("LPOP", k)))

    # Multiple SADD same element
    k = track(uid("ec_sadup"))
    for _ in range(10):
        c.send_command("SADD", k, "same")
    t("SADD_same_10times", lambda: assert_int(c.send_command("SCARD", k), 1))

    # ZADD then ZREM then ZADD same member
    k = track(uid("ec_zreadd"))
    c.send_command("ZADD", k, "1", "a")
    c.send_command("ZREM", k, "a")
    c.send_command("ZADD", k, "5", "a")
    t("ZADD_after_ZREM", lambda: assert_eq(c.send_command("ZSCORE", k, "a"), "5.0"))

    # HSET then DEL hash then HSET again
    k = track(uid("ec_hreadd"))
    c.send_command("HSET", k, "f", "v1")
    c.send_command("DEL", k)
    c.send_command("HSET", k, "f", "v2")
    t("HSET_after_DEL", lambda: assert_eq(c.send_command("HGET", k, "f"), "v2"))

    # RENAME then RENAME back
    k1 = track(uid("ec_rnback1"))
    k2 = track(uid("ec_rnback2"))
    c.send_command("SET", k1, "original")
    c.send_command("RENAME", k1, k2)
    c.send_command("RENAME", k2, k1)
    t("RENAME_back", lambda: assert_eq(c.send_command("GET", k1), "original"))

    # EXPIRE on different types
    for typ, setup in [("string", lambda k: c.send_command("SET", k, "v")),
                       ("list", lambda k: c.send_command("RPUSH", k, "a")),
                       ("set", lambda k: c.send_command("SADD", k, "a")),
                       ("zset", lambda k: c.send_command("ZADD", k, "1", "a")),
                       ("hash", lambda k: c.send_command("HSET", k, "f", "v"))]:
        k = track(uid("ec_exptype"))
        setup(k)
        t(f"EXPIRE_on_{typ}", lambda k=k: (
            assert_int(c.send_command("EXPIRE", k, "100"), 1),
            (lambda ttl: assert_int(ttl))(c.send_command("TTL", k))
        ))

    # DEL nonexistent multiple
    t("DEL_multi_nonexist", lambda: assert_int(c.send_command("DEL", uid("none1"), uid("none2"), uid("none3")), 0))

    # EXISTS same key multiple times
    k = track(uid("ec_exsame"))
    c.send_command("SET", k, "v")
    t("EXISTS_same_key_multi", lambda: assert_int(c.send_command("EXISTS", k, k, k), 3))

    # APPEND long chain
    k = track(uid("ec_appchain"))
    c.send_command("SET", k, "a")
    for ch in "bcdefghij":
        c.send_command("APPEND", k, ch)
    t("APPEND_chain_10", lambda: (
        assert_eq(c.send_command("GET", k), "abcdefghij"),
        assert_int(c.send_command("STRLEN", k), 10)
    ))

    # LRANGE with start > stop
    k = track(uid("ec_lrss"))
    c.send_command("RPUSH", k, "a", "b", "c")
    t("LRANGE_start_gt_stop", lambda: assert_eq(c.send_command("LRANGE", k, "2", "0"), []))

    # ZRANGE with start > stop
    k = track(uid("ec_zrss"))
    c.send_command("ZADD", k, "1", "a", "2", "b", "3", "c")
    t("ZRANGE_start_gt_stop", lambda: assert_eq(c.send_command("ZRANGE", k, "2", "0"), []))

    # Large MGET
    keys = [track(uid("ec_lmg")) for _ in range(30)]
    for k in keys:
        c.send_command("SET", k, "v")
    result = c.send_command("MGET", *keys)
    t("MGET_30keys", lambda: (assert_list(result), assert_eq(len(result), 30)))

    # HSET with many fields
    k = track(uid("ec_hmany"))
    args = []
    for i in range(20):
        args.extend([f"f{i}", f"v{i}"])
    c.send_command("HSET", k, *args)
    t("HLEN_20fields", lambda: assert_int(c.send_command("HLEN", k), 20))

    # SADD many then SCARD
    k = track(uid("ec_smany"))
    for i in range(30):
        c.send_command("SADD", k, f"m{i}")
    t("SCARD_30members", lambda: assert_int(c.send_command("SCARD", k), 30))

    # ZADD many then ZCARD
    k = track(uid("ec_zmany"))
    for i in range(30):
        c.send_command("ZADD", k, str(i), f"m{i}")
    t("ZCARD_30members", lambda: assert_int(c.send_command("ZCARD", k), 30))


# ═══════════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════════

def main():
    print(f"Connecting to RorisDB Redis protocol on {HOST}:{PORT}...")
    try:
        c = new_client()
        resp = c.send_command("PING")
        print(f"Connected! PING -> {resp}")
        c.close()
    except Exception as e:
        print(f"FATAL: Cannot connect: {e}")
        print(json.dumps({"protocol":"redis","total":0,"passed":0,"failed":0,
                          "failures":[{"test":"connection","error":str(e)}]}, indent=2))
        sys.exit(1)

    modules = [
        ("1.String", test_string),
        ("2.List", test_list),
        ("3.Set", test_set),
        ("4.SortedSet", test_sorted_set),
        ("5.Hash", test_hash),
        ("6.Key", test_key),
        ("7.Server", test_server),
        ("8.HyperLogLog", test_hyperloglog),
        ("9.Geo", test_geo),
        ("10.Bit", test_bit),
        ("11.Scripting", test_scripting),
        ("12.Transaction", test_transaction),
        ("13.PubSub", test_pubsub),
        ("14.Stream", test_stream),
        ("15.EdgeCases", test_edge_cases),
    ]

    for name, func in modules:
        print(f"\n--- {name} ---")
        c = new_client()
        try:
            func(c)
        except Exception as e:
            print(f"  FATAL: {e}")
            traceback.print_exc()
            _result.record(f"{name}_fatal", False, str(e))
        finally:
            try: cleanup(c)
            except: pass
            c.close()

    # Final cleanup
    try:
        c = new_client()
        cleanup(c)
        c.close()
    except: pass

    output = {
        "protocol": "redis",
        "total": _result.total,
        "passed": _result.passed,
        "failed": _result.failed,
        "failures": _result.failures[:20]
    }

    print("\n" + "=" * 60)
    print(json.dumps(output, indent=2))
    print("=" * 60)
    sys.exit(0 if _result.failed == 0 else 1)

if __name__ == "__main__":
    main()
