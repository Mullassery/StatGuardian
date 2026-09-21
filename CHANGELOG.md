# Changelog

All notable changes to this project are documented in this file. Format is
loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This file was started 2026-09-21, reconstructed from `git log` for recent
history only. It is not a complete, retroactive history back to the
project's first commit — see `git log` for that. Entries below are based on
actual commit messages and diffs, not invented.

## [Unreleased]

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
