# RorisDB Developer Guide

This document provides guidance for contributors who want to participate in RorisDB development.

## Development Environment Setup

### Prerequisites

- **Rust**: Version 1.75+
- **Cargo**: Matching Rust version
- **Git**: Version control tool
- **IDE**: Recommended VS Code + rust-analyzer plugin, or IntelliJ IDEA + Rust plugin

### Getting the Source Code

```bash
git clone https://github.com/your-repo/RorisDB.git
cd RorisDB
```

### Building the Project

```bash
# Development mode (fast compilation, less optimization)
cargo build

# Release mode (optimized compilation, for performance testing)
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fe-sql-parser
cargo test -p be-storage

# Run integration tests
cargo test --test integration
```

## Project Structure

```
RorisDB/
├── roris-server/          # FE and BE binary entry points
│   └── src/
│       ├── fe_main.rs     # Frontend Server main entry
│       └── be_main.rs    # Backend Server main entry
├── crates/                # Core modules (14 crates)
│   ├── fe-sql-parser/   # SQL parsing → AST
│   ├── fe-sql-planner/  # AST → logical plan → physical plan + optimization
│   ├── fe-catalog/      # Database/table/partition metadata management
│   ├── fe-scheduler/    # Fragment planning, distributed scheduling
│   ├── fe-expression/   # Vectorized expression evaluation
│   ├── fe-common/       # FE shared modules (EditLog, MetaService)
│   ├── mysql-protocol/  # MySQL wire protocol server
│   ├── be-storage/      # Tablet, Rowset, Segment, Compaction
│   ├── be-execution/    # Pipeline execution engine
│   ├── be-segment/      # Columnar Segment format
│   ├── be-common/       # BE shared (configuration, metrics)
│   ├── data-io/         # CSV/JSON import, Stream Load
│   ├── types/           # Vector, Bitmap, Block, DataType, Schema
│   ├── common/          # Error handling, configuration, utility functions
│   ├── proto/           # gRPC protocol definitions (protobuf)
│   └── rpc/             # gRPC service implementation
├── tools/                # Tools and utilities
│   ├── roris-cli/       # Command-line client (REPL)
│   ├── tpch_test/       # TPC-H testing tool
│   └── mysql_server/    # MySQL server tool
├── benches/              # Benchmarks
│   └── tpch/            # TPC-H benchmark suite
├── tests/                # Tests
│   ├── integration/     # SQL and protocol integration tests
│   └── common/          # Test common modules
├── docs/                 # Documentation
├── conf/                 # Configuration files
└── Cargo.toml           # Workspace configuration
```

## Core Crate Descriptions

### Frontend Crates

#### `fe-sql-parser`
- **Responsibility**: SQL parsing, converting SQL text to AST
- **Key files**: `src/parser.rs`, `src/ast.rs`
- **Dependencies**: `sqlparser` crate

#### `fe-sql-planner`
- **Responsibility**: Query planning, AST → logical plan → physical plan
- **Key files**: `src/logical_plan.rs`, `src/physical_plan.rs`, `src/optimizer.rs`
- **Optimization rules**: Predicate pushdown, column pruning, Limit pushdown, Join reordering

#### `fe-catalog`
- **Responsibility**: Metadata management (database, table, partition)
- **Key files**: `src/catalog.rs`, `src/database.rs`, `src/table.rs`
- **Status**: Currently in-memory storage, planning to persist to EditLog

#### `fe-scheduler`
- **Responsibility**: Fragment planning, distributed query scheduling
- **Key files**: `src/fragment.rs`, `src/scheduler.rs`
- **Features**: Load-aware BE node selection, failure rescheduling

#### `fe-expression`
- **Responsibility**: Vectorized expression evaluation
- **Key files**: `src/expression.rs`, `src/functions/`
- **Support**: 30+ built-in scalar functions, aggregate functions, window functions

#### `mysql-protocol`
- **Responsibility**: MySQL wire protocol server
- **Key files**: `src/server.rs`, `src/protocol.rs`
- **Support**: Handshake, authentication, COM_QUERY, result set

### Backend Crates

#### `be-storage`
- **Responsibility**: Storage engine (Tablet, Rowset, Segment, Compaction)
- **Key files**: `src/tablet.rs`, `src/rowset.rs`, `src/segment.rs`, `src/compaction.rs`

#### `be-execution`
- **Responsibility**: Pipeline execution engine
- **Key files**: `src/pipeline.rs`, `src/operators/`
- **Operators**: Scan, Filter, Project, Aggregate, Join, Exchange

#### `be-segment`
- **Responsibility**: Columnar Segment format
- **Key files**: `src/format.rs`, `src/page.rs`, `src/index.rs`
- **Encoding**: Plain, RLE, LZ4 compression
- **Indexing**: ZoneMap, BloomFilter

### Shared Crates

#### `types`
- **Responsibility**: Data type system
- **Key files**: `src/vector.rs`, `src/bitmap.rs`, `src/block.rs`, `src/data_type.rs`
- **Types**: Int8/16/32/64, Float32/64, String, Date, Boolean

#### `common`
- **Responsibility**: Shared utility functions
- **Key files**: `src/error.rs`, `src/config.rs`, `src/util.rs`

#### `proto` and `rpc`
- **Responsibility**: gRPC protocol definitions and implementation
- **Key files**: `proto/*.proto`, `rpc/src/service.rs`

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/your-bug-fix
```

### 2. Development

Follow the project code style:
- Use `rustfmt` to format code: `cargo fmt`
- Use `clippy` to check code: `cargo clippy`
- Write unit tests
- Update relevant documentation

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run specific tests
cargo test -p fe-sql-parser -- test_function_name

# Check code
cargo clippy -- -D warnings
```

### 4. Commit Code

```bash
git add .
git commit -m "feat: add new feature"
# or
git commit -m "fix: resolve bug"
```

**Commit Message Convention** (refer to Conventional Commits):
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation update
- `refactor:` Code refactoring
- `test:` Test related
- `chore:` Build process or auxiliary tool changes

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Adding New Features Examples

### Example 1: Adding a New Scalar Function

Suppose you want to add a `REVERSE` function (reverse a string):

#### 1. Create a new file or modify an existing file in `fe-expression/src/functions/`

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

#### 2. Register in the function registry

```rust
// fe-expression/src/functions/mod.rs
pub fn register_functions(registry: &mut FunctionRegistry) {
    // ... other functions
    registry.register_scalar("reverse", reverse);
}
```

#### 3. Write tests

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
        // Verify results...
    }
}
```

#### 4. Run tests

```bash
cargo test -p fe-expression -- reverse
```

### Example 2: Adding a New Data Type

Suppose you want to add a `DECIMAL` type:

#### 1. Add type definition in `types/src/data_type.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // ... existing types
    Decimal(u8, u8), // precision, scale
}
```

#### 2. Add Vector implementation in `types/src/vector.rs`

```rust
pub enum Vector {
    // ... existing types
    Decimal(DecimalVector),
}
```

#### 3. Update related modules (storage, expression, etc.)

Need to update:
- `be-segment`: Serialization/deserialization
- `fe-expression`: Expression evaluation for new type
- `mysql-protocol`: Result set encoding

## Debugging Tips

### Using Logs

RorisDB uses Rust's logging system:

```rust
use log::{info, debug, warn, error};

info!("This is an info message");
debug!("Debug value: {}", value);
warn!("This might be a problem");
error!("Something went wrong: {}", err);
```

Set log level:

```bash
# Environment variable
export RUST_LOG=debug
./target/release/roris-fe

# Or configuration file
log_level = debug
```

### Using gdb/lldb Debugger

```bash
# Compile in debug mode
cargo build

# Use lldb for debugging (macOS)
lldb ./target/debug/roris-fe

# In lldb
run --http-port 8030
```

### Viewing Query Plans

```sql
EXPLAIN SELECT * FROM user WHERE age > 20;
```

## Performance Analysis

### Using Benchmarks

```bash
# Run TPC-H benchmark
cargo bench -p tpch-bench

# Or manual testing
time mysql -h 127.0.0.1 -P 9030 -uroot -e "SELECT COUNT(*) FROM large_table"
```

### Using perf (Linux)

```bash
perf record -g ./target/release/roris-be
perf report
```

## Code Review Checklist

Before submitting a PR, please ensure:

- [ ] Code has been formatted with `cargo fmt`
- [ ] Code has been checked with `cargo clippy` (no warnings)
- [ ] Necessary unit tests have been added
- [ ] Relevant documentation has been updated
- [ ] All tests pass (`cargo test`)
- [ ] Commit messages are clear and follow conventions
- [ ] New features have appropriate error handling
- [ ] Avoid unnecessary `unwrap()` and `expect()`

## Release Process

### Version Numbering Convention

RorisDB follows Semantic Versioning:
- **Major version**: Incompatible API changes
- **Minor version**: Backward-compatible functionality additions
- **Patch version**: Backward-compatible bug fixes

Example: `v0.1.3`

### Release Steps

1. Update version number in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create release tag:
   ```bash
   git tag v0.1.4
   git push origin v0.1.4
   ```
4. Build release version:
   ```bash
   cargo build --release
   ```
5. Create Release on GitHub

## Contribution Guidelines

### How to Contribute

1. **Report Bugs**: Describe the issue in detail in GitHub Issues
2. **Feature Suggestions**: Describe new feature requirements in GitHub Issues
3. **Submit Code**: Fork → Create branch → Submit PR
4. **Documentation Improvements**: Improve documentation, fix errors

### Code of Conduct

- Respect other contributors
- Accept constructive criticism
- Focus on project goals
- Maintain code quality

## Resources

- **Rust Official Documentation**: https://www.rust-lang.org/learn
- **Cargo Manual**: https://doc.rust-lang.org/cargo/
- **Tokio Async Runtime**: https://tokio.rs/
- **Apache Doris Documentation**: https://doris.apache.org/docs/

## Next Steps

- Check [Feature List](features.md) to understand current project features
- Read [Architecture Design Document](architecture.md) to understand system design
- Refer to [Product Overview](product-overview.md) to understand project positioning
