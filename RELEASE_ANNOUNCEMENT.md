# StatGuard v0.1.0 Release — Production-Grade Data Quality Engine

**Announcing StatGuard: A Rust-native data quality, validation, and statistical drift monitoring engine with a Python API that's 13-25× faster than pandera.**

🚀 **Now available via pip, uv, and curl**

---

## What is StatGuard?

StatGuard is a high-performance data quality platform that compiles a declarative **DSL contract** into an optimised columnar execution plan. Write once, validate everywhere — across Parquet, CSV, JSON, Delta Lake, Apache Iceberg, S3, GCS, Azure, PostgreSQL, BigQuery, Snowflake, and Apache Spark.

**Stack:** Rust 2021 · Polars 0.44 · PyO3 · maturin · pest PEG grammar · Rayon · Apache Arrow

**License:** MIT (with optional LGPL for PostgreSQL)

---

## Why StatGuard?

| Metric | pandera | Great Expectations | Pydantic v2 | **StatGuard** |
|--------|---------|--------------------|----|---|
| **Performance (100k rows)** | 26.5 ms | 50.4 ms | 43.5 ms | **2.0 ms** |
| **Speedup** | 1× | 25× slower | 22× slower | **13-25× faster** |
| **Delta Lake (no Spark)** | ✗ | ✗ | ✗ | **✓** |
| **Iceberg (no Spark)** | ✗ | ✗ | ✗ | **✓** |
| **Cloud (S3/GCS/Azure)** | via extras | ✓ | ✗ | **✓ native** |
| **SQL (13 databases)** | via SQLAlchemy | 12 connectors | ✗ | **13 OSS connectors** |
| **Spark** | ✓ | ✓ | ✗ | **✓ Arrow bridge** |

---

## Key Features

✅ **Schema & Quality Validation** — types, nulls, ranges, regex, uniqueness, enums  
✅ **Statistical Drift Detection** — PSI, KS test, mean/std/percentile drift  
✅ **Anomaly Detection** — outliers (IQR/z-score), duplicates, cardinality explosions  
✅ **Declarative DSL** — single contract file for all validation rules  
✅ **Lakehouse Formats** — Delta Lake & Iceberg without Spark (pure Rust)  
✅ **Cloud Storage** — S3, GCS, Azure (Polars lazy readers)  
✅ **SQL Databases** — PostgreSQL, MySQL, SQLite, BigQuery, Snowflake, Redshift, Databricks, ClickHouse, DuckDB, Trino  
✅ **Apache Spark** — PySpark DataFrames via Arrow columnar bridge  
✅ **Streaming** — Micro-batch windows with configurable emit strategies  
✅ **Profiling** — min/max/mean/std/percentiles/histogram + HyperLogLog cardinality  
✅ **Multiple Output Formats** — JSON, Prometheus, human-readable summaries  

---

## Quick Start

### Install

```bash
pip install statguardian        # Basic
pip install statguardian[cloud] # Cloud storage
pip install statguardian[sql]   # All SQL databases
pip install statguardian[spark] # Apache Spark
```

### Validate in 3 lines

```python
import statguardian, polars as pl

contract = statguardian.DataContract.from_file("orders.sg")
report = statguardian.execute(contract, pl.read_parquet("orders.parquet"))
print(report.summary())
# [statguardian] PASS ✓ | dataset=orders | score=0.97 (A) | rows=500000 | violations=2 | 3ms
```

### DSL Contract Example

```
dataset orders {
    schema {
        order_id:   string, not_null, unique, primary_key
        amount:     float,  positive, max=100000.0
        status:     string, not_null, enum=["pending","paid","cancelled"]
    }
    quality {
        @blocking: completeness(order_id) > 0.9999
        @warning:  uniqueness(order_id) == 1.0
    }
    stats {
        amount.mean drift < 0.15
        amount.p95  drift < 0.25
    }
    anomalies {
        detect_outliers(amount, method="iqr")
        @blocking: detect_duplicates(order_id)
    }
}
```

---

## Real-World Performance

**100,000 rows × 4 columns** — 5 checks (null, type, range, regex, uniqueness):

```
StatGuard (Rust/Polars)     ~2.0 ms  ← baseline
Polars manual expressions    1.4 ms  (lower bound, no contract overhead)
Pure Python loops           11.5 ms  5.8× slower
pandera 0.31               26.5 ms  13× slower
Great Expectations 1.18    50.4 ms  25× slower
Pydantic v2                43.5 ms  22× slower
```

See [BENCHMARKS.md](https://github.com/Mullassery/statguardian/blob/main/BENCHMARKS.md) for full methodology and scaling tables.

---

## Support Matrix

| Format | StatGuard | Competitors |
|--------|-----------|-------------|
| **Files** (Parquet, CSV, JSON, Avro, ORC) | ✓ native | via pandas |
| **Delta Lake** (without Spark) | **✓ unique** | ✗ |
| **Iceberg** (without Spark) | **✓ unique** | ✗ |
| **Cloud** (S3, GCS, Azure) | ✓ | via extras |
| **SQL** (13 connectors) | ✓ | 8-12 connectors |
| **Spark** | ✓ Arrow bridge | ✓ |

Full compatibility matrix: [docs/FORMAT_COMPATIBILITY.md](https://github.com/Mullassery/statguardian/blob/main/docs/FORMAT_COMPATIBILITY.md)

---

## Use Cases

- **Data pipeline quality gates** — dbt, Airflow, Spark jobs
- **ML feature drift monitoring** — detect distribution shifts automatically
- **Lakehouse validation** — verify Delta/Iceberg snapshots on every write
- **Cloud data warehouse validation** — BigQuery, Snowflake, Redshift
- **Streaming data validation** — Kafka micro-batches with configurable windows
- **CI/CD data contracts** — fail builds on quality violations

---

## Architecture Highlights

**Compilation Pipeline:**
```
DSL text  →  pest parser  →  AST  →  Compiler  →  Optimized DAG
                                              ↓
                          Dedup → Fuse null checks → Cost-sort
                                              ↓
                   BatchExecutor (Rayon parallel per column)
                                              ↓
                              ValidationReport (JSON/Prometheus)
```

**Why It's Fast:**
- Columnar execution (Arrow/Polars SIMD kernels)
- Compiled DAG (not interpreted rules)
- Cost-ordered checks (fail fast on cheap violations)
- Rayon parallelism (all columns concurrently)
- Zero Python allocation per row
- HyperLogLog cardinality (O(1) memory, ~0.8% error)

---

## Licensing & Compliance

✅ **Core:** MIT (permissive, no restrictions)  
✅ **All optional features:** MIT/Apache-2.0 (except PostgreSQL)  
⚠️ **PostgreSQL driver:** LGPL-2.1 (disclosed, requires source with binary distribution)  
❌ **Proprietary drivers:** Intentionally excluded (Oracle, SQL Server ODBC)

Full license audit: [LICENSES.md](https://github.com/Mullassery/statguardian/blob/main/LICENSES.md)

---

## Documentation

| Resource | Link |
|----------|------|
| **Full Guide** | [README.md](https://github.com/Mullassery/statguardian#readme) |
| **Installation** | [INSTALL.md](https://github.com/Mullassery/statguardian/blob/main/INSTALL.md) |
| **CLI Reference** | [docs/CLI.md](https://github.com/Mullassery/statguardian/blob/main/docs/CLI.md) |
| **Format Matrix** | [docs/FORMAT_COMPATIBILITY.md](https://github.com/Mullassery/statguardian/blob/main/docs/FORMAT_COMPATIBILITY.md) |
| **Benchmarks** | [BENCHMARKS.md](https://github.com/Mullassery/statguardian/blob/main/BENCHMARKS.md) |
| **Contributing** | [CONTRIBUTING.md](https://github.com/Mullassery/statguardian/blob/main/CONTRIBUTING.md) |

---

## Getting Started

```bash
# Install
pip install statguardian

# Validate a file
statguardian validate --contract orders.sg --file data.parquet

# Check DSL syntax
statguardian check --contract orders.sg

# Python API
python -c "import statguard; print(statguardian.__version__)"
```

For cloud/SQL/Spark validation:

```python
import statguardian

# Cloud storage
report = statguardian.execute_cloud(contract, "s3://bucket/data/")

# SQL database
report = statguardian.execute_sql(contract, "postgresql://host/db", "SELECT * FROM orders")

# Apache Spark
report = statguardian.execute_spark(contract, spark_df)
```

---

## Roadmap

🗺️ **Planned (not yet released):**
- Kafka topic validation (micro-batch streams)
- Apache Flink integration (Python API)
- Airflow operator (DAG orchestration)
- Cloud storage on Rust layer (currently Python-only)

---

## Community & Contributing

We welcome contributions! See [CONTRIBUTING.md](https://github.com/Mullassery/statguardian/blob/main/CONTRIBUTING.md).

**Test suite:** 33 tests (all passing)  
**Code quality:** `cargo clippy` + `cargo fmt`  
**License:** MIT

---

## Author

**Georgi Mammen Mullassery**  
GitHub: [@Mullassery](https://github.com/Mullassery)  
MIT License © 2026

---

## Links

- **GitHub:** https://github.com/Mullassery/statguardian
- **PyPI:** https://pypi.org/project/statguardian
- **Documentation:** https://github.com/Mullassery/statguardian#readme
- **Issues:** https://github.com/Mullassery/statguardian/issues

---

**Try StatGuard today and experience data quality validation that's fast, comprehensive, and easy to use.**

```bash
pip install statguardian
```
