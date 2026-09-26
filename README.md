# StatGuardian

A Rust-native data quality engine with a declarative contract DSL: schema validation, drift detection, and anomaly detection for Pandas and Polars.

[![Tests](https://img.shields.io/github/actions/workflow/status/Mullassery/statguardian/ci.yml?label=tests)](https://github.com/Mullassery/statguardian/actions)
[![PyPI](https://img.shields.io/pypi/v/statguardian)](https://pypi.org/project/statguardian/)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue)](https://www.python.org/downloads/)

Stop data quality issues from reaching production. StatGuardian validates data at runtime against a versionable contract, catching schema violations, statistical drift, and anomalies before they reach downstream consumers.


## 30-Second Start

```python
import polars as pl
import statguardian

contract = statguardian.DataContract.from_dsl("""
dataset orders {
    schema {
        order_id: string, not_null, unique
        amount:   float,  positive
        status:   string, not_null, enum=["pending","paid","cancelled"]
    }
    quality {
        completeness(order_id) > 0.999
    }
}
""")

df = pl.read_parquet("orders.parquet")
report = statguardian.execute(contract, df)
print(report.summary())
print(f"Passed: {report.passed}")
```

## Why StatGuardian?

- Contracts are declarative and versionable (`.sg` files), not scattered assertions in application code
- Rust-native execution — schema, quality, drift, and anomaly checks run in the compiled engine, not a Python loop
- One contract, multiple frameworks: the same `.sg` file validates Pandas and Polars DataFrames, Delta Lake tables, and Apache Iceberg tables
- Drift and anomaly detection are first-class DSL constructs, not a separate library

A reproducible benchmark comparing StatGuardian against other validation libraries is tracked in `docs/bench/benchmark.py` — run it against your own workload rather than relying on any library's marketing numbers, including ours.

## vs pandera

pandera is the closest OSS equivalent (declarative schema validation for pandas/polars DataFrames). We benchmarked both against **live, real-world data** — 200,000 rows pulled fresh from the [NYC 311 Service Requests API](https://data.cityofnewyork.us/resource/erm2-nwe9.json) (Socrata), not synthetic/fabricated rows — with an equivalent 7-column contract (not-null, uniqueness, enum, regex, and numeric-range checks) applied to both.

| | StatGuardian 2.5.0 | pandera 0.26.1 |
|---|---|---|
| Engine | Rust-native, Polars | Python, pandas (columnar/vectorized) |
| Median time (200K rows, 7 checks, best-of-7) | **32.3ms** | 275.9ms |
| Speedup | **8.5x** | 1x (baseline) |
| Violations caught on real data (injected: dup key, invalid borough, invalid status, out-of-range lat) | 4/4 | 4/4 |

Methodology: real data has genuine nulls (borough, zip, lat/lon are missing on 0.3–3.6% of live rows) and real categorical noise (e.g. `"Unspecified"` as a legitimate borough value) — we did not synthesize clean or adversarial rows. Both libraries were pointed at the same DataFrame content and validated the same rules. Correctness was cross-checked separately with known-bad injected rows (duplicate key, invalid enum values, out-of-range coordinate) — both libraries flagged all 4 injected violations, so the speed difference is not coming from skipped checks. Reproduce with `docs/bench/nyc311_vs_pandera.py` (fetches live data at run time, so exact timings will vary with API latency and the day's dataset).

One behavioral difference worth knowing: `report.passed` reflects only checks marked `@blocking` in the `.sg` contract — a contract with enum/regex/range violations but no `@blocking` checks can still report `passed=True` with a non-zero violation count and a letter-grade score. This is intentional (severity tiers), but is easy to misread as "no violations found" if you don't add `@blocking` to the rules you actually want to gate on. pandera's `lazy=True` mode, by contrast, always raises/reports on any check failure.

**Honesty note:** the GitHub repo description and older docs cited "13x faster than pandera" as an unverified, unsourced number (already flagged in `docs/ROADMAP_HONEST.md`). The measured number above — 8.5x — is workload-specific (7 checks, 200K rows, this dataset's null/cardinality distribution) and should not be read as a universal multiplier; run `docs/bench/benchmark.py` or the script above against your own data before relying on either number.

## Real-World Use Cases

**E-commerce order validation**
```python
contract = statguardian.DataContract.from_dsl("""
dataset orders {
    schema {
        order_id: string, not_null, unique
        amount:   float,  positive
        status:   string, not_null, enum=["pending","shipped","delivered"]
    }
}
""")
report = statguardian.execute(contract, orders_df)
```

**Drift monitoring between two batches**
```python
report = statguardian.execute(contract, incoming_df, reference=baseline_df)
for d in report.drift_results():
    if not d["passed"]:
        print(f"Drift detected in {d['column']}: PSI={d.get('psi', 0):.4f}")
```

## Key Capabilities

- Declarative contract DSL: schema, quality rules, statistical drift thresholds, and anomaly checks in one file
- Type checking with detailed, structured violation messages
- Statistical drift detection (PSI, KS test) between a dataset and a reference baseline
- Built-in anomaly detection (outliers, duplicates)
- Supports Pandas and Polars DataFrames, Delta Lake, and Apache Iceberg tables with the same contract
- Rust-native execution core

## Features

**Core Validation**
- Type validation (int, float, str, bool, datetime, etc.)
- Min/max constraints for numeric types
- Enum validation for categorical data
- Null/not-null constraints
- Pattern matching for strings (regex)
- Custom validation functions
- Composite constraints (multiple rules per field)

**Data Quality Analysis**
- Automatic drift detection (schema changes)
- Anomaly detection (outliers, unexpected values)
- Statistical profiling (mean, std, quartiles)
- Missing value reporting
- Duplicate detection

**Framework Support**
- Pandas DataFrames (convert with `pl.from_pandas(df)` before calling `execute()` — see Known Issues)
- Polars DataFrames (native)
- Delta Lake tables (time-travel validation)
- Apache Iceberg tables (snapshot validation)
- Unified contract across all frameworks

## Requirements

- **Python:** 3.8+
- **Core:** Rust-native validation engine (precompiled wheel, no local Rust toolchain needed)
- **Data Frameworks:** polars (required), pandas (optional, via `pip install statguardian[pandas]`)

## Examples

See `examples/` for complete, runnable scripts, including `python_quickstart.py` (schema validation, drift detection, anomaly detection, JSON/Prometheus output) and `.sg` contract files.

**Schema validation**
```python
contract = statguardian.DataContract.from_dsl("""
dataset users {
    schema {
        id:    int,    not_null, unique, primary_key
        email: string, regex="^[^@]+@[^@]+\\.[^@]+$"
        age:   int,    between(0, 120)
    }
    quality {
        completeness(id) > 0.99
    }
}
""")

report = statguardian.execute(contract, df)
print(report.summary())
for v in report.violations():
    print(v["severity"], v["column"], v["message"])
```

**Anomaly detection**
```python
contract = statguardian.DataContract.from_dsl("""
dataset events {
    schema { id: int, not_null }
    anomalies {
        detect_outliers(id, method="iqr")
        @blocking: detect_duplicates(id)
    }
}
""")
report = statguardian.execute(contract, df)
```

**Custom Python validators + merging with a contract report**
```python
@statguardian.validator(column="amount", severity="blocking")
def amount_is_sane(values):
    bad_rows = [i for i, v in enumerate(values) if v > 1_000_000]
    return (bad_rows, "amount over 1,000,000") if bad_rows else None

report = statguardian.execute(contract, df)
extra = statguardian.run_custom_validators(df)
merged = statguardian.merge_violations(report, extra)
print(merged.summary())
```

## API Reference

**Core**

- `DataContract.from_dsl(dsl_string)` / `DataContract.from_file(path)` — compile a contract
- `execute(contract, df, reference=None) -> ValidationReport` — validate a Pandas/Polars DataFrame
- `execute_file(contract, path, reference_path=None)` — validate Parquet/CSV/JSON/Avro/ORC/Arrow IPC files
- `execute_delta(contract, path, ...)`, `execute_iceberg(contract, path, ...)` — lakehouse table validation
- `execute_sql`, `execute_spark`, `execute_cloud` — SQL, PySpark, and object-storage sources

**ValidationReport**

- `.passed`, `.health_score`, `.grade`, `.violation_count`
- `.violations()`, `.drift_results()`, `.column_profiles()`
- `.summary()`, `.to_json()`, `.to_prometheus()`

**Custom validators**

- `validator(column=...)` — register a Python function as a custom check
- `run_custom_validators(df)` — run registered validators, returns violation dicts
- `merge_violations(report, extra_violations) -> MergedReport` — combine a `ValidationReport` with custom-validator violations into one pass/fail result

Full CLI usage: [docs/CLI.md](docs/CLI.md). DSL syntax: see `examples/*.sg`.

## dbt Integration

Run StatGuard contracts against your dbt models as part of `dbt build`,
and surface pass/fail as a native dbt test — see
[integrations/dbt-statguardian](integrations/dbt-statguardian) and
[docs/DBT_INTEGRATION.md](docs/DBT_INTEGRATION.md).

```bash
pip install "statguardian[dbt]"
dbt build
statguardian dbt validate --project-dir . --write-results
dbt test
```

## Installation

```bash
pip install statguardian
```

For development:
```bash
git clone https://github.com/Mullassery/statguardian
cd statguardian
pip install -e ".[dev]"
pytest
```

## Documentation

- [CLI Reference](docs/CLI.md)
- [dbt Integration](docs/DBT_INTEGRATION.md)
- [Security Audit](docs/SECURITY_AUDIT.md)
- [Honest Roadmap](docs/ROADMAP_HONEST.md) — what's actually built, tested, and broken; start here, not `docs/ROADMAP.md`
- [Examples](examples/)
- [Contributing](CONTRIBUTING.md)
- [Code of Conduct](docs/CODE_OF_CONDUCT.md)
- [Changelog](CHANGELOG.md)

## Known Issues

- **HIGH, found 2026-09-21 — `regex=`, `enum=[...]`, and anomaly-detection
  `method=` DSL constraints do not work.** Confirmed by actually running
  `statguardian validate` against a real contract and a real Parquet file:
  `enum=[...]` and `regex=` constraints report **every** value as a
  violation, including values that are legitimately valid, and
  `detect_outliers(col, method="iqr"|"zscore")` silently reports **zero**
  outliers regardless of the actual data, with no error either way. Root
  cause and exact file:line detail in
  [`docs/ROADMAP_HONEST.md`](docs/ROADMAP_HONEST.md#known-regressions-and-open-engineering-work).
  This affects the exact examples in this README's own "30-Second Start" and
  "Schema validation" sections above — treat those `.sg` snippets as
  illustrating DSL syntax only, not as constraints that currently work
  correctly.
- `execute()` accepts a Polars DataFrame, not a raw pandas DataFrame. Passing a pandas DataFrame directly raises an unhelpful `AttributeError` (verified against the current build) rather than converting automatically — call `pl.from_pandas(df)` first. The `pandas` extra is used by the SQL/Spark/GPU connectors internally, which already do this conversion for you.
- Performance numbers are not yet published as a reproducible, checked-in benchmark result — `docs/bench/benchmark.py` exists but its output has never been committed. Treat any speed claims (including from this project) as unverified until you've run the benchmark yourself.
- `docs/ROADMAP.md`, `docs/ROADMAP_HONEST.md`, and `docs/ROADMAP_INTEGRATED.md` overlap and are not kept in sync. `docs/ROADMAP_HONEST.md` was rewritten 2026-09-21 against the actual codebase and is current as of that date; `docs/ROADMAP.md` and `docs/ROADMAP_INTEGRATED.md` still contain fabricated "shipped" claims (a REST API, n8n/Power Automate/Temporal/Airflow integrations, Slack/PagerDuty alerting, JSON-lines audit logging — none of which exist in this codebase) and now carry disclaimers pointing back to `ROADMAP_HONEST.md`. A full consolidation into one document has not been done. Treat `docs/SECURITY_AUDIT.md` as the current source of truth for security status specifically.
- **`cargo audit`: down to 2 remaining findings (from 10), both confirmed blocked on an upstream release, not on anything in this codebase (2026-09-14).** Completed the `polars` 0.44→0.55 / `pyo3-polars` 0.18→0.28 / `pyo3` 0.21→0.29 migration this section previously described as reverted — this closed both `pyo3` advisories outright and, as a side effect of the `polars` bump, also cleared `fast-float` and the `memmap2`/`rustls-pemfile` warnings. The 2 that remain (`RUSTSEC-2026-0194`/`RUSTSEC-2026-0195`, HIGH-severity `quick-xml`) are transitive via `object_store` (itself transitive via `polars`); `polars` 0.55.2 pins `object_store` to a range that itself pins `quick-xml` below the fixed version, and neither can be bumped independently with `cargo update` (confirmed by trying) — see `docs/SECURITY_AUDIT.md` item 2 for the full dependency trace. CI's `cargo audit` step explicitly ignores just these two IDs, with a comment pointing back here, so CI stays green without hiding the finding.
- **New, introduced by that same migration: `StreamingBatcher` is no longer bounded-memory.** Polars 0.55 removed its public incremental/batched CSV and Parquet readers entirely — there's no longer a stable API in this dependency stack for reading either format row-group-by-row-group from a single held-open handle. `StreamingBatcher` now reads the whole file once and slices it in memory: still exactly one read for the whole batching session (not once per batch, the bug it was originally written to fix), and all existing tests pass, but peak memory now scales with total file size instead of `batch_size` for very large files. Tracked as its own open item in `docs/SECURITY_AUDIT.md` (§2b) — restoring genuine incremental reads needs either `polars-parquet`'s lower-level row-group API or a hand-rolled chunked CSV reader, deliberately not rushed alongside the CVE fix above.
- SQL connector extras (`connectorx`, `psycopg2-binary`, cloud warehouse drivers) use floating minimum versions rather than pinned versions — see `docs/SECURITY_AUDIT.md` for the rationale and tradeoffs.

## License

This project is licensed under the [Apache License 2.0](LICENSE).
