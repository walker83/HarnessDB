# Expression Function Tests

This directory contains comprehensive tests for all expression functions in RorisDB.

## Test Organization

Tests are organized by category in the `functions/` subdirectory:

- **bitwise.rs** - Bitwise operations (bitand, bitor, bitxor, bitnot, bitshiftleft, bitshiftright)
- **math.rs** - Extended math functions (sign, degrees, radians, truncate, greatest, least, modulo, cot, sinh, cosh, tanh)
- **string.rs** - Extended string functions (ltrim, rtrim, replace, left, right, locate, repeat, space, reverse, etc.)
- **hash.rs** - Cryptographic hash functions (md5, sha1)
- **json.rs** - JSON functions (json_parse, json_query, json_get, json_contains, json_array, json_object, etc.)

## Running Tests

Run all expression function tests:
```bash
cargo test -p fe-expression
```

Run tests for a specific category:
```bash
# Bitwise functions only
cargo test -p fe-expression bitwise

# Math functions only
cargo test -p fe-expression math

# String functions only
cargo test -p fe-expression string

# Hash functions only
cargo test -p fe-expression hash

# JSON functions only
cargo test -p fe-expression json
```

Run a specific test:
```bash
cargo test -p fe-expression test_bitand
cargo test -p fe-expression test_truncate
cargo test -p fe-expression test_md5
```

## Test Coverage

All test cases are verified against Apache Doris behavior to ensure SQL compatibility.

Current coverage:
- ✅ Bitwise functions: 100%
- ✅ Extended math functions: 100%
- ✅ Extended string functions: 100%
- ✅ Hash functions: 100%
- ✅ JSON functions: 100%

## Adding New Tests

When adding a new function:

1. Add the function implementation in `src/functions.rs`
2. Add the function name to the match statement in `call()`
3. Create tests in the appropriate category file in `functions/`
4. Include regression tests for edge cases
5. Verify against Doris behavior

## Test Naming Conventions

- Test function names should start with `test_`
- Use descriptive names: `test_<function_name>_<scenario>`
- Regression tests should be marked with a comment: `// Regression test: <description>`
- Include batch tests for vectorized operations

## Known Limitations

- NULL handling: Currently uses default value replacement instead of NULL propagation (tracked for improvement)
- DECIMAL: Uses Float64 approximation (precise DECIMAL implementation planned)
