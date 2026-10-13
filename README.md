# StatGuardian

A Rust-native data quality engine with a declarative contract DSL for schema validation, drift detection, and anomaly detection for Pandas and Polars DataFrames.

## What it is

StatGuardian validates data at runtime against versionable contracts written in a declarative `.sg` DSL. It catches schema violations, statistical drift, and anomalies before they reach downstream consumers. The core validation and statistics logic run in Rust while providing a Python API for integration into data pipelines.

## Features / Highlights

- **Declarative contracts**: Define schema, quality rules, statistical drift thresholds, and anomaly checks in one `.sg` file.
- **Rust-native execution**: Core checks and statistics execute in compiled Rust for performance.
- **Multi-backend support**: Validate Pandas and Polars DataFrames with the same contract. Delta Lake and Apache Iceberg tables are supported as inputs.
- **Drift detection**: Detect distribution shifts between datasets using PSI and KS tests via the DSL.
- **Anomaly detection**: Identify outliers and duplicates with built-in rules.
- **Blocking controls**: Severity tiers let you mark critical checks as `@blocking` to gate pipeline execution.
- **Detailed reports**: Structured violation messages, scores, and per-check results.

## Who it's for / Use cases

- **Data engineering pipelines**: Enforce contracts on batch or streaming inputs before writes.
- **MLOps/feature pipelines**: Catch schema or distribution drift between training and inference data.
- **ETL/data integration**: Validate external data against agreed schemas and quality thresholds.
- **Data governance**: Version and review contracts alongside code as the source of truth.

## What it is not

- **Not a full observability platform**: Focused on contract-based validation; does not include dashboards/alerting infrastructure beyond returning structured reports.
- **Not a generic testing framework**: Designed for data contract enforcement at runtime and in pipelines.
- **Benchmarks vary**: Performance depends on workload (row count, columns, checks). Run benchmarks against your own data before making comparisons.

## Project status

StatGuardian is in active development. Core features (schema validation, quality rules, drift, anomaly detection, multi-backend DataFrame support) are implemented. Streaming validation (StreamGuard) and additional integrations are documented under development paths; see [docs/ROADMAP_HONEST.md](docs/ROADMAP_HONEST.md) for known limitations and planned work.

## Installation

### From PyPI

```bash
pip install statguardian
```

### From source (Python + Rust)

```bash
pip install maturin
maturin develop
```

### System requirements

- Python 3.8+
- Rust toolchain (for source builds)

## Quick start

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

## Usage

Validate a Polars DataFrame against a contract:

```python
import statguardian

contract = statguardian.DataContract.from_dsl("dataset orders { schema { id: string, not_null } }")
report = statguardian.execute(contract, df)
```

Drift detection between reference and incoming data:

```python
report = statguardian.execute(contract, incoming_df, reference=baseline_df)
for d in report.drift_results():
    if not d["passed"]:
        print(f"Drift: {d['column']} PSI={d.get('psi', 0):.4f}")
```

Load a contract from a `.sg` file:

```python
contract = statguardian.DataContract.from_file("contracts/orders.sg")
```

Note: When validating a Pandas DataFrame, convert it to Polars first (e.g., `pl.from_pandas(df)`) before calling `execute()`.

## Documentation

- [CLI documentation](docs/CLI.md)
- [StreamGuard (streaming validation)](docs/STREAMGUARD.md)
- [Data contract DSL](docs/contract-dsl.md)
- [Benchmarking](docs/bench/benchmark.py)
- [Honest roadmap & limitations](docs/ROADMAP_HONEST.md)

## Contributing

Contributions are welcome. Please review [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on development setup, code style, and testing.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
