# RorisDB Documentation / RorisDB 文档中心

Welcome to the RorisDB Documentation Center. Here you will find all the documentation needed to use, develop, and administer RorisDB.

欢迎来到 RorisDB 文档中心。这里包含了使用、开发和管理 RorisDB 所需的所有文档。

---

## 📖 Documentation Index / 文档索引

### English Documentation / 英文文档
📁 **[English Docs](./en/)**

| Document | Description |
|----------|-------------|
| [README](./en/README.md) | Documentation index and navigation |
| [Product Overview](./en/product-overview.md) | What is RorisDB, core features, and use cases |
| [Architecture](./en/architecture.md) | System architecture, module design, execution flow |
| [Installation](./en/installation.md) | Build, install, and deploy RorisDB (standalone/distributed) |
| [Getting Started](./en/getting-started.md) | From connecting to database to running your first query |
| [SQL Reference](./en/sql-reference.md) | Complete SQL syntax, functions, and operations guide |
| [Configuration](./en/configuration.md) | FE/BE configuration parameters explained |
| [Developer Guide](./en/developer-guide.md) | Development environment, project structure, contribution workflow |
| [Features](./en/features.md) | Detailed feature list and implementation status |
| [Performance](./en/performance.md) | TPC-H benchmarks and optimization notes |
| [Compatibility Matrix](./en/compatibility-matrix.md) | Feature comparison with Apache Doris |

### 中文文档 / Chinese Documentation
📁 **[中文文档](./zh/)**

| 文档 | 说明 |
|------|------|
| [README](./zh/README.md) | 文档索引和导航 |
| [产品概述](./zh/product-overview.md) | RorisDB 是什么、核心特性和适用场景 |
| [架构设计](./zh/architecture.md) | 系统架构、模块设计、执行流程 |
| [安装部署](./zh/installation.md) | 编译、安装和部署 RorisDB（单机/分布式） |
| [快速开始](./zh/getting-started.md) | 从连接数据库到执行第一个查询 |
| [SQL 参考手册](./zh/sql-reference.md) | 完整的 SQL 语法、函数和操作指南 |
| [配置说明](./zh/configuration.md) | FE/BE 配置参数详解 |
| [开发者指南](./zh/developer-guide.md) | 开发环境搭建、项目结构、贡献流程 |
| [功能特性](./zh/features.md) | 详细的功能列表和实现状态 |
| [性能报告](./zh/performance.md) | TPC-H 基准测试和优化说明 |
| [编译与打包方案](./zh/build-plan.md) | 编译、打包和分发说明 |
| [改进计划](./zh/改进计划.md) | 未来改进方向和待办事项 |

---

## Quick Navigation by Audience / 按读者类型导航

### 🆕 New Users / 新用户
- **English**: [Product Overview](./en/product-overview.md) → [Installation](./en/installation.md) → [Getting Started](./en/getting-started.md)
- **中文**: [产品概述](./zh/product-overview.md) → [安装部署](./zh/installation.md) → [快速开始](./zh/getting-started.md)

### 🔧 Database Administrators / 数据库管理员
- **English**: [Installation](./en/installation.md) → [Configuration](./en/configuration.md) → [SQL Reference](./en/sql-reference.md)
- **中文**: [安装部署](./zh/installation.md) → [配置说明](./zh/configuration.md) → [SQL 参考手册](./zh/sql-reference.md)

### 👨‍💻 Developers / 开发者
- **English**: [Architecture](./en/architecture.md) → [Developer Guide](./en/developer-guide.md) → [Features](./en/features.md)
- **中文**: [架构设计](./zh/architecture.md) → [开发者指南](./zh/developer-guide.md) → [功能特性](./zh/features.md)

### 🏗️ Architects / 架构师
- **English**: [Product Overview](./en/product-overview.md) → [Architecture](./en/architecture.md) → [Compatibility Matrix](./en/compatibility-matrix.md)
- **中文**: [产品概述](./zh/product-overview.md) → [架构设计](./zh/architecture.md) → [兼容性矩阵](./zh/compatibility-matrix.md) (待创建)

### 📊 Performance Engineers / 性能工程师
- **English**: [Performance](./en/performance.md) → [Architecture](./en/architecture.md) → [Developer Guide](./en/developer-guide.md)
- **中文**: [性能报告](./zh/performance.md) → [架构设计](./zh/architecture.md) → [开发者指南](./zh/developer-guide.md)

---

## Version Information / 版本信息

- **Current Version / 当前版本**: v0.1.3
- **Project Status / 项目状态**: Proof-of-Concept (概念验证阶段)
- **License / 许可证**: MIT / Apache-2.0

---

## Quick Links / 快速链接

- **GitHub Repository / GitHub 仓库**: [RorisDB on GitHub](https://github.com/your-repo/RorisDB)
- **Issue Tracking / 问题反馈**: [GitHub Issues](https://github.com/your-repo/RorisDB/issues)
- **Contribution Guide / 贡献指南**: See [Developer Guide](./en/developer-guide.md) / 参见[开发者指南](./zh/developer-guide.md)

---

## Documentation Structure / 文档结构

```
docs/
├── README.md                    # This file (bilingual index)
│
├── en/                          # English Documentation
│   ├── README.md               # Documentation index
│   ├── product-overview.md
│   ├── architecture.md
│   ├── installation.md
│   ├── getting-started.md
│   ├── sql-reference.md
│   ├── configuration.md
│   ├── developer-guide.md
│   ├── features.md
│   ├── performance.md
│   └── compatibility-matrix.md
│
└── zh/                          # 中文文档
    ├── README.md               # 文档索引
    ├── product-overview.md
    ├── architecture.md
    ├── installation.md
    ├── getting-started.md
    ├── sql-reference.md
    ├── configuration.md
    ├── developer-guide.md
    ├── features.md
    ├── performance.md
    ├── build-plan.md           # 编译与打包方案
    └── 改进计划.md             # 改进计划
```

---

## How to Contribute Documentation / 如何贡献文档

We welcome contributions to improve the documentation!

欢迎贡献和改进文档！

### English
1. Fork the repository
2. Create a feature branch
3. Modify or add documentation in `docs/en/`
4. Submit a Pull Request

### 中文
1. Fork 仓库
2. 创建功能分支
3. 修改或添加 `docs/zh/` 中的文档
4. 提交 Pull Request

Documentation is written in Markdown format.

文档使用 Markdown 格式编写。

---

## Need Help? / 需要帮助？

- **English**: Check the [Getting Started](./en/getting-started.md) guide or open an [issue](https://github.com/your-repo/RorisDB/issues)
- **中文**: 查看[快速开始](./zh/getting-started.md)文档，或在 [GitHub Issues](https://github.com/your-repo/RorisDB/issues) 提问

---

**RorisDB** = **R**ust + (D)**oris** + **DB**
