# StatGuardian

A Rust-native data quality engine with a declarative contract DSL for schema validation, drift detection, and anomaly detection for Pandas and Polars DataFrames.

> **Not to be confused with pipeline validators like [PyDBTGuard](https://github.com/Mullassery/PyDBTGuard) or [PyBeamGuard](https://github.com/Mullassery/PyBeamGuard):** those check pipeline definitions before they run; StatGuardian checks the quality of the *data* at runtime.

## Which project do I need?

Part of the Mullassery open-source family. These tools have similar names — here is the short version of who does what:

| If you need to… | Use |
| --- | --- |
| Build a repository's dependency graph and match it against vulnerability advisories (Python, Rust, Node, containers) | [PyDeepMesh](https://github.com/Mullassery/PyDeepMesh) |
| Understand *why* a Python dependency exists, who added it, and whether it is actually used | [PyDependencyCheck](https://github.com/Mullassery/PyDependencyCheck) |
| Statically analyze a **dbt** project (risky tests, test blast radius, cost) | [PyDBTGuard](https://github.com/Mullassery/PyDBTGuard) |
| Statically analyze **Apache Beam / Dataflow** pipelines (cost, hot keys, reliability) | [PyBeamGuard](https://github.com/Mullassery/PyBeamGuard) |
| Validate **Airflow** DAGs and dbt runs | [PyAirflowTester](https://github.com/Mullassery/PyAirflowTester) |
| Simulate cascading-failure **blast radius** across infrastructure, data pipelines, and AI systems | [PyBlastRadius](https://github.com/Mullassery/PyBlastRadius) |
| Check the **quality of the data itself** at runtime (schema, drift, anomalies) | [StatGuardian](https://github.com/Mullassery/StatGuardian) |
| Discover **devices, services, and vulnerabilities on a network** | [PyNetworkIntel](https://github.com/Mullassery/PyNetworkIntel) |
| Discover and score **API security** risk (real traffic, agent/MCP, Cedar policy) | [PyAPICheck](https://github.com/Mullassery/PyAPICheck) |
| Trace and root-cause **AI agent** failures | [PyAgentHound](https://github.com/Mullassery/PyAgentHound) |
| Diagnose **RAG retrieval** failures | [PyVectorHound](https://github.com/Mullassery/PyVectorHound) |
| Orchestrate **local, dependency-aware tasks** (DAG scheduler + content-addressed cache) | [PyTaskWireCheck](https://github.com/Mullassery/PyTaskWireCheck) |

[Browse all projects →](https://github.com/Mullassery?tab=repositories)

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

## HTTP service

```bash
pip install "statguardian[api]"
statguardian serve --contract orders.sg --host 127.0.0.1 --port 8000
```

`POST /validate` accepts `{"entity": {"attributes": {...}}}` and returns
`passed`, `quality_score`, `issues`, and `schema_version` for PyReverseETL's
`StatGuardianClient`. `/detect-changes` is explicitly 501, not faked.
See [`deploy/README.md`](deploy/README.md) for a Docker Compose deployment.

## Integrations

- **dbt** — `integrations/dbt-statguardian`.
- **Apache Airflow** — `integrations/airflow-statguardian` provides
  `StatGuardianValidateOperator`, which runs a `.sg` contract against a data
  path and fails the task when validation does not pass.

## Documentation

- [CLI documentation](docs/CLI.md)
- [StreamGuard (streaming validation)](docs/STREAMGUARD.md)
- [Data contract DSL](docs/contract-dsl.md)
- [Benchmarking](docs/bench/benchmark.py)
- [Honest roadmap & limitations](docs/ROADMAP_HONEST.md)

## Known issues and deferred work

**Blocked / failed:**
- Runtime Docker validation (daemon unavailable here).
- Streaming bounded-memory regression; reproducible benchmarks.

**Rejected / deferred:**
- Dashboards/alerting platform.

Full shipped/blocked/rejected detail: [`QUEUE_STATUS.md`](QUEUE_STATUS.md).

## Queue status

Shipped, blocked/failed, and rejected work for this repository is documented in
[`QUEUE_STATUS.md`](QUEUE_STATUS.md).

## Contributing

Contributions are welcome. Please review [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on development setup, code style, and testing.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
