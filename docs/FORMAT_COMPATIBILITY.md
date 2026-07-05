# Format, Storage & Connector Compatibility

Comprehensive matrix comparing StatGuard against Pydantic, pandera, and Great Expectations.
All StatGuard connectors use open-source licensed drivers only (MIT / Apache-2.0 / BSD).

**Verified against:** StatGuard 0.1 · Pydantic 2.13 · pandera 0.31 · Great Expectations 1.18

---

## File formats

| Format | StatGuard | pandera | Great Expectations | Pydantic v2 |
|---|---|---|---|---|
| **Parquet** | ✓ native | ✓ via pandas/polars | ✓ via pandas or Spark | ✗ load first |
| **CSV / TSV** | ✓ native | ✓ via pandas/polars | ✓ via pandas or Spark | ✗ load first |
| **JSON / NDJSON** | ✓ native | ✓ via pandas/polars | ✓ via pandas or Spark | ✓ native dicts |
| **Arrow IPC** | ✓ native | ✓ via pyarrow | ✗ | ✗ load first |
| **Avro** | ✓ native | ✓ via fastavro | ✓ via Spark | ✗ load first |
| **ORC** | ✓ opt-in | ✓ via pyarrow | ✓ via Spark | ✗ load first |
| **Excel** | ✗ | ✓ via openpyxl | ✓ via pandas | ✗ load first |

---

## Lakehouse table formats

| Format | StatGuard | pandera | Great Expectations | Pydantic v2 |
|---|---|---|---|---|
| **Delta Lake** | ✓ native (no Spark) | ✗ | ✓ Spark required | ✗ |
| **Apache Iceberg** | ✓ native (no Spark) | ✗ | ✓ Spark required | ✗ |
| **Apache Hudi** | ✗ (roadmap) | ✗ | ✓ Spark required | ✗ |
| **Apache Hive** | ✗ | ✗ | ✓ Spark required | ✗ |

### Delta Lake — features

| Feature | StatGuard | Great Expectations |
|---|---|---|
| Read current snapshot | ✓ | ✓ (Spark required) |
| Time-travel by version | ✓ | ✓ (Spark required) |
| Time-travel by timestamp | ✓ | ✓ (Spark required) |
| Two-version drift comparison | ✓ built-in | ✗ |
| Local use (no cluster) | ✓ | ✗ |
| Dependency | none (pure Rust log replay) | PySpark + delta-spark JAR |

### Apache Iceberg — features

| Feature | StatGuard | Great Expectations |
|---|---|---|
| Read current snapshot | ✓ | ✓ (Spark required) |
| Time-travel by snapshot ID | ✓ | ✓ (Spark required) |
| Time-travel by timestamp | ✓ | ✓ (Spark required) |
| Named branch / tag reads | ✓ (`read_ref("main")`) | ✗ |
| Snapshot listing | ✓ (`list_iceberg_snapshots()`) | ✗ |
| Two-snapshot drift comparison | ✓ built-in | ✗ |
| Spec version | v1 + v2 | via Spark/Iceberg JAR |
| Local use (no cluster) | ✓ | ✗ |
| Dependency | none (pure Rust metadata parsing) | PySpark + Iceberg JAR |

---

## Cloud storage (S3, GCS, Azure)

| Feature | StatGuard | pandera | Great Expectations | Pydantic v2 |
|---|---|---|---|---|
| **AWS S3** | ✓ (`s3://`) | ✓ via s3fs/pandas | ✓ native datasource | ✗ |
| **S3-compatible** (MinIO, Ceph, R2) | ✓ (`s3://` + `AWS_ENDPOINT_URL`) | ✓ via s3fs | ✗ | ✗ |
| **Google Cloud Storage** | ✓ (`gs://`) | ✓ via gcsfs/pandas | ✓ native datasource | ✗ |
| **Azure Blob Storage** | ✓ (`az://`, `abfss://`) | ✓ via adlfs/pandas | ✓ native datasource | ✗ |
| **DBFS (Databricks)** | ✗ | ✗ | ✓ native datasource | ✗ |
| Glob patterns (`*.parquet`) | ✓ | ✗ | ✓ | ✗ |
| Partitioned dataset scan | ✓ | ✗ | ✓ | ✗ |
| Delta Lake on S3/GCS/Azure | ✗ (roadmap) | ✗ | ✓ via Spark | ✗ |

### Cloud authentication

StatGuard picks up credentials automatically from standard env vars — no
config file needed:

```bash
# AWS S3
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_DEFAULT_REGION=us-east-1

# MinIO / S3-compatible
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin
export AWS_ENDPOINT_URL=http://localhost:9000

# Google Cloud Storage
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json

# Azure Blob Storage
export AZURE_STORAGE_ACCOUNT=myaccount
export AZURE_STORAGE_ACCESS_KEY=mykey
```

Instance profiles, Workload Identity, and Managed Identity are also picked
up automatically when running on EC2/GKE/Azure VMs.

### Cloud API

```python
import statguardian

# Read directly from cloud — format auto-detected from extension
report = statguardian.execute_cloud(contract, "s3://bucket/events/2026/06/*.parquet")
report = statguardian.execute_cloud(contract, "gs://bucket/data.csv")
report = statguardian.execute_cloud(contract, "az://container/data.parquet")

# Or use execute_file — same thing, auto-detects cloud URIs
report = statguardian.execute_file(contract, "s3://bucket/data.parquet")

# With drift reference (yesterday's data)
report = statguardian.execute_cloud(
    contract,
    uri="s3://bucket/events/today/*.parquet",
    reference_uri="s3://bucket/events/yesterday/*.parquet",
)
```

### Build with cloud support

Cloud features are opt-in at compile time (keeps the binary small by default):

```bash
# S3 only
maturin build --release --cargo-extra-args="--features s3"

# All cloud providers
maturin build --release --cargo-extra-args="--features cloud"

# Everything
maturin build --release --cargo-extra-args="--features full"
```

---

## SQL databases and warehouses

### Open-source connectors (all MIT / Apache-2.0)

| Database | URL scheme | Driver | Layer | License |
|---|---|---|---|---|
| **PostgreSQL** | `postgresql://` | sqlx (pure Rust) | Rust | MIT/Apache-2.0 |
| **CockroachDB** | `postgresql://` | sqlx (PostgreSQL wire) | Rust | MIT/Apache-2.0 |
| **TimescaleDB** | `postgresql://` | sqlx (PostgreSQL wire) | Rust | MIT/Apache-2.0 |
| **MySQL** | `mysql://` | sqlx (pure Rust) | Rust | MIT/Apache-2.0 |
| **MariaDB** | `mysql://` | sqlx (pure Rust) | Rust | MIT/Apache-2.0 |
| **PlanetScale** | `mysql://` | sqlx (MySQL wire) | Rust | MIT/Apache-2.0 |
| **SQLite** | `sqlite:///` | sqlx (bundled libsqlite3) | Rust | MIT/Apache-2.0 |
| **DuckDB** | `duckdb:///` | connectorx | Python | MIT |
| **Google BigQuery** | `bigquery://` | google-cloud-bigquery | Python | Apache-2.0 |
| **Snowflake** | `snowflake://` | snowflake-connector-python | Python | Apache-2.0 |
| **Amazon Redshift** | `redshift+psycopg2://` | amazon-redshift-python-driver | Python | Apache-2.0 |
| **Databricks SQL** | `databricks+connector://` | databricks-sql-connector | Python | Apache-2.0 |
| **ClickHouse** | `clickhouse+http://` | clickhouse-driver | Python | MIT |
| **Trino / Presto** | `trino://` | trino-python-client | Python | Apache-2.0 |
| **AWS Athena** | `awsathena+rest://` | PyAthena | Python | MIT |

### Intentionally excluded (proprietary drivers)

| Database | Reason |
|---|---|
| **Oracle** | Requires Oracle Instant Client (proprietary, non-redistributable) |
| **SQL Server** | Microsoft ODBC Driver is proprietary on Linux/macOS |

> If you need Oracle or SQL Server, export your data to Parquet and use
> `statguardian.execute_file()`. This keeps all validation open-source.

### SQL API

```python
import statguardian

contract = statguardian.DataContract.from_file("orders.sg")

# PostgreSQL (Rust layer — fastest)
report = statguardian.execute_sql(
    contract,
    connection_string="postgresql://user:pass@localhost:5432/mydb",
    query="SELECT * FROM orders WHERE date >= '2026-01-01'",
)

# BigQuery (Python layer via google-cloud-bigquery)
# pip install google-cloud-bigquery
report = statguardian.execute_sql(
    contract,
    connection_string="bigquery://my-project/my-dataset",
    query="SELECT * FROM events LIMIT 1000000",
)

# Snowflake (Python layer via snowflake-connector-python)
# pip install snowflake-connector-python connectorx
report = statguardian.execute_sql(
    contract,
    connection_string="snowflake://user:pass@account/db/schema?warehouse=wh",
    query="SELECT * FROM ORDERS",
)

# Redshift (Python layer)
# pip install amazon-redshift-python-driver connectorx
report = statguardian.execute_sql(
    contract,
    connection_string="redshift+psycopg2://user:pass@host.redshift.amazonaws.com:5439/db",
    query="SELECT * FROM events",
)

# With drift reference (compare two queries)
report = statguardian.execute_sql(
    contract,
    connection_string="postgresql://localhost/db",
    query="SELECT * FROM events WHERE date = CURRENT_DATE",
    reference_query="SELECT * FROM events WHERE date = CURRENT_DATE - 1",
)
```

### Build with SQL support

```bash
# PostgreSQL
maturin build --release --cargo-extra-args="--features sql-postgres"

# All SQL (Postgres + MySQL + SQLite)
maturin build --release --cargo-extra-args="--features sql"

# BigQuery / Snowflake / Redshift / Databricks — Python layer, no Rust build flag needed
pip install connectorx                    # MIT, fastest
pip install google-cloud-bigquery         # BigQuery
pip install snowflake-connector-python    # Snowflake
pip install amazon-redshift-python-driver # Redshift
pip install databricks-sql-connector      # Databricks
pip install clickhouse-driver             # ClickHouse
```

---

## Apache Spark integration

StatGuard accepts PySpark DataFrames directly via the Arrow columnar bridge —
no data serialisation to Python dicts.

```python
from pyspark.sql import SparkSession
import statguardian

# Enable Arrow for efficient transfer (required)
spark = SparkSession.builder \
    .config("spark.sql.execution.arrow.pyspark.enabled", "true") \
    .getOrCreate()

contract = statguardian.DataContract.from_file("events.sg")
spark_df  = spark.read.parquet("s3a://bucket/events/")

# Validate
report = statguardian.execute_spark(contract, spark_df)
print(report.summary())

# Drift detection — compare two Spark DataFrames
today_df     = spark.read.parquet("s3a://bucket/events/today/")
yesterday_df = spark.read.parquet("s3a://bucket/events/yesterday/")

report = statguardian.execute_spark(contract, today_df, reference_spark_df=yesterday_df)
for d in report.drift_results():
    print(f"{d['column']}.{d['stat']}: PSI={d['psi']:.4f}")
```

### Spark requirements

- PySpark 3.0+ (`pip install pyspark`)
- Arrow enabled: `spark.sql.execution.arrow.pyspark.enabled = true`
- pyarrow: `pip install pyarrow`

Works on: local mode, YARN, Kubernetes, Databricks, AWS EMR, Google Dataproc, Azure HDInsight.

---

## Compute engines

| Engine | StatGuard | pandera | Great Expectations | Pydantic v2 |
|---|---|---|---|---|
| Polars | ✓ native (primary) | ✓ experimental | ✗ | ✗ |
| Pandas | ✓ via Polars convert | ✓ primary | ✓ primary | ✗ |
| Apache Spark | ✓ Arrow bridge | ✓ separate install | ✓ native | ✗ |
| Dask | ✗ | ✗ | ✗ | ✗ |
| Ray | ✗ | ✗ | ✗ | ✗ |

---

## Feature summary

| Capability | StatGuard | pandera | Great Expectations | Pydantic v2 |
|---|---|---|---|---|
| Native file formats | 8 | 0 (via pandas) | 0 (via pandas) | 0 |
| Cloud storage (S3/GCS/Azure) | ✓ | via extras | ✓ native | ✗ |
| Delta Lake without Spark | **✓** | ✗ | ✗ | ✗ |
| Iceberg without Spark | **✓** | ✗ | ✗ | ✗ |
| Auto-detect format from path | **✓** | ✗ | ✗ | ✗ |
| SQL databases (OSS drivers) | 3 Rust + 10 Python | via SQLAlchemy | ✓ (12 connectors) | ✗ |
| SQL warehouses | 5 via Python | via extras | ✓ | ✗ |
| Spark DataFrames | ✓ Arrow bridge | ✓ | ✓ native | ✗ |
| **Kafka (🗺️ roadmap)** | **planned** | ✗ | ✗ | ✗ |
| **Apache Flink (🗺️ roadmap)** | **planned** | ✗ | ✗ | ✗ |
| **Airflow operator (🗺️ roadmap)** | **planned** | partial | partial | ✗ |
| OSS-only connectors | **✓ enforced** | ✗ (no policy) | ✗ (no policy) | n/a |
| Glob / partitioned reads | ✓ | ✗ | ✓ | ✗ |

🗺️ = roadmap item (not yet implemented, architecture planned)

---

## Roadmap — streaming & orchestration

### Kafka

```python
# (Planned) Validate a Kafka topic as a micro-batch stream
topic_config = {
    "bootstrap_servers": ["localhost:9092"],
    "topic": "orders",
    "group_id": "statguard-validation",
}
reports = statguardian.execute_kafka(contract, topic_config, batch_size=1000)
```

Open-source drivers: `confluent-kafka-python` (Apache-2.0) or `kafka-python` (MIT).

### Apache Flink

```python
# (Planned) Validate Flink DataStream in Python
env = StreamExecutionEnvironment.get_execution_environment()
ds = env.add_source(...)  # Kafka, S3, etc.

# Convert to Arrow table batch, validate
report = statguardian.execute_flink(contract, ds)
```

Integration via Flink Python API + Arrow IPC serialisation (zero-copy).

### Apache Airflow operator

```python
# (Planned) Orchestrate validation as an Airflow DAG task
from statguardian.airflow import StatGuardOperator

validation_task = StatGuardOperator(
    task_id="validate_orders",
    contract_path="orders.sg",
    data_path="s3://bucket/orders/{{ ds }}/",
    reference_path="s3://bucket/orders/{{ yesterday_ds }}/",
    fail_on_error=True,
)
```

Uses the standard Airflow task API. No Airflow-specific code in StatGuard itself—
the operator is a thin wrapper around `execute_cloud()` / `execute_sql()` / `execute_spark()`.

Available as: `pip install statguardian[airflow]` (installs `apache-airflow` + orchestration adapter).

---

## Implementation notes — open-source policy

All connectors use **MIT / Apache-2.0 / BSD licensed** drivers only:

**Included:**
- `confluent-kafka-python` (Apache-2.0) — Kafka
- `flink` Python API (Apache-2.0) — Flink
- `apache-airflow` (Apache-2.0) — Airflow
- `connectorx` (MIT) — SQL
- `polars` (MIT) — cloud storage, Arrow
- `pyspark` (Apache-2.0) — Spark

**Intentionally excluded:**
- Oracle Instant Client (proprietary)
- Microsoft ODBC Driver (proprietary on Linux/macOS)
- Salesforce Analytics Cloud (proprietary SDK)
- Proprietary data warehouse native connectors (export to Parquet instead)
