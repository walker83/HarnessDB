# CRITICAL BUG: MySQL DEPRECATE_EOF 结果集终止包头字节错误

## Status
**PRIORITY**: HIGH (P0)
**EFFORT**: SMALL
**CURRENT STATE**: RESOLVED ✅ (v0.3.3)

---

## 问题描述

MySQL 8.0+ 客户端（包括 mysql CLI、JDBC、Go MySQL Driver 等）连接 HarnessDB 后，所有 SELECT 查询永久挂起，无任何返回。服务本身正常运行，端口监听正常，TCP 连接可建立，但查询响应无法被客户端正确解析。

### 复现步骤

```bash
# 启动服务
./target/release/harness-db --mysql-port 9030

# 另一个终端连接
mysql -h 127.0.0.1 -P 9030 -uroot
# 任何 SELECT 查询都会挂起，无响应
```

### 影响范围

- 所有使用 MySQL 8.0+ 客户端的连接（MySQL 8.0 默认启用 `CLIENT_DEPRECATE_EOF`）
- 所有 SELECT / SHOW / DESCRIBE 等返回结果集的语句
- INSERT/UPDATE/DELETE 等 OK 响应不受影响（走不同的代码路径）

---

## Root Cause

### 能力协商

服务端在握手包中广播 `CLIENT_DEPRECATE_EOF`（`0x01000000`）能力标志：

```rust
// packet.rs
pub const DEFAULT_CAPABILITIES: u32 = ...
    | CapabilityFlags::DEPRECATE_EOF;  // 0x01000000
```

MySQL 8.0+ 客户端也支持此标志，协商后双方都设置 `CLIENT_DEPRECATE_EOF`。

### 代码逻辑分支

```rust
// connection.rs — send_result_set()
let use_eof = (self.capability_flags & CapabilityFlags::DEPRECATE_EOF) == 0;
// CLIENT_DEPRECATE_EOF 已设置 → use_eof = false
```

当 `use_eof = false` 时，代码进入 `else` 分支，调用 `make_ok_packet()` 发送结果集终止包：

```rust
} else {
    let ok = packet::make_ok_packet(
        self.seq_id,
        result.rows.len() as u64,
        0,
        packet::SERVER_STATUS_AUTOCOMMIT,
        0,
    );
    self.write_all(&ok).await?;  // ← BUG: 0x00 header
}
```

### 协议规范

根据 MySQL 内部协议文档，`COM_QUERY` 响应在 `CLIENT_DEPRECATE_EOF` 模式下的完整序列为：

```
1. Column count (lenenc int)
2. Column definitions (N packets)
3. [无 EOF 包 — 已废弃]
4. Row data packets
5. OK-style terminator: 0xFE header + lenenc fields  ← 必须是 0xFE！
```

客户端通过首字节区分：
- 5 字节 payload = 旧式 EOF（`0xFE` header）
- ≥7 字节 payload = OK-in-disguise（`0xFE` header + OK 字段）
- `0x00` = 普通 OK 包（**不是结果集终止符**）

### 实际行为

代码发送 `0x00` 头字节（`make_ok_packet`），客户端在结果集状态机中不识别此包为终止符，继续等待正确的 `0xFE` 终止包 → 永久挂起。

---

## Resolution

### 新增函数（`packet.rs`）

```rust
/// Build the result-set terminator when CLIENT_DEPRECATE_EOF IS set.
/// This is an OK-style packet but with 0xFE header byte (not 0x00).
pub fn make_result_set_eof_ok_packet(
    seq_id: u8,
    affected_rows: u64,
    last_insert_id: u64,
    status_flags: u16,
    warning_count: u16,
) -> BytesMut {
    let mut pb = PacketBuilder::new(seq_id);
    pb.put_u8(0xFE);  // EOF/OK header byte
    pb.lenenc_int(affected_rows);
    pb.lenenc_int(last_insert_id);
    pb.put_u16_le(status_flags);
    pb.put_u16_le(warning_count);
    let (packet, _) = pb.finish();
    packet
}
```

### 修复调用点（`connection.rs`）

两处均替换（`send_result_set` 和 `send_binary_result_set`）：

```rust
} else {
    // DEPRECATE_EOF: result-set terminator uses 0xFE header (not 0x00)
    let ok = packet::make_result_set_eof_ok_packet(
        self.seq_id,
        result.rows.len() as u64,
        0,
        packet::SERVER_STATUS_AUTOCOMMIT,
        0,
    );
    self.write_all(&ok).await?;
    self.seq_id = self.seq_id.wrapping_add(1);
}
```

---

## Verification

修复后测试：

```
$ mysql -h 127.0.0.1 -P 9030 -uroot --skip-password -e "SELECT 1 AS test;"
test
1

$ mysql -h 127.0.0.1 -P 9030 -uroot --skip-password -e "
  SHOW DATABASES;
  CREATE DATABASE IF NOT EXISTS test_db;
  USE test_db;
  CREATE TABLE IF NOT EXISTS t1 (id INT, name VARCHAR(50));
  INSERT INTO t1 VALUES (1, 'hello'), (2, 'world');
  SELECT * FROM t1;
"
Database
stock_analysis
information_schema
affected_rows
2
id  name
1   hello
2   world
```

所有操作正常返回，查询不再挂起。

---

## Impact

- **严重性**: 🔴 严重 — 服务对 MySQL 8.0+ 客户端完全不可用
- **触发条件**: 任何 `CLIENT_DEPRECATE_EOF` 能力协商成功的连接（MySQL 8.0 默认行为）
- **受影响功能**: 所有返回结果集的操作（SELECT, SHOW, DESCRIBE, EXPLAIN）
- **不受影响**: INSERT/UPDATE/DELETE（走 OK 包路径，不经此处）；MySQL 5.x 客户端（不使用 DEPRECATE_EOF）
- **修复风险**: 低 — 仅修改包编码逻辑，不影响查询执行层

---

## Lessons Learned

1. MySQL 协议的 `0x00`（OK）和 `0xFE`（EOF/OK-in-disguise）虽然字段格式相同，但首字节语义不同，不可互换
2. 能力协商代码（`DEFAULT_CAPABILITIES`）的默认值会影响所有连接的编码路径，新增能力标志需要全面检查所有相关代码路径
3. 建议在 CI 中加入 MySQL 8.0 客户端的端到端连通性测试，避免协议层回归
