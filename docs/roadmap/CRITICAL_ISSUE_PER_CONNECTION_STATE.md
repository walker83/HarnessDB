# Critical Architecture Issue: Per-Connection Session State

## Status
**PRIORITY: HIGH** - Affects multi-connection scenarios and connection pool support
**EFFORT: LARGE** - Requires refactoring 40+ methods across 6 files
**CURRENT STATE: DOCUMENTED** - Working state maintained, issue tracked for future fix

## Problem

E2E integration tests show failures when multiple connections are active:
```
table 'roris.test_29970_15.sales' not found
```

Tests interfere with each other even when running sequentially.

## Root Cause

All MySQL connections share a single `RorisQueryHandler` instance with shared state:
- `current_database: Arc<PlRwLock<String>>` - shared across ALL connections
- `session_vars: Arc<PlRwLock<SessionVariables>>` - shared across ALL connections  
- `transaction: Arc<PlRwLock<SimpleTransactionState>>` - shared across ALL connections

When test A executes `USE database_a`, it sets the shared `current_database`. When test B executes `USE database_b`, it overwrites the same field. This causes:
1. Test A's queries may execute against database_b instead of database_a
2. Data appears to "leak" between connections
3. Tables seem to disappear or have wrong data

## Impact

- **Critical**: Multi-connection scenarios are broken
- **High**: Connection pools will malfunction
- **High**: Web SQL editor sessions will interfere
- **Medium**: Tests fail intermittently in parallel mode

## Current Workaround

1. Tests use unique database names per test
2. Single-connection scenarios work correctly
3. 137 real-world scenario tests pass (from GitHub applications)
4. Web SQL editor works for single-user context

## Recommended Fix (Future Work)

### Approach
Refactor to support per-connection session state using a `HashMap<u32, SessionState>`.

### Key Changes
1. Update `QueryHandler` trait to pass `conn_id` to all methods
2. Add `SessionState` struct per connection in `RorisQueryHandler`
3. Update 40+ methods in `query_executor.rs` to use session state
4. Update all call sites in `connection.rs` to pass `conn_id`

### Files Affected
- `crates/mysql-protocol/src/server.rs` - QueryHandler trait
- `crates/mysql-protocol/src/connection.rs` - MySQL protocol handler  
- `roris-server/src/handler_struct.rs` - RorisQueryHandler struct
- `roris-server/src/fe_main.rs` - QueryHandler implementation
- `roris-server/src/query_executor.rs` - Statement execution (40+ methods)
- `roris-server/src/ddl_handler.rs` - DDL execution (15+ methods)
- `roris-server/src/web/routes.rs` - Web SQL editor

### Estimated Effort
- **Time**: 4-6 hours for experienced Rust developer
- **Risk**: Medium - mechanical changes but many call sites
- **Testing**: Requires comprehensive multi-connection tests

## Why Not Fixed Yet

1. **Pragmatic decision**: The refactoring is large and risky
2. **Working alternatives**: Single-connection scenarios work fine
3. **Test coverage**: 137 real-world tests pass with current architecture
4. **Priority**: Focus on GitHub star acquisition and user adoption first
5. **Future improvement**: Can be done as v0.4.0 or v0.5.0 enhancement

## Next Steps

1. ✅ Document the issue (this file)
2. ✅ Maintain working state with single-connection support
3. ⏳ Focus on GitHub star acquisition (GITHUB_STRATEGY.md)
4. ⏳ Schedule refactoring for v0.4.0 or v0.5.0 release
5. ⏳ Add multi-connection stress tests before refactoring

## Related Files

- `docs/roadmap/done/config-ops-backup-sql-editor.md` - Completed features
- `GITHUB_STRATEGY.md` - Star acquisition plan
- `tests/real_world_scenarios/` - 137 passing tests

## Testing Strategy (After Fix)

After the refactoring is complete:
1. Run parallel tests: `cargo test --workspace` (currently fails)
2. Multiple simultaneous MySQL connections
3. Connection pool stress test (e.g., HikariCP with 10 connections)
4. Web editor with multiple browser tabs
5. Load testing with sysbench or similar tool
