# RorisDB Architecture Design Document

## System Architecture Overview

RorisDB adopts a classic FE (Frontend) + BE (Backend) distributed architecture, designed based on the MPP (Massively Parallel Processing) model.

### Overall Architecture

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

## FE (Frontend) Architecture

### Core Modules

#### 1. SQL Parser (`fe-sql-parser`)
- Implemented based on the `sqlparser` crate
- Parses SQL text into AST (Abstract Syntax Tree)
- Supports MySQL-compatible SQL syntax

#### 2. Query Planner (`fe-sql-planner`)
- AST → Logical Plan → Physical Plan
- Rule-based optimization (RBO):
  - Predicate Pushdown
  - Column Pruning
  - Limit Pushdown
  - Join Reordering

#### 3. Catalog (`fe-catalog`)
- Manages metadata for databases, tables, partitions, etc.
- Currently uses in-memory storage (planned persistence to EditLog + BDBJE)
- Supports creation and querying of databases, tables, and views

#### 4. Scheduler (`fe-scheduler`)
- Fragment planning: Splits physical plans into distributable Fragments
- Distributed scheduling:
  - Load-aware BE node selection
  - Round-robin allocation strategy
  - Failure rescheduling

#### 5. Expression Engine (`fe-expression`)
- Vectorized expression evaluation
- 30+ built-in scalar functions
- Batch processing for optimized CPU cache utilization

#### 6. MySQL Protocol (`mysql-protocol`)
- Implements MySQL wire protocol
- Supports handshake, authentication, query, and result set return
- Compatible with MySQL client tools

### FE Main Workflow

```
SQL Text
    ↓
[Parser] → AST
    ↓
[Planner] → Logical Plan → Physical Plan
    ↓
[Optimizer] → Optimized Physical Plan
    ↓
[Scheduler] → Fragments + Execution Plan
    ↓
[Coordinator] → Distribute to BE for execution
    ↓
[Collector] → Collect results and return to client
```

## BE (Backend) Architecture

### Core Modules

#### 1. Storage Engine (`be-storage`)
- **Tablet**: Basic unit of data sharding
- **Rowset**: Collection of data generated from a single import or Compaction
- **Segment**: Columnar storage file containing multiple Column Pages
- **MemTable**: In-memory buffer for real-time writes

Storage hierarchy:
```
Table
  └── Partition (optional)
       └── Tablet (shard)
            └── Rowset
                 └── Segment
                      ├── Column Page (columnar data)
                      ├── ZoneMap Index (range index)
                      ├── BloomFilter Index (bloom filter)
                      └── Null Bitmap
```

#### 2. Segment Format (`be-segment`)
- Columnar storage format
- Supports multiple encodings:
  - **Plain Encoding**: Direct storage
  - **RLE (Run-Length Encoding)**: Run-length encoding, suitable for repeated values
  - **LZ4 Compression**: Lightweight compression
- Index support:
  - **ZoneMap**: Records min/max values for each column, used for range filtering
  - **BloomFilter**: Used for equality filtering on high-cardinality columns

#### 3. Execution Engine (`be-execution`)
- Pipeline execution model
- Vectorized execution: Batch processing of data
- Operator types:
  - **Scan**: Scan table data
  - **Filter**: Filter data
  - **Project**: Project columns
  - **Aggregate**: Aggregation calculations
  - **Join**: Join operations (Hash Join, Nested Loop Join)
  - **Exchange**: Data exchange (HashPartition, Broadcast, Gather)

#### 4. Compaction
- **Cumulative Compaction**: Merges small files, quickly merges latest data
- **Base Compaction**: Merges large files, optimizes query performance
- Priority queue-based scheduling strategy

### BE Main Workflow

```
[Receive Fragment]
    ↓
[Pipeline Builder] → Build execution Pipeline
    ↓
[Executor] → Vectorized execution
    ↓
[Operator Chain] → Scan → Filter → Aggregate → ...
    ↓
[Result Sender] → Send results to FE
```

## Data Type System (`types`)

### Basic Types
- **Integer types**: Int8, Int16, Int32, Int64
- **Floating-point types**: Float32, Float64
- **String type**: String (UTF-8)
- **Date/Time**: Date, DateTime
- **Boolean type**: Boolean
- **Null value**: Null (tracked via Null Bitmap)

### Vectorized Representation
- Each type corresponds to a Vector implementation
- Batch storage of data for optimized cache locality
- Supports Null Bitmap for tracking null values

```rust
pub enum Vector {
    Int64(Int64Vector),
    Float64(Float64Vector),
    String(StringVector),
    Boolean(BooleanVector),
    // ...
}
```

## Expression System (`fe-expression`)

### Expression Types
- **Literal**: Constant expression
- **ColumnRef**: Column reference
- **BinaryOp**: Binary operation (+, -, *, /, etc.)
- **UnaryOp**: Unary operation (NOT, -, etc.)
- **FunctionCall**: Function call
- **Cast**: Type conversion
- **Subquery**: Subquery

### Vectorized Evaluation
Expression evaluation uses batch processing mode:

```rust
impl Expression for BinaryOpExpr {
    fn eval(&self, batch: &Batch) -> Vector {
        let left = self.left.eval(batch);
        let right = self.right.eval(batch);
        // Batch computation, processing multiple rows at once
        vector_binary_op(&left, &right, self.op)
    }
}
```

## Query Execution Flow

### 1. SQL Parsing Phase
```sql
SELECT age, COUNT(*) FROM user WHERE age > 20 GROUP BY age
```
↓
```rust
AST: Query {
  select: [ColumnRef("age"), FunctionCall(COUNT, *)],
  from: Table("user"),
  filter: BinaryOp(ColumnRef("age"), >, Literal(20)),
  group_by: [ColumnRef("age")]
}
```

### 2. Logical Plan Phase
```
LogicalPlan:
  Aggregate {
    group_by: [age],
    aggr_exprs: [COUNT(*)],
    input: Filter {
      predicate: age > 20,
      input: Scan { table: "user" }
    }
  }
```

### 3. Physical Plan Phase
```
PhysicalPlan:
  HashAggregate {
    group_by: [age],
    aggr_exprs: [COUNT(*)],
    input: Filter {
      predicate: age > 20,
      input: TableScan { table: "user", projections: [age] }
    }
  }
```

### 4. Fragment Splitting
```
Fragment 1 (BE Local):
  TableScan → Filter → HashAggregate (Partial)
  
Fragment 2 (BE Local):
  HashAggregate (Final) → Output
  
Exchange: HashPartition (by age) from Fragment 1 → Fragment 2
```

### 5. Distributed Execution
- FE distributes Fragments to multiple BE nodes
- Each BE executes local Fragments
- Data exchange via Exchange operators
- FE collects final results and returns to client

## Data Import

### Supported Formats
- **CSV**: Comma-separated values
- **JSON Lines**: One JSON object per line
- **Stream Load**: HTTP streaming import

### Import Workflow
```
Client
  ↓
[FE] Receive import request
  ↓
[BE] Write data to MemTable
  ↓
[BE] Flush to disk (generate Rowset/Segment)
  ↓
[BE] Trigger Compaction (optional)
```

## Network Protocols

### MySQL Protocol (`mysql-protocol`)
- Supports MySQL handshake and authentication
- Supports commands like COM_QUERY, COM_PING, COM_QUIT, etc.
- Returns standard MySQL result set format

### gRPC Protocol (`rpc`)
- Communication between FE and BE via gRPC
- Uses Protocol Buffers for message format definition
- Main services:
  - `BackendService`: BE registration, heartbeat, query execution
  - `QueryService`: Query coordination and execution

## Cluster Management

### BE Node Management
- BE nodes register with FE on startup
- Send periodic heartbeats (including load information)
- FE tracks load score for each BE
- Selects BE nodes with lower load during query scheduling

### High Availability (Planned)
- Current FE is a single point
- Planning to implement FE metadata replication based on Raft
- BE nodes support multiple replicas

## Storage Format Details

### Segment File Structure
```
Segment File:
├── Header (magic number, version)
├── Column Pages
│   ├── Page 1: column data + metadata
│   ├── Page 2: column data + metadata
│   └── ...
├── ZoneMap Index
│   ├── min value per column
│   ├── max value per column
│   └── null count
├── BloomFilter Index (optional)
│   └── bloom filter per column
└── Footer (offset table, checksum)
```

### Compaction Strategy
1. **Cumulative Compaction**
   - Merges small Rowsets (recently imported data)
   - Fast merge, reduces number of small files
   - Trigger condition: Number of Rowsets exceeds threshold

2. **Base Compaction**
   - Merges large Rowsets (historical data)
   - Deep merge, optimizes query performance
   - Trigger condition: Too many Cumulative files or periodic trigger

## Performance Optimization Techniques

### Vectorized Execution
- Batch processing of data (Batch size = 1024 or larger)
- Reduces function call overhead
- Improves CPU cache hit rate

### Zero-Copy
- Uses Rust's borrowing mechanism to avoid data copying
- Uses references instead of ownership transfer where possible

### Late Materialization
- Materialize data only when necessary
- Filter data early to reduce the amount of data for subsequent processing

### Index Optimization
- ZoneMap: Quickly skip Segments that don't satisfy range conditions
- BloomFilter: Quickly determine if a value exists in a Segment
- Column pruning: Only read columns needed by the query

## Architecture Comparison with Apache Doris

| Architecture Component | Apache Doris | RorisDB |
|----------------------|--------------|---------|
| Language | C++ | Rust |
| FE Metadata | BDBJE | EditLog (BDBJE planned) |
| High Availability | BDBJE Master/Follower | Raft (planned) |
| Storage Format | Tablet/Rowset/Segment | Tablet/Rowset/Segment |
| Execution Model | Vectorized + Pipeline | Vectorized + Pipeline |
| Network Protocol | MySQL + Thrift | MySQL + gRPC |
| Compression Algorithms | zstd, LZ4, Zlib | LZ4 (more planned) |

## Future Roadmap

### Short-term (v0.2)
- Catalog persistence (EditLog + BDBJE)
- Materialized view transparent query rewriting
- HA high availability (Raft consensus)

### Medium-term (v0.3)
- Federated query (Hive/Iceberg/Hudi)
- More compression algorithms (zstd, Zlib)
- Cloud-native mode (S3 shared storage)

### Long-term
- Kubernetes Operator
- Multi-database transactions
- UDF/UDAF support
- Row-level security
