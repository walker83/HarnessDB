# RorisDB Product Overview

## What is RorisDB

RorisDB is a **real-time OLAP (Online Analytical Processing) database** reimplemented in Rust, with its architecture inspired by Apache Doris.

**RorisDB** = **R**ust + (D)**oris** + **DB**

### Project Positioning

RorisDB is not a fork or wrapper of Apache Doris, but an independently reimplemented open-source project. It is rewritten in Rust with the following goals:

- **Memory Safety**: Prevent memory safety vulnerabilities through Rust's ownership system
- **Zero-cost Abstractions**: Maintain code maintainability while achieving high performance
- **Fine-grained Resource Control**: Asynchronous I/O based on Tokio and configurable memory limits

### Core Features

- **Real-time Analytics**: Supports real-time data ingestion and sub-second query response
- **Columnar Storage**: Adopts columnar memory layout to optimize analytical query performance
- **Vectorized Execution**: Batch processing of data, fully utilizing CPU caches and SIMD instructions
- **Distributed Architecture**: MPP (Massively Parallel Processing) architecture with horizontal scaling support
- **MySQL Compatible**: Supports MySQL protocol, allowing direct connection using MySQL clients
- **Rich Data Types**: Supports various data types including numeric, string, date, and more
- **Full SQL Support**: Supports complex queries, aggregations, window functions, and more

## Why Choose RorisDB

### Relationship with Apache Doris

RorisDB draws upon the proven design concepts of Apache Doris in its architecture:

- Same MPP architecture
- Similar columnar storage format (Tablet → Rowset → Segment)
- Same Compaction strategy (Cumulative + Base)

However, RorisDB is a completely independent implementation, rewritten in Rust, bringing the following advantages:

| Comparison Dimension | Apache Doris (C++) | RorisDB (Rust) |
|---------------------|-------------------|----------------|
| Memory Safety | Requires careful management | Guaranteed at compile time |
| Concurrency Safety | Relies on developer experience | Guaranteed by borrow checker |
| Cloud-native Deployment | Supported | No unsafe code paths |
| Resource Management | Manual control | Fine-grained control |

### Use Cases

RorisDB is suitable for the following scenarios:

- **Real-time Data Analytics**: Businesses requiring fast analysis of the latest data
- **Data Warehousing**: Building enterprise-grade data warehouses
- **BI Reporting**: Supports ad-hoc queries for business intelligence tools
- **User Behavior Analytics**: Analyzing user behavior, clickstreams, and other data
- **Log Analytics**: Real-time analysis of system logs, application logs

## Technical Architecture

RorisDB adopts the classic FE (Frontend) + BE (Backend) architecture:

```
┌─────────────────────────────────────────────────────────────────┐
│                        RorisDB Cluster                            │
│                                                                 │
│   ┌──────────────┐                        ┌──────────────────┐    │
│   │  FE (Rust)  │◄────── RPC ─────────►│   BE 1 (Rust)   │    │
│   │              │     (gRPC)            │                  │    │
│   │  ┌────────┐ │                        │  ┌────────────┐  │    │
│   │  │ Parser │ │                        │  │ Storage   │  │    │
│   │  │Planner │ │                        │  │ Engine    │  │    │
│   │  │Scheduler│ │                        │  │ (Segment) │  │    │
│   │  │Catalog │ │                        │  └────────────┘  │    │
│   │  └────────┘ │                        │  ┌────────────┐  │    │
│   │              │                        │  │ Execution  │  │    │
│   │  MySQL      │                        │  │ Pipeline   │  │    │
│   │  Protocol   │                        │  └────────────┘  │    │
│   └──────────────┘                        └──────────────────┘    │
│          │                                        │               │
│          │             ┌──────────────────┐    │               │
│          └───────────►│   BE 2 (Rust)   │◄───┘               │
│                         └──────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
```

### FE (Frontend) Responsibilities

- SQL parsing and syntax analysis
- Query planning and optimization
- Metadata management (databases, tables, partitions)
- Query scheduling and coordination
- Cluster management (BE node registration, heartbeats)

### BE (Backend) Responsibilities

- Data storage and management (Tablet, Rowset, Segment)
- Query execution (Pipeline execution engine)
- Data compression and Compaction
- Data import and export

## Current Status

RorisDB is currently in the **Proof-of-Concept** stage (v0.1.3), with core features largely completed, including:

- ✅ SQL parsing and query planning
- ✅ Vectorized expression engine
- ✅ Aggregate functions and window functions
- ✅ Columnar storage engine
- ✅ MySQL protocol support
- ✅ Distributed query scheduling
- ✅ Data import (CSV, JSON)

Work in progress:

- 🚧 Materialized views
- 🚧 High Availability (HA) based on Raft
- 🚧 Catalog persistence
- 🚧 Federated queries (Hive/Iceberg/Hudi)

## License

RorisDB is dual-licensed:

- MIT License
- Apache License 2.0

Users can choose either license for using this project.

## Community and Contributions

Contributions, issue reports, and suggestions are welcome!

- GitHub: [RorisDB Repository](https://github.com/your-repo/RorisDB)
- Issue Reporting: Via GitHub Issues
- Contribution Guide: See [Developer Guide](developer-guide.md)
