# Contributing to statguardian

Thank you for contributing! This document covers how to set up the project,
the conventions we follow, and the review process.

---

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | ≥ 1.75 | `rustup update stable` |
| Python | ≥ 3.8 | for Python bindings |
| maturin | ≥ 1.7 | `pip install maturin` |
| Polars (Python) | ≥ 0.20 | `pip install polars` |

## Quick setup

```bash
git clone https://github.com/Mullassery/statguardian.git
cd statguardian

# Build and test the Rust crates (statguardian is the pyo3 extension-module
# crate, exercised via the maturin/pytest path below instead — see the
# `rust-build` job comment in .github/workflows/ci.yml for why)
cargo test --workspace --exclude statguardian

# Build the Python extension (development mode)
maturin develop --release

# Run Python tests
pip install ".[dev]"
pytest tests/
```

## Project structure

- `crates/statguardian-core` — AST and `.sg` DSL parsing (pest grammar) and compilation
- `crates/statguardian-engine` — query execution planner and operator pipeline
- `crates/statguardian-validators` — schema validation, quality rules, anomaly detection
- `crates/statguardian-stats` — statistical drift detection (PSI, KS test)
- `crates/statguardian-io` — file/table format readers (Parquet, CSV, JSON, Avro, Delta, Iceberg, SQL, cloud)
- `crates/statguardian-lineage` — data lineage tracking
- `crates/statguardian-metrics` — report generation and scoring
- `crates/statguardian-py` — PyO3 FFI layer exposing the Rust engine to Python
- `python/statguardian/` — the Python package (CLI, dbt integration, thin wrappers around the compiled extension)

## Making changes

### Rust crates

1. Make your changes in the relevant crate under `crates/`.
2. Write or update tests (`#[cfg(test)]` in the same file, or `tests/integration_test.rs`).
3. Run `cargo clippy --workspace` and fix any warnings.
4. Run `cargo fmt --all`.
5. Run `cargo test --workspace --exclude statguardian`.

### Python bindings

1. Edit `crates/statguardian-py/src/lib.rs`.
2. Rebuild with `maturin develop --release`.
3. Test from Python.

### DSL grammar

The grammar lives in `crates/statguardian-core/src/parser/grammar.pest`.
After editing:

1. Rebuild with `cargo build`.
2. Update `parse_*` functions in `crates/statguardian-core/src/parser/mod.rs`.
3. Add a DSL test in `parser/mod.rs` `tests` module.

## Code conventions

- **No row loops** — use Polars/Arrow columnar APIs in hot paths.
- **No `unwrap()` in library code** — use `?` and typed errors.
- All public report/AST types must derive `Serialize, Deserialize`.
- New public functions need doc comments (`///`).
- Keep changes focused; one logical change per PR.

## Pull request checklist

- [ ] `cargo test --workspace --exclude statguardian` passes
- [ ] `cargo clippy --workspace` has no warnings
- [ ] `cargo fmt --all --check` passes
- [ ] New feature has at least one new test
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] If a new file format was added: this file's "Project structure" section and `README.md`'s format table updated

## Reporting bugs

Open an issue at <https://github.com/Mullassery/statguardian/issues> with:

- statguardian version
- Minimal reproducing DSL and DataFrame
- Expected vs actual behaviour

## License

By contributing you agree that your changes will be licensed under the
[Apache License 2.0](LICENSE).
