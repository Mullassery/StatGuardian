# StatGuard Open Source Licenses

StatGuard is licensed under the **MIT License**. This document details the licenses of all included and optional dependencies.

## Core Project License

- **StatGuard** — MIT License (see [LICENSE](LICENSE))

---

## Workspace Dependencies (All OSI-Approved Open Source)

All dependencies included in the default build use MIT, Apache-2.0, or BSD licenses:

| Package | Version | License | Category |
|---------|---------|---------|----------|
| **Data Processing** |
| polars | 0.44+ | MIT | core |
| pyo3-polars | 0.15+ | MIT | Python bindings |
| **Parsing & Compilation** |
| pest | 2.7+ | MIT/Apache-2.0 dual | core |
| **Serialization** |
| serde | 1.0+ | MIT/Apache-2.0 dual | core |
| serde_json | 1.0+ | MIT/Apache-2.0 dual | core |
| **Error Handling** |
| thiserror | 1.0+ | MIT/Apache-2.0 dual | core |
| anyhow | 1.0+ | MIT/Apache-2.0 dual | core |
| **Concurrency** |
| rayon | 1.10+ | MIT/Apache-2.0 dual | core |
| **Utilities** |
| regex | 1.10+ | MIT/Apache-2.0 dual | core |
| chrono | 0.4+ | MIT/Apache-2.0 dual | core |
| indexmap | 2.0+ | MIT/Apache-2.0 dual | core |
| uuid | 1.6+ | MIT/Apache-2.0 dual | core |
| log | 0.4+ | MIT/Apache-2.0 dual | core |
| env_logger | 0.11+ | MIT/Apache-2.0 dual | core |
| pyo3 | 0.21+ | MIT/Apache-2.0 dual | core |
| tempfile | 3+ | MIT/Apache-2.0 dual | dev-only |
| **Testing** |
| pretty_assertions | 1.4+ | MIT/Apache-2.0 dual | dev-only |
| **SQL (Rust layer, optional)** |
| sqlx | 0.8+ | MIT/Apache-2.0 dual | optional, sql feature |
| tokio | 1.0+ | MIT | optional, sql + cloud features |

---

## Optional SQL Dependencies (⚠️ Includes LGPL)

When using `execute_sql()` for PostgreSQL:

### PostgreSQL Driver — ⚠️ LGPL-2.1 with exceptions

- **psycopg2** / **psycopg2-binary** — LGPL-2.1 with exceptions
  - Installed via: `pip install statguard[sql-postgres]`
  - **License Impact**: Using this driver means your application adds an
    LGPL-licensed component. LGPL is a copyleft license, but more permissive
    than GPL:
    - Your source code does NOT need to be LGPL
    - **Binary distributions must** either:
      1. Include the psycopg2 source code, OR
      2. Provide instructions on how to download it
    - See: https://www.postgresql.org/about/licence/

### Other SQL Drivers — MIT or Apache-2.0

- **PyMySQL** — MIT (MySQL/MariaDB)
- **sqlalchemy** — MIT (SQL toolkit)
- **connectorx** — MIT (query execution layer)
- **google-cloud-bigquery** — Apache-2.0 (BigQuery)
- **snowflake-connector-python** — Apache-2.0 (Snowflake)
- **amazon-redshift-python-driver** — Apache-2.0 (Redshift)
- **databricks-sql-connector** — Apache-2.0 (Databricks)
- **clickhouse-driver** — MIT (ClickHouse)
- **duckdb** — MIT (DuckDB)
- **trino-python-client** — Apache-2.0 (Trino)

---

## Optional Cloud Dependencies — MIT or Apache-2.0

All cloud storage drivers are OSS with permissive licenses:

- **Polars cloud features** (s3, gcp, azure) — MIT
  - Uses Arrow's object_store crate (Apache-2.0)
  - Installed via: `pip install statguard[cloud]`
  - Includes AWS SDK, Google Cloud client, Azure SDK (all Apache-2.0)

---

## Optional Spark Dependency — Apache-2.0

- **pyspark** — Apache-2.0 (Spark integration)
  - Installed via: `pip install statguard[spark]`

---

## Roadmap Dependencies (Not Yet Included)

These are documented for future releases but not yet in use:

- **confluent-kafka-python** — Apache-2.0 (Kafka, planned)
- **apache-airflow** — Apache-2.0 (Airflow operator, planned)
- **pyflink** — Apache-2.0 (Flink, planned)

---

## Intentionally Excluded — Proprietary Drivers

The following are **NOT included** and cannot be added due to proprietary licensing:

| System | Reason | Workaround |
|--------|--------|-----------|
| **Oracle Database** | Requires Oracle Instant Client (proprietary, non-open-source) | Export data to Parquet and validate with `execute_file()` |
| **Microsoft SQL Server** | Official ODBC driver is proprietary on Linux/macOS | Export data to Parquet and validate with `execute_file()` |
| **Salesforce** | Proprietary SDK | Export data to Parquet |

---

## Compliance Summary

✅ **StatGuard Core**: MIT (permissive, no restrictions)

⚠️ **With psycopg2** (PostgreSQL): LGPL-2.1 with exceptions
   - Binary distributions must include source/download instructions

✅ **With all other optionals**: MIT or Apache-2.0 (no additional restrictions)

❌ **Never included**: Proprietary ODBC/JDBC drivers (by policy)

---

## Using StatGuard in Your Project

### If your project is MIT/Apache-2.0 licensed:

✅ Safe to use: All features (include cloud, Spark, SQL with non-PostgreSQL DBs)

⚠️ With care: PostgreSQL support (psycopg2 is LGPL — add LGPL notice to your NOTICES/COPYING file)

### If your project is GPL:

✅ Safe to use: All features (GPL is compatible with LGPL)

### If your project is proprietary/closed-source:

✅ Safe to use: Core + Cloud + Spark + non-PostgreSQL SQL (MIT/Apache-2.0)

⚠️ With compliance effort: PostgreSQL support (psycopg2 is LGPL — must provide source/download instructions with binary)

---

## For Maintainers

When adding new dependencies:

1. ✅ Prefer: MIT, Apache-2.0, BSD, ISC licenses (permissive, no restrictions)
2. ⚠️ Consider: LGPL (copyleft, but acceptable with documented restrictions)
3. ❌ Avoid: GPL (unless project is GPL), proprietary licenses

Update this file when adding new dependencies with their licenses clearly documented.
