# StatGuardian

A Rust-native data quality engine with a declarative contract DSL: schema validation, drift detection, and anomaly detection for Pandas and Polars.

[![Tests](https://img.shields.io/github/actions/workflow/status/Mullassery/StatGuardian/tests.yml?label=tests)](https://github.com/Mullassery/StatGuardian/actions)
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

Full CLI usage: [docs/CLI.md](docs/CLI.md). DSL syntax: see `examples/*.sg`.

## Installation

```bash
pip install statguardian
```

For development:
```bash
git clone https://github.com/Mullassery/StatGuardian
cd StatGuardian
pip install -e ".[dev]"
pytest
```

## Documentation

- [CLI Reference](docs/CLI.md)
- [Security Audit](docs/SECURITY_AUDIT.md)
- [Roadmap](docs/ROADMAP.md)
- [Examples](examples/)
- [Contributing](CONTRIBUTING.md)

## Known Issues

- `execute()` accepts a Polars DataFrame, not a raw pandas DataFrame. Passing a pandas DataFrame directly raises an unhelpful `AttributeError` (verified against the current build) rather than converting automatically — call `pl.from_pandas(df)` first. The `pandas` extra is used by the SQL/Spark/GPU connectors internally, which already do this conversion for you.
- Performance numbers are not yet published as a reproducible, checked-in benchmark result — `docs/bench/benchmark.py` exists but its output has never been committed. Treat any speed claims (including from this project) as unverified until you've run the benchmark yourself.
- `docs/ROADMAP.md`, `docs/ROADMAP_HONEST.md`, and `docs/ROADMAP_INTEGRATED.md` currently overlap and are not kept in sync — some content in `ROADMAP_HONEST.md` predates features (e.g. Iceberg support) that have since shipped. Treat `docs/SECURITY_AUDIT.md` as the current source of truth for security status; the roadmap docs need consolidation.
- SQL connector extras (`connectorx`, `psycopg2-binary`, cloud warehouse drivers) use floating minimum versions rather than pinned versions — see `docs/SECURITY_AUDIT.md` for the rationale and tradeoffs.

## License

Proprietary — free to use with attribution. See [LICENSE](LICENSE).
