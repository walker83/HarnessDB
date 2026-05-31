# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.3.2]: https://github.com/walker83/RorisDB/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/walker83/RorisDB/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/walker83/RorisDB/releases/tag/v0.3.0
