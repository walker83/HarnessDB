# RorisDB 内核参数配置、运维、备份、SQL编辑器 实施计划

## Context

RorisDB 当前缺乏生产级运维能力：配置仅靠 CLI 参数、SET/SHOW 被 MySQL 协议层拦截返回空值、备份/恢复是 stub、无审计日志集成、无 SQL 开发工具。本计划新增 4 大功能模块，使数据库具备可配置、可运维、可备份、可开发的能力。

---

## Phase 0: 基础设施修复（前置条件）

### 0a: 修复 MySQL 协议层拦截 — 让 SET/SHOW 命令到达 QueryHandler

**文件**: `crates/mysql-protocol/src/connection.rs` (行 401-512)

**修改**:
- 删除行 406-409 的 `set ` 前缀拦截（直接返回 OK）
- 删除行 476-485 的 `SHOW VARIABLES` 拦截（返回空结果集）
- 删除行 486-495 的 `SHOW STATUS` 拦截（返回空结果集）
- 删除行 496-511 的 `SHOW PROCESSLIST` 拦截（返回空结果集）
- **保留**: `SELECT @@version_comment`、`SELECT database()` 拦截（MySQL 客户端初始化必需）
- **保留**: `SHOW WARNINGS`/`SHOW ERRORS` 拦截（返回空是合理的）
- **改造**: `SELECT @@variable` 拦截 — 对于已知变量保留当前行为，未知的传递给 handler

### 0b: 新增 AST 变体

**文件**: `crates/fe-sql-parser/src/ast.rs`

新增变体（在 Statement 枚举尾部添加）:
```rust
ShowStatus { global: bool, pattern: Option<String> },
KillQuery(u64),
KillConnection(u64),
AdminCheckTable(String),
AdminShowReplica,
```

**文件**: `crates/fe-sql-parser/src/parser.rs`

在现有 SHOW/KILL/ADMIN 解析逻辑附近添加：
- `SHOW STATUS [GLOBAL|SESSION] [LIKE pattern]` → `ShowStatus`
- `KILL QUERY <id>` / `KILL <id>` → `KillQuery` / `KillConnection`
- `ADMIN CHECK TABLE <table>` → `AdminCheckTable`
- `ADMIN SHOW REPLICA` → `AdminShowReplica`

### 0c: 新增 workspace 依赖

**文件**: `Cargo.toml` (workspace)

```toml
# 新增
toml = "0.8"
sysinfo = "0.33"
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }

# 新增 workspace members
"crates/fe-config",
"crates/fe-backup",
```

**文件**: `roris-server/Cargo.toml`

新增依赖: `fe-config`, `fe-backup`, `toml`, `sysinfo`, `axum`, `tower-http`

---

## Phase 1: 内核参数配置系统

### 1a: 创建 `crates/fe-config/` crate

**新文件**:
- `crates/fe-config/Cargo.toml`
- `crates/fe-config/src/lib.rs`
- `crates/fe-config/src/config.rs` — TOML 配置加载
- `crates/fe-config/src/variables.rs` — 系统变量管理

### 1b: RorisConfig — TOML 配置文件结构

文件: `crates/fe-config/src/config.rs`

```toml
# roris.toml 示例
[server]
mysql_port = 9030
bind_addr = "127.0.0.1"
max_connections = 100
wait_timeout = 28800
http_port = 8080

[storage]
data_dir = "data/fe/storage"
compression = "zstd"       # zstd | snappy | uncompressed
page_size = 4096

[query]
query_timeout = 300
max_allowed_packet = 4194304
sql_mode = ""
time_zone = "SYSTEM"

[logging]
enable_audit_log = true
slow_query_threshold_ms = 1000
audit_log_dir = "data/fe/audit"
audit_log_max_size_mb = 100
audit_log_max_files = 10

[security]
auth_enabled = false
```

`RorisConfig` 结构体使用 `serde::Deserialize` 从 TOML 文件加载，所有字段有 `#[serde(default)]` 提供默认值。

CLI 参数覆盖配置文件值: `--config-file` (默认 `roris.toml`), 现有 `--mysql-port`/`--data-dir`/`--meta-dir` 变为可选覆盖。

### 1c: SystemVariableManager — 全局+会话变量

文件: `crates/fe-config/src/variables.rs`

核心结构:
- `VarDef { name, default_value, scope: VarScope, kind: VarKind, description }` — 变量定义
- `VarScope { Global, Session, Both }` — 作用域
- `GlobalVariables` — 全局变量存储（`RwLock<HashMap<String, String>>`）
- `SessionVariables` — 会话变量存储（每个连接独立）
- `SystemVariableManager` — 统一管理器，提供 `get()`, `set_global()`, `set_session()`, `match_like()` 方法

预定义变量（约 25 个）:
| 变量名 | 默认值 | 作用域 | 说明 |
|--------|--------|--------|------|
| `version` | `"5.7.42"` | Global | 服务器版本 |
| `version_comment` | `"RorisDB"` | Global | 版本注释 |
| `max_connections` | `"100"` | Global | 最大连接数 |
| `query_timeout` | `"300"` | Both | 查询超时（秒） |
| `max_allowed_packet` | `"4194304"` | Both | 最大包大小 |
| `storage_compression` | `"zstd"` | Global | 存储压缩算法 |
| `enable_audit_log` | `"true"` | Global | 审计日志开关 |
| `slow_query_threshold` | `"1000"` | Both | 慢查询阈值（ms） |
| `autocommit` | `"1"` | Both | 自动提交 |
| `sql_mode` | `""` | Both | SQL 模式 |
| `time_zone` | `"SYSTEM"` | Both | 时区 |
| `wait_timeout` | `"28800"` | Both | 空闲超时（秒） |
| `character_set_client` | `"utf8mb4"` | Session | 客户端字符集 |
| `collation_connection` | `"utf8mb4_general_ci"` | Session | 连接排序规则 |
| `http_port` | `"8080"` | Global | HTTP 管理端口 |
| ... 等 | | | |

### 1d: 改造 RorisQueryHandler

**文件**: `roris-server/src/handler_struct.rs`

新增字段:
```rust
pub(crate) config: RorisConfig,
pub(crate) sys_vars: Arc<SystemVariableManager>,
pub(crate) session_vars: Arc<PlRwLock<SessionVariables>>,
```

修改 `RorisQueryHandler::new()` 接受 `RorisConfig` 和 `Arc<SystemVariableManager>`。

### 1e: 实现 SET / SHOW VARIABLES

**文件**: `roris-server/src/query_executor.rs`

替换 stub `set_variable()`:
- 解析变量名和值
- 根据 `is_global` 调用 `sys_vars.set_global()` 或 `sys_vars.set_session()`
- 类型校验（int/bool/enum 范围检查）

替换 stub `show_variables()`:
- 调用 `sys_vars.match_like(pattern)` 获取所有匹配变量
- 返回 `Variable_name | Value` 两列结果集

---

## Phase 2: 运维能力

### 2a: 接入审计日志

**文件修改**:
- `roris-server/src/handler_struct.rs` — 添加 `audit_logger: Arc<AuditLogger>` 字段
- `roris-server/src/fe_main.rs` — 将 `monitoring.audit_log` 传入 handler
- `roris-server/src/fe_main.rs` 的 `handle_query()` — 在执行前后记录审计信息

实现:
```rust
// 在 handle_query 中
let start = Instant::now();
let result = /* 原有执行逻辑 */;
let duration_ms = start.elapsed().as_millis() as u64;

// 异步写入审计日志（不阻塞查询返回）
let audit = self.audit_logger.clone();
let entry = AuditLogEntry {
    timestamp: Utc::now(),
    user: "root".to_string(),
    host: "127.0.0.1".to_string(),
    database: Some(self.current_database.read().clone()),
    query: trimmed.to_string(),
    query_type: QueryType::from_sql(trimmed),
    status: if result_has_error { QueryStatus::Failed } else { QueryStatus::Success },
    duration_ms,
    rows_affected: None,
    bytes_scanned: None,
    error_message: extract_error(&result),
};
tokio::spawn(async move { audit.log_entry(entry).await; });
```

需要在 `AuditLogger` 中添加 `log_entry(entry)` 方法（直接接受已构建的 entry）。

### 2b: ConnectionTracker — 连接追踪

**新文件**: `roris-server/src/connection_tracker.rs`

```rust
pub struct ConnectionTracker {
    connections: RwLock<HashMap<u32, ConnectionInfo>>,
    next_id: AtomicU32,
    total_connections: AtomicU64,
    active_queries: AtomicU64,
    total_queries: AtomicU64,
    peak_connections: AtomicU32,
    startup_time: Instant,
}
```

方法:
- `register(host, user) -> conn_id` — 注册新连接
- `unregister(conn_id)` — 移除连接
- `update_sql(conn_id, sql)` — 更新当前执行的 SQL
- `list() -> Vec<ConnectionInfo>` — 列出所有活跃连接
- `kill(conn_id) -> bool` — 标记连接需终止
- 指标方法: `uptime()`, `total_queries()`, `active_queries()`, `total_connections()`, `peak_connections()`

### 2c: 改造 MysqlServer 集成 ConnectionTracker

**文件修改**: `crates/mysql-protocol/src/server.rs`

`ServerConfig` 新增:
```rust
pub connection_tracker: Option<Arc<ConnectionTracker>>,
```

`Connection` 在建立时调用 `tracker.register()`，断开时调用 `tracker.unregister()`，执行查询前调用 `tracker.update_sql()`。

需要给 `QueryHandler` trait 添加可选的连接上下文，或通过共享 Arc 的方式让 handler 可以访问 tracker。

**最简方案**: ConnectionTracker 存放在 RorisQueryHandler 中（通过 Arc 共享），MysqlServer 也持有同一个 Arc。Connection 在创建时从 tracker 获取 ID，查询时通过 handler 的 `set_current_conn_id()` 方法通知 handler。

### 2d: 实现 SHOW PROCESSLIST

**文件**: `roris-server/src/query_executor.rs`

替换 stub `show_processlist()`:
- 从 `connection_tracker.list()` 获取连接信息
- 返回 8 列结果集: `Id | User | Host | db | Command | Time | State | Info`

### 2e: 实现 SHOW STATUS

**文件**: `roris-server/src/query_executor.rs`

新增 `show_status()` 方法:
- 从 `connection_tracker` 获取运行时指标
- 使用 `sysinfo` 获取系统信息（内存、CPU）
- 返回 `Variable_name | Value` 结果集

主要指标:
| 指标 | 说明 |
|------|------|
| `Uptime` | 服务器运行秒数 |
| `Queries` | 总查询数 |
| `Threads_connected` | 当前连接数 |
| `Threads_running` | 活跃查询数 |
| `Connections` | 历史总连接数 |
| `Max_used_connections` | 峰值连接数 |
| `Slow_queries` | 慢查询数 |
| `Bytes_received` / `Bytes_sent` | 网络流量（可后续实现） |
| `Total_data_size` | 数据目录总大小 |
| `Database_count` | 数据库数量 |
| `Table_count` | 表数量 |

### 2f: 实现 KILL QUERY 和 ADMIN 命令

**文件**: `roris-server/src/query_executor.rs`

- `kill_query(id)` — 调用 `connection_tracker.kill(id)`
- `admin_check_table(table)` — 读取 Parquet 文件验证完整性，返回行数、文件大小
- `admin_show_replica()` — 单节点模式返回 N/A 状态

---

## Phase 3: 备份/恢复系统

### 3a: 创建 `crates/fe-backup/` crate

**新文件**:
- `crates/fe-backup/Cargo.toml`
- `crates/fe-backup/src/lib.rs`
- `crates/fe-backup/src/backup_manager.rs` — 核心备份逻辑
- `crates/fe-backup/src/repository.rs` — 仓库管理
- `crates/fe-backup/src/export.rs` — 导出/导入

### 3b: BackupManager 核心结构

文件: `crates/fe-backup/src/backup_manager.rs`

```rust
pub struct BackupManager {
    repositories: RwLock<HashMap<String, RepositoryInfo>>,
    meta_dir: PathBuf,
}

pub struct RepositoryInfo {
    pub name: String,
    pub path: PathBuf,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct BackupManifest {
    pub backup_name: String,
    pub database: String,
    pub timestamp: String,
    pub backup_type: String,  // "full" | "incremental"
    pub tables: Vec<BackupTableInfo>,
    pub total_size_bytes: u64,
    pub total_rows: u64,
}

#[derive(Serialize, Deserialize)]
pub struct BackupTableInfo {
    pub name: String,
    pub file_name: String,
    pub row_count: u64,
    pub size_bytes: u64,
}
```

### 3c: BACKUP 实现

**流程**:
1. 解析仓库路径: `repositories[name].path`
2. 创建备份目录: `{repo_path}/{backup_name}/`
3. 从 catalog 获取数据库所有表
4. 对每个表:
   - 拷贝 `{data_dir}/{db}/{table}/data.parquet` → `{backup_dir}/{table}/data.parquet`
   - 记录行数、文件大小
5. 序列化 `BackupManifest` → `{backup_dir}/manifest.json`
6. 序列化表的 catalog 元数据 → `{backup_dir}/catalog_snapshot.json`

### 3d: RESTORE 实现

**流程**:
1. 读取 `{repo_path}/{backup_name}/manifest.json`
2. 读取 `{backup_dir}/catalog_snapshot.json`
3. 如果目标数据库不存在则创建
4. 对每个表:
   - 从 `catalog_snapshot.json` 恢复表定义到 catalog
   - 拷贝 `{backup_dir}/{table}/data.parquet` → `{data_dir}/{db}/{table}/data.parquet`
   - 在 DataFusion catalog 中注册
5. 保存 catalog

### 3e: Repository 管理

**文件**: `crates/fe-backup/src/repository.rs`

仓库信息持久化到 `{meta_dir}/repositories.json`:
```json
{
  "repositories": {
    "local_backup": { "name": "local_backup", "path": "/data/backups", "created_at": "..." }
  }
}
```

实现:
- `create_repository(name, path)` — 创建目录 + 持久化
- `drop_repository(name)` — 删除（不删除备份数据）
- `list_repositories()` — 列出所有仓库
- `show_repository(name)` — 显示仓库详情和备份列表

### 3f: EXPORT / IMPORT 实现

**文件**: `crates/fe-backup/src/export.rs`

EXPORT TABLE:
- 读取 Parquet 文件
- 根据 `PROPERTIES("format" = "csv"|"parquet")` 输出（默认 parquet）
- CSV 使用 Arrow CSV writer
- 输出到指定路径

IMPORT TABLE:
- 读取文件（parquet 或 csv）
- 如果表不存在则报错
- 将数据写入 storage（复用 `storage.insert()` 逻辑）

### 3g: 接入 QueryHandler

**文件修改**:
- `roris-server/src/handler_struct.rs` — 添加 `backup_manager: Arc<BackupManager>`
- `roris-server/src/ddl_handler.rs` — 替换 stub:
  - `create_repository()` → 调用 `backup_manager.create_repository()`
  - `drop_repository()` → 调用 `backup_manager.drop_repository()`
  - `backup_database()` → 调用 `backup_manager.backup_database()`
  - `restore_database()` → 调用 `backup_manager.restore_database()`
  - `export_table()` → 调用 `export::export_table()`
  - `show_repositories()` → 调用 `backup_manager.list_repositories()`

---

## Phase 4: 小型化 SQL 编辑器

### 4a: Web 模块结构

**新文件**（在 `roris-server` 内部）:
- `roris-server/src/web/mod.rs` — Axum HTTP 服务器启动
- `roris-server/src/web/routes.rs` — REST API 处理器
- `roris-server/src/web/editor.html` — 嵌入式 SQL 编辑器（单 HTML 文件）

### 4b: Axum HTTP 服务器

文件: `roris-server/src/web/mod.rs`

```rust
pub struct WebState {
    pub handler: Arc<RorisQueryHandler>,
    pub query_history: Arc<RwLock<Vec<QueryHistoryEntry>>>,
}

pub async fn start_web_server(state: Arc<WebState>, port: u16) {
    let app = Router::new()
        .route("/", get(serve_editor))
        .route("/api/query", post(api_query))
        .route("/api/databases", get(api_databases))
        .route("/api/tables/{db}", get(api_tables))
        .route("/api/schema/{db}/{table}", get(api_schema))
        .route("/api/history", get(api_history))
        .route("/api/status", get(api_status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    tracing::info!("SQL Editor web server on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await.unwrap();
}
```

使用 `include_str!("editor.html")` 嵌入 HTML 文件，编译进二进制。

### 4c: REST API

| 端点 | 方法 | 功能 |
|------|------|------|
| `/` | GET | 返回 SQL 编辑器 HTML 页面 |
| `/api/query` | POST | 执行 SQL，返回结果集 |
| `/api/databases` | GET | 列出所有数据库 |
| `/api/tables/{db}` | GET | 列出指定库的所有表 |
| `/api/schema/{db}/{table}` | GET | 获取表的列定义 |
| `/api/history` | GET | 查询历史（最近 100 条） |
| `/api/status` | GET | 服务器状态信息 |

`POST /api/query` 请求/响应:
```json
// Request
{ "sql": "SELECT * FROM t1", "database": "test" }

// Response
{
  "columns": [{"name": "id", "type": "Int64"}, {"name": "name", "type": "Utf8"}],
  "rows": [["1", "Alice"], ["2", "Bob"]],
  "duration_ms": 12,
  "row_count": 2,
  "error": null
}
```

### 4d: SQL 编辑器前端（单 HTML 文件）

文件: `roris-server/src/web/editor.html`

一个自包含的 HTML 文件（内嵌 CSS + JS，无需构建工具）:

**布局**:
```
+----------------------------------------------------------+
|  RorisDB SQL Editor                    [DB: ▼] [Execute] |
+------------------+---------------------------------------+
| Database Browser |  SQL Editor                            |
|                  |  +-----------------------------------+ |
| 📁 information_  |  | SELECT * FROM                     | |
| 📁 test_db       |  | WHERE id > 10                     | |
|   📋 users       |  +-----------------------------------+ |
|   📋 orders      |                                       |
|   📋 products    |  Results (2 rows, 12ms)               |
|                  |  +-----------------------------------+ |
| 📁 analytics     |  | id | name  | age |                | |
|                  |  | 11 | Alice | 25  |                | |
| History          |  | 12 | Bob   | 30  |                | |
| - SELECT * FROM  |  +-----------------------------------+ |
| - CREATE TABLE   |                                       |
+------------------+---------------------------------------+
```

**功能**:
1. **左侧边栏**:
   - 数据库树形浏览器（可展开查看表、列）
   - 查询历史列表（点击可重用）
2. **SQL 编辑区**:
   - 等宽字体 textarea
   - 基本 SQL 关键字高亮（用 CSS + JS 简单的 overlay 实现）
   - Ctrl+Enter 执行快捷键
   - 自动补全：数据库名、表名、列名（从 API 获取）
3. **结果区**:
   - 表格显示查询结果
   - 显示执行时间、行数
   - 错误信息红色显示
   - 结果可滚动
4. **状态栏**:
   - 当前数据库
   - 连接状态
   - 服务器版本

**技术实现**: 纯 Vanilla JS（无框架），`fetch()` API 与后端通信，`<textarea>` 编辑器，CSS Grid 布局。

### 4e: 启动集成

**文件**: `roris-server/src/fe_main.rs`

在 main() 中:
1. 加载配置文件（如果存在）
2. 创建 `SystemVariableManager`
3. 创建 `ConnectionTracker`
4. 创建 `BackupManager`
5. 创建 `RorisQueryHandler`（传入所有组件）
6. 启动 MySQL 服务器
7. **新增**: 如果 `http_port > 0`，启动 Web SQL 编辑器服务器
8. 启动后台任务（EditLog flush, Catalog save）

```rust
// Web SQL Editor
if config.server.http_port > 0 {
    let web_state = Arc::new(WebState {
        handler: Arc::new(query_handler_for_web),
        query_history: Arc::new(RwLock::new(Vec::new())),
    });
    let port = config.server.http_port;
    tokio::spawn(async move {
        start_web_server(web_state, port).await;
    });
    tracing::info!("SQL Editor available at http://127.0.0.1:{}", port);
}
```

---

## 新文件清单

| 文件路径 | 说明 |
|----------|------|
| `crates/fe-config/Cargo.toml` | 配置 crate |
| `crates/fe-config/src/lib.rs` | 导出 |
| `crates/fe-config/src/config.rs` | RorisConfig + TOML 加载 |
| `crates/fe-config/src/variables.rs` | SystemVariableManager |
| `crates/fe-backup/Cargo.toml` | 备份 crate |
| `crates/fe-backup/src/lib.rs` | 导出 |
| `crates/fe-backup/src/backup_manager.rs` | BackupManager + backup/restore |
| `crates/fe-backup/src/repository.rs` | 仓库管理 |
| `crates/fe-backup/src/export.rs` | EXPORT/IMPORT |
| `roris-server/src/connection_tracker.rs` | 连接追踪 |
| `roris-server/src/web/mod.rs` | Web 服务器启动 |
| `roris-server/src/web/routes.rs` | REST API |
| `roris-server/src/web/editor.html` | SQL 编辑器前端 |
| `roris.toml` | 默认配置文件示例 |

## 修改文件清单

| 文件路径 | 修改说明 |
|----------|----------|
| `Cargo.toml` | 添加 toml/sysinfo/axum/tower-http 依赖，添加 fe-config/fe-backup 成员 |
| `roris-server/Cargo.toml` | 添加新依赖 |
| `crates/fe-sql-parser/src/ast.rs` | 添加 ShowStatus/KillQuery/Admin* 变体 |
| `crates/fe-sql-parser/src/parser.rs` | 添加对应解析逻辑 |
| `crates/mysql-protocol/src/connection.rs` | 移除 SET/SHOW VARIABLES/STATUS/PROCESSLIST 拦截 |
| `crates/mysql-protocol/src/server.rs` | 可选: 集成 ConnectionTracker |
| `crates/fe-monitor/src/audit_log.rs` | 添加 log_entry() 方法 |
| `roris-server/src/handler_struct.rs` | 添加 config/sys_vars/audit_logger/backup_manager 字段 |
| `roris-server/src/fe_main.rs` | 加载配置、创建组件、启动 Web 服务器 |
| `roris-server/src/query_executor.rs` | 实现所有新命令，替换 stub |
| `roris-server/src/ddl_handler.rs` | 替换 backup/restore/repository stub |

## 实施顺序

```
Step 1: Phase 0a — 修复 MySQL 协议拦截（所有后续步骤的前提）
Step 2: Phase 0b — 新增 AST 变体和解析
Step 3: Phase 0c — 添加 workspace 依赖
Step 4: Phase 1a-1c — 创建 fe-config crate（config.rs + variables.rs）
Step 5: Phase 1d-1e — 改造 handler，实现 SET/SHOW VARIABLES
Step 6: Phase 2b — 创建 ConnectionTracker
Step 7: Phase 2a — 接入审计日志
Step 8: Phase 2c-2f — 实现 SHOW PROCESSLIST/STATUS, KILL, ADMIN
Step 9: Phase 3a-3f — 创建 fe-backup crate，实现备份/恢复/导出
Step 10: Phase 3g — 接入 handler
Step 11: Phase 4a-4e — 创建 Web SQL 编辑器
Step 12: 集成测试，修复编译错误
```

## 验证方案

1. **配置系统验证**:
   - 创建 `roris.toml`，启动服务器，`SHOW VARIABLES LIKE '%port%'` 显示配置值
   - `SET GLOBAL query_timeout = 600` → `SHOW VARIABLES LIKE 'query_timeout'` 验证
   - `SET @user_var = 'hello'` → `SELECT @user_var` 验证

2. **运维验证**:
   - 打开两个 MySQL 客户端连接，`SHOW PROCESSLIST` 应显示两条记录
   - `SHOW STATUS` 应显示 uptime, queries 等指标
   - 执行查询后检查 `data/fe/audit/` 目录生成审计日志文件

3. **备份验证**:
   ```sql
   CREATE REPOSITORY local_repo WITH BROKER ON '/tmp/roris_backup';
   BACKUP DATABASE test_db TO local_repo AS 'backup_20260525';
   DROP TABLE test_db.users;
   RESTORE DATABASE test_db FROM local_repo AS 'backup_20260525';
   SELECT * FROM test_db.users;  -- 数据应恢复
   ```

4. **SQL 编辑器验证**:
   - 浏览器访问 `http://127.0.0.1:8080`
   - 左侧浏览数据库/表
   - 执行 SQL 并查看结果
   - 查看查询历史
