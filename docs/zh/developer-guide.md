# RorisDB 开发者指南

本文档为希望参与 RorisDB 开发的贡献者提供指导。

## 开发环境搭建

### 前置要求

- **Rust**：1.75+ 版本
- **Cargo**：与 Rust 版本匹配
- **Git**：版本控制工具
- **IDE**：推荐 VS Code + rust-analyzer 插件，或 IntelliJ IDEA + Rust 插件

### 获取源代码

```bash
git clone https://github.com/your-repo/RorisDB.git
cd RorisDB
```

### 编译项目

```bash
# 开发模式（快速编译，优化较少）
cargo build

# 发布模式（优化编译，用于测试性能）
cargo build --release
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p fe-sql-parser
cargo test -p be-storage

# 运行集成测试
cargo test --test integration
```

## 项目结构

```
RorisDB/
├── roris-server/          # FE 和 BE 二进制入口点
│   └── src/
│       ├── fe_main.rs     # Frontend Server 主入口
│       └── be_main.rs    # Backend Server 主入口
├── crates/                # 核心模块（14个crate）
│   ├── fe-sql-parser/   # SQL 解析 → AST
│   ├── fe-sql-planner/  # AST → 逻辑计划 → 物理计划 + 优化
│   ├── fe-catalog/      # 数据库/表/分区元数据管理
│   ├── fe-scheduler/    # Fragment 规划、分布式调度
│   ├── fe-expression/   # 向量化表达式求值
│   ├── fe-common/       # FE 共享模块（EditLog、MetaService）
│   ├── mysql-protocol/  # MySQL 线协议服务器
│   ├── be-storage/      # Tablet、Rowset、Segment、Compaction
│   ├── be-execution/    # Pipeline 执行引擎
│   ├── be-segment/      # 列式 Segment 格式
│   ├── be-common/       # BE 共享（配置、指标）
│   ├── data-io/         # CSV/JSON 导入、Stream Load
│   ├── types/           # Vector、Bitmap、Block、DataType、Schema
│   ├── common/          # 错误处理、配置、工具函数
│   ├── proto/           # gRPC 协议定义（protobuf）
│   └── rpc/             # gRPC 服务实现
├── tools/                # 工具和实用程序
│   ├── roris-cli/       # 命令行客户端（REPL）
│   ├── tpch_test/       # TPC-H 测试工具
│   └── mysql_server/    # MySQL 服务器工具
├── benches/              # 基准测试
│   └── tpch/            # TPC-H 基准测试套件
├── tests/                # 测试
│   ├── integration/     # SQL 和协议集成测试
│   └── common/          # 测试公共模块
├── docs/                 # 文档
├── conf/                 # 配置文件
└── Cargo.toml           # Workspace 配置
```

## 核心 Crate 说明

### Frontend Crates

#### `fe-sql-parser`
- **职责**：SQL 解析，将 SQL 文本转换为 AST
- **关键文件**：`src/parser.rs`, `src/ast.rs`
- **依赖**：`sqlparser` crate

#### `fe-sql-planner`
- **职责**：查询规划，AST → 逻辑计划 → 物理计划
- **关键文件**：`src/logical_plan.rs`, `src/physical_plan.rs`, `src/optimizer.rs`
- **优化规则**：谓词下推、列裁剪、Limit 下推、Join 重排序

#### `fe-catalog`
- **职责**：元数据管理（数据库、表、分区）
- **关键文件**：`src/catalog.rs`, `src/database.rs`, `src/table.rs`
- **状态**：当前内存存储，计划持久化到 EditLog

#### `fe-scheduler`
- **职责**：Fragment 规划、分布式查询调度
- **关键文件**：`src/fragment.rs`, `src/scheduler.rs`
- **功能**：负载感知的 BE 节点选择、失败重调度

#### `fe-expression`
- **职责**：向量化表达式求值
- **关键文件**：`src/expression.rs`, `src/functions/`
- **支持**：30+ 内置标量函数、聚合函数、窗口函数

#### `mysql-protocol`
- **职责**：MySQL 线协议服务器
- **关键文件**：`src/server.rs`, `src/protocol.rs`
- **支持**：握手、认证、COM_QUERY、结果集

### Backend Crates

#### `be-storage`
- **职责**：存储引擎（Tablet、Rowset、Segment、Compaction）
- **关键文件**：`src/tablet.rs`, `src/rowset.rs`, `src/segment.rs`, `src/compaction.rs`

#### `be-execution`
- **职责**：Pipeline 执行引擎
- **关键文件**：`src/pipeline.rs`, `src/operators/`
- **算子**：Scan、Filter、Project、Aggregate、Join、Exchange

#### `be-segment`
- **职责**：列式 Segment 格式
- **关键文件**：`src/format.rs`, `src/page.rs`, `src/index.rs`
- **编码**：Plain、RLE、LZ4 压缩
- **索引**：ZoneMap、BloomFilter

### 共享 Crates

#### `types`
- **职责**：数据类型系统
- **关键文件**：`src/vector.rs`, `src/bitmap.rs`, `src/block.rs`, `src/data_type.rs`
- **类型**：Int8/16/32/64、Float32/64、String、Date、Boolean

#### `common`
- **职责**：共享工具函数
- **关键文件**：`src/error.rs`, `src/config.rs`, `src/util.rs`

#### `proto` 和 `rpc`
- **职责**：gRPC 协议定义和实现
- **关键文件**：`proto/*.proto`, `rpc/src/service.rs`

## 开发工作流

### 1. 创建功能分支

```bash
git checkout -b feature/your-feature-name
# 或
git checkout -b fix/your-bug-fix
```

### 2. 进行开发

遵循项目代码风格：
- 使用 `rustfmt` 格式化代码：`cargo fmt`
- 使用 `clippy` 检查代码：`cargo clippy`
- 编写单元测试
- 更新相关文档

### 3. 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test -p fe-sql-parser -- test_function_name

# 检查代码
cargo clippy -- -D warnings
```

### 4. 提交代码

```bash
git add .
git commit -m "feat: add new feature"
# 或
git commit -m "fix: resolve bug"
```

**提交信息规范**（参考 Conventional Commits）：
- `feat:` 新功能
- `fix:` 修复 bug
- `docs:` 文档更新
- `refactor:` 代码重构
- `test:` 测试相关
- `chore:` 构建过程或辅助工具变动

### 5. 推送并创建 Pull Request

```bash
git push origin feature/your-feature-name
```

然后在 GitHub 上创建 Pull Request。

## 添加新功能示例

### 示例 1：添加新的标量函数

假设要添加 `REVERSE` 函数（反转字符串）：

#### 1. 在 `fe-expression/src/functions/` 创建新文件或修改现有文件

```rust
// fe-expression/src/functions/string.rs
pub fn reverse(args: &[Vector]) -> Result<Vector, ExpressionError> {
    if args.len() != 1 {
        return Err(ExpressionError::InvalidArgumentCount(1, args.len()));
    }
    
    match &args[0] {
        Vector::String(v) => {
            let result: Vec<Option<String>> = v.data()
                .iter()
                .map(|opt_s| {
                    opt_s.as_ref().map(|s| s.chars().rev().collect())
                })
                .collect();
            Ok(Vector::String(StringVector::from_vec(result)))
        }
        _ => Err(ExpressionError::InvalidArgumentType("string".to_string())),
    }
}
```

#### 2. 在函数注册表中注册

```rust
// fe-expression/src/functions/mod.rs
pub fn register_functions(registry: &mut FunctionRegistry) {
    // ... 其他函数
    registry.register_scalar("reverse", reverse);
}
```

#### 3. 编写测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reverse() {
        let input = StringVector::from_vec(vec![
            Some("hello".to_string()),
            Some("world".to_string()),
            None,
        ]);
        let result = reverse(&[Vector::String(input)]).unwrap();
        // 验证结果...
    }
}
```

#### 4. 运行测试

```bash
cargo test -p fe-expression -- reverse
```

### 示例 2：添加新的数据类型

假设要添加 `DECIMAL` 类型：

#### 1. 在 `types/src/data_type.rs` 中添加类型定义

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // ... 现有类型
    Decimal(u8, u8), // precision, scale
}
```

#### 2. 在 `types/src/vector.rs` 中添加 Vector 实现

```rust
pub enum Vector {
    // ... 现有类型
    Decimal(DecimalVector),
}
```

#### 3. 更新相关模块（存储、表达式等）

需要更新：
- `be-segment`：序列化/反序列化
- `fe-expression`：支持新类型的表达式求值
- `mysql-protocol`：结果集编码

## 调试技巧

### 使用日志

RorisDB 使用 Rust 的日志系统：

```rust
use log::{info, debug, warn, error};

info!("This is an info message");
debug!("Debug value: {}", value);
warn!("This might be a problem");
error!("Something went wrong: {}", err);
```

设置日志级别：

```bash
# 环境变量
export RUST_LOG=debug
./target/release/roris-fe

# 或配置文件
log_level = debug
```

### 使用 gdb/lldb 调试

```bash
# 使用 debug 模式编译
cargo build

# 使用 lldb 调试（macOS）
lldb ./target/debug/roris-fe

# 在 lldb 中
run --http-port 8030
```

### 查看查询计划

```sql
EXPLAIN SELECT * FROM user WHERE age > 20;
```

## 性能分析

### 使用 benchmark

```bash
# 运行 TPC-H benchmark
cargo bench -p tpch-bench

# 或手动测试
time mysql -h 127.0.0.1 -P 9030 -uroot -e "SELECT COUNT(*) FROM large_table"
```

### 使用 perf（Linux）

```bash
perf record -g ./target/release/roris-be
perf report
```

## 代码审查清单

提交 PR 前，请确保：

- [ ] 代码已通过 `cargo fmt` 格式化
- [ ] 代码已通过 `cargo clippy` 检查（无警告）
- [ ] 已添加必要的单元测试
- [ ] 已更新相关文档
- [ ] 所有测试通过（`cargo test`）
- [ ] 提交信息清晰、符合规范
- [ ] 新功能有适当的错误处理
- [ ] 避免不必要的 `unwrap()` 和 `expect()`

## 发布流程

### 版本号规范

RorisDB 遵循语义化版本（Semantic Versioning）：
- **主版本号**：不兼容的 API 修改
- **次版本号**：向下兼容的功能性新增
- **修订号**：向下兼容的问题修正

示例：`v0.1.3`

### 发布步骤

1. 更新 `Cargo.toml` 中的版本号
2. 更新 `CHANGELOG.md`
3. 创建发布标签：
   ```bash
   git tag v0.1.4
   git push origin v0.1.4
   ```
4. 构建发布版本：
   ```bash
   cargo build --release
   ```
5. 在 GitHub 上创建 Release

## 贡献指南

### 如何贡献

1. **报告 Bug**：在 GitHub Issues 中详细描述问题
2. **功能建议**：在 GitHub Issues 中描述新功能需求
3. **提交代码**：Fork → 创建分支 → 提交 PR
4. **文档改进**：完善文档、修正错误

### 行为准则

- 尊重其他贡献者
- 接受建设性的批评
- 关注项目目标
- 保持代码质量

## 资源链接

- **Rust 官方文档**：https://www.rust-lang.org/learn
- **Cargo 手册**：https://doc.rust-lang.org/cargo/
- **Tokio 异步运行时**：https://tokio.rs/
- **Apache Doris 文档**：https://doris.apache.org/docs/

## 下一步

- 查看[功能特性](features.md)了解项目当前功能
- 阅读[架构设计文档](architecture.md)了解系统设计
- 参考[产品概述](product-overview.md)了解项目定位
