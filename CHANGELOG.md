# Changelog

All notable changes to this project are documented in this file. Format is
loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This file was started 2026-09-21, reconstructed from `git log` for recent
history only. It is not a complete, retroactive history back to the
project's first commit — see `git log` for that. Entries below are based on
actual commit messages and diffs, not invented.

## [Unreleased]

### Fixed
- Add `pandas` (and `narwhals`) to the `api` extra so the HTTP `/validate`
  service works on a clean `statguardian[api]` install. Verified in a clean
  venv and by the PyReverseETL cross-repo integration test.
- Declare `license = "Apache-2.0"` in project metadata so PyPI/wheels expose a PEP 639 `License-Expression` (verified in the built wheel).

### Added
- `statguardian._mcp_connector.BaseMCPConnector`: the canonical shared base
  that PyStreamMCP, PyStreamPDF, and other sibling projects attempt to import.
  It generates the DAB config, manages the subprocess lifecycle, and tracks
  readiness. Secure defaults are preserved: loopback-only unless
  `allow_remote=True`, no CORS origins by default, and read-only permissions.
  Covered by `tests/test_mcp_connector.py`.
- `integrations/airflow-statguardian`: `StatGuardianValidateOperator` runs a
  `.sg` contract via `statguardian.execute_file()` and fails the Airflow task
  when the report does not pass. Supports `fail_on_warning` and templated
  contract/data/reference paths. Unit tests cover the pass/fail decision
  without requiring Airflow in the test environment.

## [2.8.1] - 2026-10-23

### Added
- Exported `BaseMCPConnector` from the public API.

### Fixed
- Cleaned up the HTTP service's type annotations and an unused local so the
  API module passes the current ruff configuration.

## [2.8.0] - 2026-10-19

### Added
- Local HTTP service (`statguardian serve --contract orders.sg`) exposing
  `/healthz` and `/validate` in the shape PyReverseETL's
  `StatGuardianClient` expects. `/detect-changes` returns an explicit 501.
- `[api]` extra (FastAPI + uvicorn). Service tests inject a fake contract/runner
  and cover healthy, passing, failing, unbound, and 501 paths.

## [2.7.2] - 2026-10-18

### Security
- `DataContract.from_dsl` / `from_file` now enforce the DSL safety checks
  that were previously applied only by the CLI (`_dsl_validator.py`):
  empty/oversized contracts, missing `dataset` keyword, brace nesting deeper
  than 50, and the reserved/suspicious patterns (`__…__`, `;;`, null byte)
  are rejected at the binding boundary, so the programmatic entry point
  cannot be handed pathological input (STATGUARDIAN-006).

### Fixed
- `statguardian.__version__` now agrees with the published wheel version.
  The extension reports `CARGO_PKG_VERSION` (workspace `Cargo.toml`), which
  had been left at 2.7.0 while `pyproject.toml` moved to 2.7.1; both are now
  bumped together (2.7.2).

## [2.7.1] - 2026-10-17

### Fixed
- Audit remediation: security hardening, correctness fixes, and lint/CI gates.

## [2.7.0] - 2026-10-12

### Added
- **StreamGuard** (`crates/statguardian-stream`): incremental, bounded-memory
  NDJSON validation. `StreamValidator` reads one record at a time from any
  `BufRead`, buffers micro-batches, and reuses `Engine::execute` so contracts,
  rules, and findings are unchanged. Includes `StreamLimits` (records/bytes/
  line size/batch size), `MalformedPolicy` (`FailFast`/`Skip`/`Collect`),
  stable global record indices, and exit codes (`0` pass / `1` validation fail,
  with execution errors returned as `Err` → `2` at the CLI).
- **Sanitization**: opt-in `RedactionPolicy` (mask/mask-with/hash/null/drop) and
  `NormalizationPolicy` (trim/lower/upper) that never mutate the source and
  never emit raw values; `SanitizationSummary` aggregates counts by kind/field.
- **AgentGuard** (`statguardian-stream::agent`): SDK-free agent-runtime
  observability — normalized `AgentEvent` model, `Provenance`
  (`Measured`/`Estimated`/`Unknown`), advisory `Enforcement`, and heuristic
  loop detection labeled as such.
- Python binding `execute_stream_ndjson` + `StreamReport` (PyO3) and the
  `statguardian stream` CLI subcommand (report→stdout, diagnostics→stderr,
  broken-pipe safe).

### Fixed
- **Silent validation skip for single-row DataFrames** (Polars 0.55 scalar
  columns). `Column::as_series()` returns `None` for a scalar-backed column,
  so `SchemaValidator`, `RuleEngine`, and the engine DAG `get_series` treated a
  1-row frame as having no checks. Streaming surfaces this because the final
  leftover micro-batch is often exactly one record. All three call sites now use
  `as_materialized_series()`; regression test added.

### Notes
- Verified: `cargo test --release --all-features --workspace --exclude
  statguardian` → 146 passed / 0 failed (plain `--release`: 145, was 111);
  `cargo clippy --release --all-features --workspace --exclude statguardian
  --all-targets -- -D warnings` → clean.
- Verified (Python + CLI): extension built with `maturin develop --release` and
  exercised on Python 3.9.6 / polars 1.32.2 / pyarrow 21.0.0 — `execute_stream_ndjson`
  and `statguardian stream` exit codes (0/1/2), stdout/stderr separation,
  malformed policy, `max_records` truncation, and blocking (missing-column)
  behavior all pass. New tests in `tests/test_python_bindings.py` (9 total;
  full `pytest tests/` → 35 passed) run in the CI `python-tests` job. See
  `docs/STREAMGUARD.md` §5.
- Distribution: `maturin build --release` produces a single abi3 wheel
  (`statguardian-2.7.0-cp38-abi3-macosx_11_0_arm64.whl`, ~17 MB) that installs
  cleanly into a fresh venv; `execute_stream_ndjson`/`StreamReport` import and
  the `statguardian stream` CLI returns 0/1/2 as expected from the installed
  wheel.

## [2.6.3] - 2026-10-12

### Fixed
- **Re-release superseding the accidental 2.6.2.** A 2.6.2 build was published
  to PyPI by mistake from a stale checkout that predated the 2.6.1 parser fix:
  it contained the `unquote()` fix for `regex=`/`enum=`/string literals/anomaly
  named args but **not** the `stream { window= watermark= emit= }` nesting fix,
  so 2.6.2 would have regressed stream-block parsing to a silent no-op. 2.6.3
  is built from the canonical 2.6.1 source and is functionally identical to
  2.6.1 (no code changes); it exists only to supersede 2.6.2 for installers.
  Since 2.6.3 > 2.6.2, resolvers (pip, uv) will select 2.6.3 by default; the
  2.6.2 release should be treated as withdrawn.

## [2.6.1] - 2026-10-05

### Fixed
- **DSL parser quote-stripping bug** (`crates/statguardian-core/src/parser/mod.rs`):
  `regex=`, `enum=[...]`, and anomaly named args (`method=`, `pattern=`) read
  a `string_literal` pair's raw `as_str()` text directly, which includes the
  surrounding `"` characters (the grammar rule is atomic). Every
  regex/enum-constrained value was always reported as a violation, and
  `detect_outliers(..., method="iqr"|"zscore")` always reported zero
  outliers — for every user, always, with no error surfaced. Found and
  left unfixed in the 2026-09-21 audit (see `docs/ROADMAP_HONEST.md`);
  fixed here via a shared `unquote()` helper applied at all broken call
  sites, with 3 new regression tests asserting actual unquoted values.
- **`stream { window= watermark= emit= }` block parsing no-op** (same file)
  — newly found while fixing the above: `parse_stream()` matched
  `opt.as_rule()` against `Rule::stream_window`/etc. directly, but the
  grammar wraps each alternative in an intermediate `stream_option` pair,
  so the match always fell through and `StreamConfig` fields were always
  `None` regardless of input. Fixed by unwrapping the extra pest layer;
  added a regression test. Note: these fields are still not consumed by
  the engine downstream — see `TECHNICAL_DEBT.md` TD-0003.
- `clippy::useless_vec` warning in `statguardian-lineage`'s test module
  (`storage.rs:623`).
- `docs/LICENSES.md:7` — `[LICENSE](LICENSE)` resolved to the nonexistent
  `docs/LICENSE`; corrected to `../LICENSE`.
- `docs/CLI.md:352` and `docs/DBT_INTEGRATION.md:63` — both linked to
  `../README.md#dsl-reference`, an anchor that has never existed in
  `README.md`; repointed to `examples/*.sg` and the README itself.
- `.github/CI_ERRORS.md` — removed. Orphaned template file with unresolved
  `$REPO_NAME`/`#$repo` placeholders and links to `CI_ERRORS_REPORT.md` and
  `TROUBLESHOOTING.md`, neither of which exists anywhere in this repo;
  nothing else referenced this file.
- `crates/statguardian-io/src/cloud.rs:163` — `clippy::single_match` warning
  in `test_format_inference_from_uri`; rewrote as `if let`. Test-only,
  behavior unchanged.

### Added
- `TECHNICAL_DEBT.md` and `AUDIT.md` — consolidated technical-debt register
  and audit report (cross-referencing `docs/SECURITY_AUDIT.md` and
  `docs/ROADMAP_HONEST.md` rather than duplicating them).

### Changed
- `cargo audit`: 3 of the remaining 5 vulnerabilities from the 2026-09-13
  remediation pass fixed by completing the `polars` 0.44→0.55 / `pyo3-polars`
  0.18→0.28 / `pyo3` 0.21→0.29 migration (2026-09-14). 2 findings remain,
  confirmed blocked on an upstream `polars`/`object_store` release — see
  `docs/SECURITY_AUDIT.md` §2.
- Bumped `rustls` 0.23.43→0.23.45 (`RUSTSEC-2026-0285`).

### Known regressions
- `StreamingBatcher` (`crates/statguardian-io/src/lib.rs`) lost bounded-memory
  incremental reads as a side effect of the polars 0.55 migration above —
  Polars removed the public batched-reader API it depended on. Still correct
  (single read, same results), no longer memory-bounded for very large
  files streamed in small batches. See `docs/SECURITY_AUDIT.md` §2b.

## [2.6.0] - 2026-08-30

### Fixed
- `SlidingWindowExecutor` (`crates/statguardian-core/src/streaming.rs`) never
  evicted expired windows, causing unbounded memory growth for long-running
  streaming validation. Now evicts on every event, mirroring
  `TumblingWindowExecutor::close_windows_before`.

## [2.5.0] - 2026-08-23

### Added
- `merge_violations(report, extra_violations)` to combine a `ValidationReport`
  with custom-validator output into one pass/fail result.

### Fixed
- Real fixed-cost streaming implementation (previous version re-read the
  full file per batch rather than streaming incrementally).
- Seasonal-anomaly timestamp bug in `_anomaly_detection.py`.
- Polars/PyArrow compatibility issue.
- `tests/test_streaming.rs` was written but not included in an earlier
  commit; added.

## [2.4.0] - 2026-08-23

### Added
- dbt package and CLI integration (`integrations/dbt-statguardian`,
  `statguardian dbt validate`).

### Fixed
- Broken `execute_sql`/`execute_spark`/`execute_cloud` imports.

## [2.3.2] - 2026-08-17

### Fixed
- README corrected to match the real API surface; unverified/aspirational
  claims removed.
- Remaining `SECURITY_AUDIT.md` remediation items closed (see that file for
  detail).

## Earlier releases

Versions before 2.3.2 (2.3.0, 2.2.0, and the 1.x series) shipped real, working
functionality but the commit history for that period mixes feature work with
several rounds of license-metadata churn and doc fixes. Rather than guess at
a clean changelog for that period, see `git log --oneline` for the
authoritative record.
