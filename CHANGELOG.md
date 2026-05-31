# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - 2026-05-31

### Bug Fixes
- **[严重] MySQL 协议 DEPRECATE_EOF 结果集终止包头字节错误**: 修复 MySQL 8.0+ 客户端连接后查询永久挂起的严重 bug
  - 当 `CLIENT_DEPRECATE_EOF` 能力标志协商成功后（MySQL 8.0+ 默认行为），结果集终止包使用了 `0x00`（OK 包头字节）而非协议规定的 `0xFE`
  - 客户端无法识别结果集结束，导致永久等待 → 所有 SELECT 查询挂起
  - 新增 `make_result_set_eof_ok_packet()` 函数，使用 `0xFE` 头字节 + OK 风格 lenenc 字段
  - 同时修复 `send_result_set()` 和 `send_binary_result_set()` 两处调用点
  - 影响范围：所有通过 MySQL 8.0+ 客户端的连接（包括 mysql CLI、JDBC、Go MySQL Driver 等）

### Code Quality
- `mysql-protocol`: 分离 OK 包（`0x00`）与结果集终止包（`0xFE`）的构造逻辑，符合 MySQL 内部协议规范

## [0.3.2] - 2026-05-31

### Bug Fixes
- **DATE/DATETIME 类型修复**: 修复 `scalar_to_text_bytes()` 函数硬编码返回 "1970-01-01" 的问题
  - Date(i32) 现在正确转换为 "YYYY-MM-DD" 格式
  - DateTime(i64) 现在正确转换为 "YYYY-MM-DD HH:MM:SS" 格式
  - 修复了通过 MySQL 二进制协议查询时日期值错误的问题
  - Fixes GitHub issue #1

### Code Quality
- 消除所有编译器警告，实现零警告 release 构建
  - 清理未使用的导入和变量
  - 为预留功能添加 `#[allow(dead_code)]` 标记
  - 迁移弃用的 DataFusion API (DFParser -> DFParserBuilder)
  - 修复 `drop(&reference)` 无效操作
  - 处理未使用的 `Result` 值

### Internal Changes
- `mysql-protocol`: 添加 chrono 依赖用于日期计算
- `fe-sql-parser`: 升级 DFParser API 到新版本的 DFParserBuilder
- 多个 crate 的代码清理和优化

## [0.3.1] - 2026-05-31

### Bug Fixes
- DATE/DATETIME 类型修复（初始版本）

## [0.3.0] - 2026-05-30

### Features
- 完整中文文档翻译
- 多数据库协议兼容性强调（MySQL、Hologres、MaxCompute）

[0.3.3]: https://github.com/walker83/RorisDB/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/walker83/RorisDB/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/walker83/RorisDB/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/walker83/RorisDB/releases/tag/v0.3.0
