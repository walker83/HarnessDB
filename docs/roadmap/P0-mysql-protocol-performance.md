# HarnessDB 性能优化计划

## Context

HarnessDB 查询 129,200 行数据耗时 894.8ms，而 DuckDB 只需 3.5ms（慢 255 倍）。
经过深度分析，瓶颈不在 Parquet 读取（~50ms），而在 **MySQL 协议序列化**（~800ms，占 89%）。

### 瓶颈拆解

| 阶段 | 耗时 | 占比 |
|------|------|------|
| Parquet 读取 | ~50ms | 5.6% |
| RecordBatch → String 矩阵 | ~200ms | 22% |
| String → bytes 拷贝 | ~100ms | 11% |
| **逐行 TCP write + flush** | **~500ms** | **56%** |
| 网络传输 | ~40ms | 4.5% |

### 核心问题

1. **`opt-level = "z"`** — Release 构建优化的是二进制体积而非速度
2. **每行一次 `flush()`** — 129,200 行 = 129,200 次 `sendmsg` 系统调用
3. **每格 3-4 次堆分配** — Arrow value → `String` → `Vec<u8>` → `BytesMut` → TCP
4. **全量物化** — 所有结果先转成 `Vec<Vec<Option<String>>>`，再逐行发送
5. **`PacketBuilder` 每行新建** — 无缓冲区复用

---

## 优化方案（按优先级排序）

### Phase 1: 立竿见影（预计提速 5-20 倍）

#### 1.1 修改 Release 优化级别
**文件**: `Cargo.toml:150`
```toml
# Before:
opt-level = "z"       # 优化体积
# After:
opt-level = 3         # 优化速度
```
**预期效果**: 整体提速 20-50%，零风险。

#### 1.2 批量 TCP 写入 + 单次 flush
**文件**: `crates/mysql-protocol/src/connection.rs:608-669`

当前 `write_all()` 每次调用都 flush：
```rust
async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
    self.stream.write_all(data).await?;
    self.stream.flush().await  // ← 每行都 flush！
}
```

改为：
- 新增 `write_no_flush()` 方法，只做 `write_all` 不 flush
- `send_result_set()` 中行数据包使用 `write_no_flush()`
- 仅在最终 EOF/OK 包后执行一次 `flush()`
- 列定义包也用 `write_no_flush()`

**预期效果**: 从 N 次 flush 减少到 1 次，对 129K 行预计提速 5-10 倍。

#### 1.3 复用 BytesMut 缓冲区
**文件**: `crates/mysql-protocol/src/connection.rs`, `crates/mysql-protocol/src/packet.rs`

当前每行创建新的 `PacketBuilder`（即新的 `BytesMut`），改为：
- 在 `Connection` 结构体中新增 `write_buf: BytesMut` 字段（与已有的 `read_buf` 对称）
- 新增 `PacketBuilder::write_into(seq_id, &mut BytesMut)` 方法，直接写入已有缓冲区（避免 `finish()` 中的二次拷贝）
- `send_result_set()` 中所有行数据包直接写入 `self.write_buf`，积累后一次性发送
- 当 `write_buf` 超过阈值（如 64KB）时批量发送并 clear

**预期效果**: 减少 ~129K 次 `BytesMut` 分配 + 消除 `finish()` 中的 payload 拷贝，提速 2-3 倍。

### Phase 2: 消除 String 中间层（预计再提速 3-5 倍）

#### 2.1 Arrow → MySQL 字节直接编码
**文件**: `harness-server/src/utils.rs:144-317`, `crates/mysql-protocol/src/connection.rs`

当前的数据转换路径：
```
Arrow Array → .to_string() → String → .as_bytes().to_vec() → Vec<u8> → encode_text_row → BytesMut → TCP
```

改为直接从 Arrow 数组写入 MySQL 协议字节：
```
Arrow Array → write_to_mysql_buf(&mut BytesMut) → TCP
```

实现一个 `arrow_column_to_mysql_text()` 函数，按类型直接写 lenenc bytes：
- Int8/16/32/64: 使用 `itoa` crate 直接写入 BytesMut（零分配）
- Float32/64: 使用 `ryu` crate 直接写入 BytesMut（零分配）
- String: 直接写入字节切片（零拷贝）
- Date32/Timestamp: 使用栈上固定缓冲区格式化，然后写入

**预期效果**: 消除每格 2-3 次堆分配，129K 行 × 13 列 ≈ 1.7M 次分配变为 0。

#### 2.2 流式发送结果
**文件**: `harness-server/src/utils.rs`, `crates/mysql-protocol/src/connection.rs`

当前流程：`df.collect()` → 全量 RecordBatches → 全量 String 矩阵 → 逐行发送
改为：边从 RecordBatch 读取边编码边发送，无需中间 `Vec<Vec<Option<String>>>`

修改 `send_result_set()` 接收 `&[RecordBatch]` + schema，直接编码发送：
```rust
async fn send_result_set_streaming(
    &mut self,
    columns: &[ColumnDef],
    batches: &[RecordBatch],
) -> std::io::Result<()>
```

**预期效果**: 内存占用从 O(rows × cols) 降到 O(batch_size × cols)，减少一次全量数据拷贝。

### Phase 3: 查询执行优化（预计再提速 2-3 倍）

#### 3.1 复用 SessionContext
**文件**: `harness-server/src/fe_main.rs:88-112`

当前每次 SELECT 都创建新的 `SessionContext`，重新注册 catalog 和 UDFs。
改为：为每个连接维护一个 `SessionContext`，仅更新 `default_catalog`/`default_schema`。

**预期效果**: 减少每次查询 ~1ms 的初始化开销，对短查询影响显著。

#### 3.2 消除 SQL 双重 to_lowercase
**文件**: `crates/mysql-protocol/src/connection.rs:455, 589`

SQL 被 `to_lowercase()` 了两次，产生两次全量字符串拷贝。
改为只 lower一次，传递引用。

#### 3.3 将 QueryHandler 改为 async
**文件**: `crates/mysql-protocol/src/server.rs`, `harness-server/src/fe_main.rs`

当前 `handle_query` 是同步方法，DataFusion 的 async 通过 `thread::spawn + block_on` 桥接。
改为 async trait，让 DataFusion 在 tokio runtime 上直接运行。

**预期效果**: 消除每查询一个 OS 线程的开销，提升并发能力。

---

## 预期优化效果

| 优化阶段 | 预期 129K 行耗时 | 相对 DuckDB 倍数 |
|----------|-----------------|-----------------|
| 当前 | 894.8ms | 255x |
| Phase 1 (opt-level + 批量flush + buffer复用) | ~50-80ms | 15-23x |
| Phase 2 (消除String中间层 + 流式) | ~15-30ms | 4-9x |
| Phase 3 (SessionContext复用 + async) | ~10-20ms | 3-6x |

> 注意：MySQL 协议本身有序列化开销，不可能与 DuckDB（直接内存 DataFrame）完全持平。
> 3-6x 是 MySQL 协议服务器的合理水平（参考 TiDB、Vitess 等）。

---

## 实施顺序

1. `git commit` 当前状态
2. Phase 1.1 — 改 opt-level（1分钟）
3. Phase 1.2 — 批量 flush（30分钟）
4. Phase 1.3 — 缓冲区复用（1小时）
5. 性能测试，验证 Phase 1 效果
6. Phase 2.1+2.2 — Arrow 直接编码 + 流式发送（2-3小时）
7. Phase 3.1+3.2 — SessionContext 复用 + 消除双重 lower（1小时）
8. 完整性能测试，对比 DuckDB

---

## 涉及文件

| 文件 | 修改内容 |
|------|---------|
| `Cargo.toml` | opt-level: "z" → 3 |
| `crates/mysql-protocol/src/connection.rs` | 批量 flush, 缓冲区复用, send_result_set 重构 |
| `crates/mysql-protocol/src/packet.rs` | write_into 方法, PacketBuilder 复用 |
| `harness-server/src/utils.rs` | arrow_column_to_mysql_text(), 流式编码 |
| `harness-server/src/fe_main.rs` | SessionContext 复用 |
| `crates/mysql-protocol/src/server.rs` | QueryHandler async 化 |
