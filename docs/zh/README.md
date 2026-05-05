# RorisDB 文档中心

欢迎来到 RorisDB 文档中心。这里包含了使用、开发和管理 RorisDB 所需的所有文档。

## 快速导航

### 📖 产品介绍
- [产品概述](product-overview.md) - 了解 RorisDB 是什么、核心特性和适用场景
- [功能特性](features.md) - 详细的功能列表和当前实现状态

### 🏗️ 架构与原理
- [架构设计文档](architecture.md) - 深入了解系统架构、模块设计和执行流程

### 🚀 快速开始
- [安装部署指南](installation.md) - 编译、安装和部署 RorisDB（单机/分布式）
- [快速开始](getting-started.md) - 从连接数据库到执行第一个查询

### 📝 SQL 参考
- [SQL 参考手册](sql-reference.md) - 完整的 SQL 语法、函数和操作指南

### ⚙️ 配置与管理
- [配置说明](configuration.md) - FE/BE 配置参数详解

### 👨‍💻 开发者资源
- [开发者指南](developer-guide.md) - 开发环境搭建、项目结构、贡献流程

### 📊 性能与兼容性
- [性能报告](performance.md) - TPC-H 基准测试和优化说明
- [兼容性矩阵](compatibility-matrix.md) - 与 Apache Doris 的功能对比

### 📋 其他文档
- [编译与打包方案](build-plan.md) - 编译、打包和分发说明
- [改进计划](改进计划.md) - 未来改进方向和待办事项

## 文档结构

```
docs/
├── README.md                    # 本文档（文档索引）
├── product-overview.md          # 产品概述
├── architecture.md              # 架构设计文档
├── installation.md              # 安装部署指南
├── getting-started.md           # 快速开始
├── sql-reference.md             # SQL 参考手册
├── configuration.md             # 配置说明
├── developer-guide.md           # 开发者指南
├── features.md                  # 功能特性
├── performance.md               # 性能报告
├── compatibility-matrix.md      # 兼容性矩阵（与 Doris 对比）
├── build-plan.md                # 编译与打包方案
└── 改进计划.md                 # 改进计划
```

## 版本信息

- **当前版本**：v0.2.0
- **项目状态**：Proof-of-Concept（概念验证阶段）
- **License**：MIT / Apache-2.0

## 快速链接

- **GitHub 仓库**：[RorisDB on GitHub](https://github.com/your-repo/RorisDB)
- **问题反馈**：[GitHub Issues](https://github.com/your-repo/RorisDB/issues)
- **贡献指南**：参见[开发者指南](developer-guide.md)

## 适用读者

| 读者类型 | 推荐文档 |
|---------|---------|
| **新用户** | [产品概述](product-overview.md) → [安装部署](installation.md) → [快速开始](getting-started.md) |
| **数据库管理员** | [安装部署](installation.md) → [配置说明](configuration.md) → [SQL 参考](sql-reference.md) |
| **开发者** | [架构设计](architecture.md) → [开发者指南](developer-guide.md) → [功能特性](features.md) |
| **架构师** | [产品概述](product-overview.md) → [架构设计](architecture.md) → [兼容性矩阵](compatibility-matrix.md) |
| **性能工程师** | [性能报告](performance.md) → [架构设计](architecture.md) → [开发者指南](developer-guide.md) |

## 如何贡献文档

欢迎贡献和改进文档！请参考[开发者指南](developer-guide.md)中的贡献流程：

1. Fork 仓库
2. 创建功能分支
3. 修改或添加文档
4. 提交 Pull Request

文档使用 Markdown 格式编写。

## 更新日志

- **2026-05-05**：更新至 v0.2.0 - 新增外部 Catalog、认证框架、CBO 优化器、分区支持、物化视图、Runtime Filter、ALTER TABLE、备份恢复等功能
- **2026-05-04**：创建完整文档体系，包括产品概述、架构设计、安装指南、SQL 参考等
- **2026-05-04**：添加兼容性矩阵和性能报告

## 需要帮助？

- 查看[快速开始](getting-started.md)文档
- 在 [GitHub Issues](https://github.com/your-repo/RorisDB/issues) 提问
- 联系开发团队

---

**RorisDB** = **R**ust + (D)**oris** + **DB**
