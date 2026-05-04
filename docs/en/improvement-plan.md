# RorisDB Improvement Plan

> **Version**: v1.0  
> **Created**: 2025-05-04  
> **Status**: Pending Implementation  
> **Estimated Duration**: 6-8 weeks  
> **Code Scale**: ~20,000 lines of Rust

---

## 1. Executive Summary

This improvement plan is based on a comprehensive review of the RorisDB codebase, identifying **25+ specific improvements** covering performance optimization, architectural improvements, code quality, and test coverage.

### Expected Benefits

| Dimension | Current State | Improvement Target | Expected Gain |
|-----------|--------------|-------------------|---------------|
| Query Performance | Row-by-row scalar computation | Vectorization + SIMD | **30-50%** |
| Concurrent Throughput | RwLock blocking | Lock-free structures | **2-3x** |
| Code Quality | 80+ Clippy warnings | Zero warnings | **100%** |
| Test Coverage | Unit tests only | Integration + performance tests | **3x** |
| Memory Safety | 0 unsafe | Maintain 0 unsafe | **Maintain** |

---

## 2. Priority Matrix

### P0 - Immediate Fixes (1-2 days)

| # | Improvement | Scope | Estimated Effort |
|---|-------------|-------|------------------|
| 1 | Fix all Clippy warnings | Global | 2-4 hours |
| 2 | Add Default trait implementations | types, be-storage | 1-2 hours |
| 3 | Clean up unused imports and variables | Global | 1 hour |
| 4 | Fix redundant closures and code patterns | fe-expression, be-storage | 2 hours |

### P1 - Performance Optimization (1-2 weeks)

| # | Improvement | Scope | Estimated Effort |
|---|-------------|-------|------------------|
| 5 | Vectorized aggregation | AggregateExecNode | 2-3 days |
| 6 | Vectorized sorting | SortExecNode | 2-3 days |
| 7 | SIMD Bitmap operations | Bitmap | 3-4 days |
| 8 | StringVector zero-copy optimization | StringVector | 2-3 days |
| 9 | MemTable structure optimization | Tablet | 3-4 days |
| 10 | Compression algorithm implementation | codec.rs | 2-3 days |

### P2 - Architectural Improvements (2-4 weeks)

| # | Improvement | Scope | Estimated Effort |
|---|-------------|-------|------------------|
| 11 | Concurrent structure optimization (DashMap) | Global | 3-4 days |
| 12 | Pipeline asynchronization | be-execution | 4-5 days |
| 13 | Vector type system refactoring | types | 5-7 days |
| 14 | Error handling refinement | common, modules | 3-4 days |
| 15 | ExecNode static dispatch | be-execution | 3-4 days |
| 16 | Resource management (MemoryTracker) | fe-scheduler | 4-5 days |

### P3 - Long-term Planning (Continuous Iteration)

| # | Improvement | Scope | Estimated Effort |
|---|-------------|-------|------------------|
| 17 | Apache Arrow integration | types, be-execution | 2-3 weeks |
| 18 | Catalog persistence | fe-catalog, fe-common | 2-3 weeks |
| 19 | Raft consensus algorithm | fe-common | 3-4 weeks |
| 20 | Cloud storage support (S3) | be-storage | 2-3 weeks |
| 21 | Materialized view completion | fe-sql-planner | 2-3 weeks |
| 22 | Federated query | fe-catalog | 3-4 weeks |

---

## 3. Detailed Improvement List

### 3.1 Performance Optimization

#### Improvement #5: Vectorized Aggregation

**Current Issue**:
```rust
// crates/be-execution/src/exec_node.rs:258-263
// Row-by-row scalar extraction, extremely poor performance
let values: Vec<ScalarValue> = (0..block.num_rows())
    .map(|i| col.scalar_at(i))
    .collect();
let agg_value = Self::compute_aggregate(&values, func);
```

**Proposed Solution**:
- Use batch operations to process Vector data directly
- Avoid intermediate ScalarValue conversions
- Reference Arrow's aggregate implementation

**Implementation Steps**:
1. Implement `sum_batch()`, `count_batch()`, `avg_batch()` methods for each Vector type
2. Modify AggregateExecNode to use batch API
3. Add benchmarks to verify performance gains

**Expected Benefit**: Aggregation query performance improved by **5-10x**

---

#### Improvement #6: Vectorized Sorting

**Current Issue**:
```rust
// crates/be-execution/src/exec_node.rs:449-480
// Row-by-row comparison, closure captures block, cannot use SIMD
indices.sort_by(|&a, &b| {
    for &(col_idx, ascending) in &order_by {
        let scalar_a = col.scalar_at(a);
        let scalar_b = col.scalar_at(b);
        // Row-by-row comparison...
    }
});
```

**Proposed Solution**:
- Use columnar comparator, compare entire columns at once
- Use `sort_unstable_by` + vectorized comparison
- Consider using `radix_sort` for numeric types

**Implementation Steps**:
1. Implement `compare_indices` batch comparison method for Vector
2. Modify SortExecNode to use batch comparison
3. Use radix sort for numeric types

**Expected Benefit**: Sorting performance improved by **3-5x**

---

#### Improvement #7: SIMD Bitmap Operations

**Current Issue**:
```rust
// crates/types/src/bitmap.rs:214-227
// Manual bit operations, not using SIMD
impl BitAnd for &Bitmap {
    fn bitand(self, rhs: &Bitmap) -> Bitmap {
        for i in 0..words {
            data.push(left & right); // Word-by-word operation
        }
    }
}
```

**Proposed Solution**:
- Use `std::simd` (nightly) or `portable-simd`
- Process 256/512 bits at once
- Use `count_ones` SIMD instructions

**Implementation Steps**:
1. Add `packed_simd` or `std::simd` dependency
2. Implement SIMD AND/OR/NOT operations
3. Implement SIMD `set_count()` and `iter_set_bits()`
4. Add benchmarks to compare performance

**Expected Benefit**: Bitmap operation performance improved by **4-8x**

---

#### Improvement #8: StringVector Zero-Copy Optimization

**Current Issue**:
```rust
// crates/types/src/vector.rs:228-246
// Re-allocate and copy on every filter/slice
pub fn filter(&self, selection: &Bitmap) -> Self {
    for idx in selection.iter_set_bits() {
        if let Some(s) = self.get(idx) {
            data.extend_from_slice(s.as_bytes()); // Copy every time
        }
    }
}
```

**Proposed Solution**:
- Use offset references instead of copies (similar to Arrow StringArray)
- Add `StringView` type that holds references to original data
- Defer copying until necessary

**Implementation Steps**:
1. Design `StringView` structure (offsets + data reference)
2. Implement zero-copy filter/slice
3. Trigger copy when necessary (e.g., when data lifetime ends)

**Expected Benefit**: String filtering performance improved by **2-4x**, memory usage reduced by **50-70%**

---

#### Improvement #9: MemTable Structure Optimization

**Current Issue**:
```rust
// crates/be-storage/src/tablet.rs:59-67
// Each row stored as a Block, high memory overhead
pub struct MemTable {
    rows: BTreeMap<MemTableKey, Block>, // One Block per row
    memory_size: u64,
    capacity: u64,
}
```

**Proposed Solution**:
- Change to columnar storage: `BTreeMap<MemTableKey, RowData>` or direct columnar
- Merge data on batch writes
- Consider using ART (Adaptive Radix Tree) instead of BTreeMap

**Implementation Steps**:
1. Redesign MemTable as columnar structure
2. Implement batch insert optimization
3. Add precise memory statistics
4. Consider integrating ART library (e.g., `art` crate)

**Expected Benefit**: Memory usage reduced by **30-50%**, write performance improved by **2-3x**

---

#### Improvement #10: Compression Algorithm Implementation

**Current Issue**:
```rust
// crates/be-segment/src/codec.rs:8-24
// All compression algorithms are TODO
pub fn encode(data: &[u8], codec: CodecType) -> Vec<u8> {
    match codec {
        CodecType::Lz4 => data.to_vec(), // TODO
        CodecType::Zstd => data.to_vec(), // TODO
    }
}
```

**Proposed Solution**:
- Integrate `lz4` and `zstd` crates
- Implement complete codec
- Add compression level configuration

**Implementation Steps**:
1. Add `lz4 = "0.4"` and `zstd = "0.13"` dependencies
2. Implement LZ4 codec
3. Implement Zstd codec (with compression level support)
4. Add Snappy support (optional)
5. Add compression ratio/performance benchmarks

**Expected Benefit**: Storage space reduced by **50-80%**, I/O performance improved by **2-4x**

---

### 3.2 Type System Refactoring

#### Improvement #13: Vector Type System Refactoring

**Current Issue**:
```rust
// crates/types/src/vector.rs:5-20
// 13-variant enum, every match must handle all cases
pub enum Vector {
    Boolean(BooleanVector),
    Int8, Int16, Int32, Int64, Int128,
    Float32, Float64,
    String, Date, DateTime, Json, Null,
}
```

**Proposed Solution**:
- Group into `NumericVector`, `TemporalVector`, `StringVector`, etc.
- Use trait `TypedVector` to define common interface
- Reduce number of match branches

**Implementation Steps**:
1. Define `TypedVector` trait
2. Refactor Vector into grouped enum
3. Use macros to generate trait implementations
4. Update all code using Vector
5. Add tests to ensure functional consistency

**Expected Benefit**: Code reduced by **30-40%**, maintainability improved

---

#### Improvement #14: Error Handling Refinement

**Current Issue**:
```rust
// crates/common/src/error.rs:4-28
// Using String for error messages, lacks specific types
pub enum DrorisError {
    Storage(String),
    Query(String),
    Catalog(String),
}
```

**Proposed Solution**:
- Use specific error types (e.g., `TabletNotFound { id: u64 }`)
- Add error chain support
- Define dedicated error types for each subsystem

**Implementation Steps**:
1. Define dedicated error types for each module
2. Add error context
3. Implement `From` trait conversions
4. Update all error handling code

**Expected Benefit**: More precise error handling, improved debugging efficiency

---

### 3.3 Concurrency and Async Improvements

#### Improvement #11: Concurrent Structure Optimization

**Current Issue**:
```rust
// Multiple uses of RwLock<HashMap>, performance bottleneck under high concurrency
tablets: RwLock<HashMap<u64, Arc<Tablet>>>,  // engine.rs:17
nodes: RwLock<HashMap<NodeId, BeNode>>,      // cluster.rs:142
```

**Proposed Solution**:
- Use `DashMap` (concurrent HashMap) as replacement
- Or use `left_right` crate for lock-free reads
- Reduce lock granularity

**Implementation Steps**:
1. Add `dashmap = "6"` dependency
2. Replace `RwLock<HashMap>` with `DashMap`
3. Update all access code
4. Add concurrency benchmarks

**Expected Benefit**: Concurrent throughput improved by **2-3x**

---

#### Improvement #12: Pipeline Asynchronization

**Current Issue**:
```rust
// crates/be-execution/src/pipeline.rs:18-20
// Synchronous call, cannot leverage Tokio advantages
pub fn get_next(&mut self) -> Result<Option<Block>> {
    self.root.get_next()
}
```

**Proposed Solution**:
- Change to `async fn get_next()`
- ScanExecNode asynchronous storage reads
- ExchangeSink/Source asynchronous communication

**Implementation Steps**:
1. Modify ExecNode trait to be async
2. Update all execution nodes
3. Implement async storage reads
4. Implement async network communication
5. Add async benchmarks

**Expected Benefit**: I/O-intensive query performance improved by **2-4x**

---

### 3.4 Code Quality

#### Improvements #1-4: Clippy Warning Fixes

**Current Issue**:
- 80+ Clippy warnings
- Unused imports and variables (~20)
- Missing Default implementations
- Manual implementation of `div_ceil`
- Redundant closures and code patterns

**Proposed Solution**:
- Run `cargo clippy --fix --allow-dirty` to auto-fix most
- Manually fix remaining warnings
- Add CI checks to prevent regression

**Implementation Steps**:
1. Run `cargo clippy --fix --allow-dirty`
2. Manually fix remaining warnings
3. Add `#[deny(clippy::all)]` to lib.rs
4. Configure CI checks

**Expected Benefit**: Improved code quality, elimination of potential bugs

---

#### Improvement #15: ExecNode Static Dispatch

**Current Issue**:
```rust
// crates/be-execution/src/exec_node.rs:6
// Box<dyn ExecNode> has virtual function overhead
pub trait ExecNode: Send + Sync {
    fn open(&mut self) -> Result<()>;
    fn get_next(&mut self) -> Result<Option<Block>>;
    fn close(&mut self) -> Result<()>;
}
```

**Proposed Solution**:
- Use enum instead of trait object
- Or use generics + static dispatch
- Reference DataFusion's ExecutionPlan enum

**Implementation Steps**:
1. Define `ExecutionPlan` enum
2. Implement static dispatch
3. Update Pipeline to use enum
4. Add benchmarks to compare performance

**Expected Benefit**: Execution performance improved by **10-20%**

---

### 3.5 Testing and Documentation

#### Improvement: Test Coverage Expansion

**Current State**:
- 203 tests, mainly unit tests
- Missing integration tests
- Missing concurrency tests
- Missing performance benchmarks

**Proposed Solution**:
- Add integration tests (cross-module)
- Add concurrency safety tests
- Complete TPC-H benchmark
- Add documentation tests

**Implementation Steps**:
1. Create integration test suite
2. Add concurrency tests
3. Complete TPC-H benchmark
4. Add documentation tests
5. Configure CI to run all tests

**Expected Benefit**: Test coverage improved by **3x**, bug rate reduced

---

---

## 4. Implementation Roadmap

### Phase 1: Basic Fixes (Week 1)

```
Day 1-2: Fix Clippy warnings (Improvements #1-4)
Day 3-4: Add Default trait implementations
Day 5:   Clean up unused code, add CI checks
```

**Deliverables**:
- ✅ Zero Clippy warnings
- ✅ CI configuration complete
- ✅ Code quality baseline established

---

### Phase 2: Performance Optimization (Weeks 2-3)

```
Week 2:
  Day 1-3: Vectorized aggregation (#5)
  Day 4-5: Vectorized sorting (#6)
  
Week 3:
  Day 1-2: SIMD Bitmap operations (#7)
  Day 3-4: StringVector zero-copy (#8)
  Day 5:   MemTable structure optimization (#9)
```

**Deliverables**:
- ✅ Aggregation query performance improved 5-10x
- ✅ Sorting performance improved 3-5x
- ✅ Bitmap operation performance improved 4-8x
- ✅ Memory usage reduced 30-50%

---

### Phase 3: Architectural Improvements (Weeks 4-6)

```
Week 4:
  Day 1-3: Concurrent structure optimization (#11)
  Day 4-5: Pipeline asynchronization (#12)
  
Week 5:
  Day 1-3: Vector type system refactoring (#13)
  Day 4-5: Error handling refinement (#14)
  
Week 6:
  Day 1-3: ExecNode static dispatch (#15)
  Day 4-5: Resource management (#16)
```

**Deliverables**:
- ✅ Concurrent throughput improved 2-3x
- ✅ I/O query performance improved 2-4x
- ✅ Code reduced 30-40%
- ✅ Resource management framework complete

---

### Phase 4: Long-term Planning (Weeks 7-8 and beyond)

```
Week 7-8:
  - Compression algorithm implementation (#10)
  - Test coverage expansion
  - Documentation completion
  
Future iterations:
  - Apache Arrow integration (#17)
  - Catalog persistence (#18)
  - Raft consensus algorithm (#19)
  - Cloud storage support (#20)
```

**Deliverables**:
- ✅ Storage space reduced 50-80%
- ✅ Test coverage improved 3x
- ✅ Complete documentation
- ✅ Long-term roadmap

---

## 5. Risk Assessment and Mitigation Strategies

### High-Risk Items

| Improvement | Risk | Mitigation Strategy |
|-------------|------|-------------------|
| Vector refactoring | Large impact, may introduce bugs | Thorough testing, gradual migration |
| Pipeline asynchronization | Major API changes, extensive modifications | Phased implementation, maintain backward compatibility |
| Arrow integration | External library dependency, potential incompatibility | Evaluate compatibility first, gradual replacement |

### Medium-Risk Items

| Improvement | Risk | Mitigation Strategy |
|-------------|------|-------------------|
| SIMD optimization | Requires nightly or additional dependencies | Use portable-simd, maintain compatibility |
| Concurrent structure optimization | DashMap may have different semantics | Thoroughly test concurrency scenarios |
| Error handling refinement | Need to modify all error handling code | Use tools to assist refactoring |

### Low-Risk Items

| Improvement | Risk | Mitigation Strategy |
|-------------|------|-------------------|
| Clippy fixes | Almost no risk | Auto-fix + manual review |
| Default implementations | No risk | Direct addition |
| Clean up unused code | No risk | Direct deletion |

---

## 6. Success Metrics

### Quantitative Metrics

| Metric | Current Value | Target Value | Measurement Method |
|--------|--------------|--------------|-------------------|
| Clippy warnings | 80+ | 0 | `cargo clippy` |
| Aggregation query performance | Baseline | +5-10x | TPC-H Q1, Q6 |
| Sorting performance | Baseline | +3-5x | TPC-H Q10 |
| Concurrent throughput | Baseline | +2-3x | Concurrent query tests |
| Memory usage | Baseline | -30-50% | Memory profiling tools |
| Test coverage | 203 tests | 600+ tests | `cargo test` |
| Code lines | ~20k | -10-15% | `wc -l` |

### Qualitative Metrics

- ✅ Improved code readability
- ✅ Reduced maintenance cost
- ✅ Increased new feature development efficiency
- ✅ Lowered barrier for community contributions

---

## 7. Appendix

### A. Reference Resources

- **Apache Arrow**: https://arrow.apache.org/
- **DataFusion**: https://github.com/apache/datafusion
- **DuckDB**: https://github.com/duckdb/duckdb
- **Apache Doris**: https://github.com/apache/doris
- **Rust SIMD**: https://doc.rust-lang.org/std/simd/
- **DashMap**: https://crates.io/crates/dashmap

### B. Recommended Tools

| Tool | Purpose |
|------|---------|
| `cargo clippy` | Code quality checks |
| `cargo bench` | Performance benchmarks |
| `cargo tarpaulin` | Test coverage |
| `cargo flamegraph` | Performance profiling |
| `cargo audit` | Security audit |

### C. Related Documentation

- `docs/build-plan.md` - Build plan
- `docs/compatibility-matrix.md` - Compatibility matrix
- `PERFORMANCE_REPORT.md` - Performance report
- `README.md` - Project description

---

## 8. Change Log

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2025-05-04 | v1.0 | Initial version, based on code review | AI Assistant |

---

*This document is the improvement plan for the RorisDB project and will be continuously updated based on implementation progress.*
