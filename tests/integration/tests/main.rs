// Integration test entry point. Cargo auto-discovers a file named `main.rs`
// in `tests/` as a single test target; by pulling every suite in via `mod`
// we get one binary (one link of the full dep graph) instead of one per file.
mod suites;
