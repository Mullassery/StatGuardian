# StatGuard CLI Reference

The `statguard` command-line tool validates data files and DSL contracts from the terminal.

---

## Installation

The CLI is included with the base `statguard` package:

```bash
pip install statguardian
statguard --version
```

For cloud storage support (S3, GCS, Azure):

```bash
pip install statguardian[cloud]
```

For SQL databases:

```bash
pip install statguardian[sql]           # All SQL databases
pip install statguardian[sql-postgres]  # PostgreSQL only
pip install statguardian[sql-snowflake] # Snowflake only
```

---

## Commands

### `statguardian validate` — Validate data

Validate a data file against a contract and output results.

**Syntax:**

```bash
statguardian validate --contract <path.sg> --file <path> [--reference <path>] [--format {summary,json,prometheus}] [--fail-on-warning]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `--contract` | ✓ | Path to `.sg` DSL contract file |
| `--file` | ✓ | Path to data file to validate (see formats below) |
| `--reference` | ✗ | Optional reference data file for drift detection |
| `--format` | ✗ | Output format: `summary` (default), `json`, or `prometheus` |
| `--fail-on-warning` | ✗ | Exit with code 1 if ANY violation found (default: only errors) |

**Return codes:**

- `0` — Validation passed
- `1` — Validation failed (errors or warnings if `--fail-on-warning`)
- `2` — Error executing validation (invalid file, DSL syntax error, etc.)

### Supported file formats

The CLI auto-detects format from file extension or directory structure:

| Extension | Format | Example |
|-----------|--------|---------|
| `.parquet` | Apache Parquet | `statguardian validate --contract orders.sg --file data.parquet` |
| `.csv`, `.tsv` | CSV/TSV | `statguardian validate --contract orders.sg --file data.csv` |
| `.json`, `.ndjson` | JSON / NDJSON | `statguardian validate --contract orders.sg --file events.json` |
| `.avro` | Apache Avro | `statguardian validate --contract orders.sg --file data.avro` |
| `.orc` | Apache ORC | `statguardian validate --contract orders.sg --file data.orc` |
| `.ipc`, `.arrow` | Apache Arrow IPC | `statguardian validate --contract orders.sg --file data.ipc` |
| `_delta_log/` dir | Delta Lake | `statguardian validate --contract orders.sg --file /path/to/delta_table/` |
| `metadata/` dir | Apache Iceberg | `statguardian validate --contract orders.sg --file /path/to/iceberg_table/` |

### Output formats

#### `summary` (default)

Human-readable one-liner:

```bash
$ statguardian validate --contract orders.sg --file data.parquet
[StatGuard] PASS ✓ | dataset=orders | score=0.97 (A) | rows=500000 | violations=2 | 3ms
```

#### `json`

Structured JSON (suitable for parsing):

```bash
$ statguardian validate --contract orders.sg --file data.parquet --format json
{
  "id": "a1b2c3d4-...",
  "dataset": "orders",
  "passed": true,
  "health": { "score": 0.97, "grade": "A" },
  "violations": [...],
  "drift_results": [...],
  ...
}
```

#### `prometheus`

Prometheus text format (for scraping):

```bash
$ statguardian validate --contract orders.sg --file data.parquet --format prometheus
# HELP statguard_validation_passed Dataset validation passed (0=fail, 1=pass)
# TYPE statguard_validation_passed gauge
statguard_validation_passed{dataset="orders"} 1.0
statguard_validation_score{dataset="orders"} 0.97
...
```

### Examples

#### Basic validation

```bash
statguardian validate --contract orders.sg --file orders.parquet
```

#### With drift detection

```bash
statguardian validate --contract orders.sg \
  --file today.parquet \
  --reference yesterday.parquet
```

#### Validate Delta Lake

```bash
statguardian validate --contract orders.sg \
  --file /data/orders_delta/
```

#### Validate Iceberg table

```bash
statguardian validate --contract orders.sg \
  --file /data/orders_iceberg/
```

#### Parse as JSON for downstream tools

```bash
statguardian validate --contract orders.sg --file data.parquet --format json | jq '.violations'
```

#### Prometheus scraping (Datadog, Grafana, etc.)

```bash
statguardian validate --contract orders.sg --file data.parquet --format prometheus | curl --data-binary @- http://localhost:9091/metrics/job/statguard
```

#### CI/CD pipeline gate (fail on any warning)

```bash
# In GitHub Actions / GitLab CI / Jenkins:
statguardian validate --contract orders.sg --file data.parquet --fail-on-warning
```

---

### `statguardian check` — Syntax-check DSL

Validate DSL syntax without validating data. Useful for CI pipeline linting.

**Syntax:**

```bash
statguardian check --contract <path.sg>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `--contract` | ✓ | Path to `.sg` DSL contract file |

**Return codes:**

- `0` — DSL is syntactically valid
- `1` — DSL has syntax errors

**Examples:**

```bash
# Check syntax only (no data needed)
statguardian check --contract orders.sg

# Use in CI pipeline
if statguardian check --contract orders.sg; then
  echo "Contract valid, proceeding..."
else
  echo "Contract syntax error!"
  exit 1
fi
```

---

## Integration examples

### GitHub Actions

```yaml
name: Validate data quality

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: "3.10"
      
      - run: pip install statguardian[cloud]
      - run: statguardian check --contract contracts/orders.sg
      - run: statguardian validate --contract contracts/orders.sg \
              --file s3://my-bucket/data/orders.parquet \
              --fail-on-warning
```

### dbt post-hook

```sql
{{ config(
  post_hook="statguardian validate --contract contracts/{{ this.name }}.sg --file {{ this.identifier }}.parquet --fail-on-warning"
) }}

SELECT * FROM raw_orders
```

### Airflow DAG

```python
from airflow.operators.bash import BashOperator

validate_orders = BashOperator(
    task_id="validate_orders",
    bash_command="""
        statguardian validate \
          --contract /contracts/orders.sg \
          --file s3://my-bucket/orders/{{ ds }}/ \
          --reference s3://my-bucket/orders/{{ yesterday_ds }}/ \
          --fail-on-warning
    """,
)
```

### Cron job (daily validation)

```bash
#!/bin/bash
# /usr/local/bin/validate_data_daily.sh

DATE=$(date +%Y-%m-%d)
LOG_FILE="/var/log/statguard/${DATE}.log"

statguardian validate \
  --contract /etc/statguard/orders.sg \
  --file /data/orders/${DATE}.parquet \
  --reference /data/orders/$(date -d yesterday +%Y-%m-%d).parquet \
  --format json \
  2>&1 | tee "$LOG_FILE"

# Alert if validation failed
if [ $? -ne 0 ]; then
  mail -s "Data quality failure: $DATE" admin@example.com < "$LOG_FILE"
fi
```

---

## Limitations

**CLI currently supports:**
- ✓ Local files (Parquet, CSV, JSON, Avro, ORC, IPC)
- ✓ Delta Lake tables
- ✓ Apache Iceberg tables

**CLI does NOT support (use Python API instead):**
- Cloud storage (S3, GCS, Azure) — use Python `execute_cloud()`
- SQL databases — use Python `execute_sql()`
- Apache Spark — use Python `execute_spark()`
- Streaming (Kafka, Flink) — use Python API

For cloud/SQL/Spark validation, use the Python API:

```python
import statguardian

contract = statguardian.DataContract.from_file("orders.sg")

# Cloud
report = statguardian.execute_cloud(contract, "s3://bucket/data/")

# SQL
report = statguardian.execute_sql(contract, "postgresql://localhost/db", "SELECT * FROM orders")

# Spark
report = statguardian.execute_spark(contract, spark_df)
```

---

## Troubleshooting

### `statguard: command not found`

StatGuard is not installed or not in PATH. Install it:

```bash
pip install statguardian
# Verify:
which statguard
```

### `Error: contract file not found`

The path to the `.sg` file is incorrect. Check that it exists:

```bash
ls -la /path/to/contract.sg
statguardian check --contract /path/to/contract.sg
```

### `Error: data file not found`

The path to the data file doesn't exist or is inaccessible:

```bash
ls -la /path/to/data.parquet
```

For S3, GCS, or Azure URIs, see "Limitations" above — use Python API instead.

### `DSL error: <message>`

The contract file has a syntax error. Check the DSL syntax:

```bash
statguardian check --contract contract.sg
```

See [DSL Reference](../README.md#dsl-reference) for grammar.

### Exit code 2 (execution error)

An unexpected error occurred during validation. Run with Python for more details:

```python
import statguardian

contract = statguardian.DataContract.from_file("contract.sg")
report = statguardian.execute_file(contract, "data.parquet")
print(report.summary())
```

---

## Environment variables

### Cloud storage credentials

**AWS S3:**

```bash
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_DEFAULT_REGION=us-east-1
```

**Google Cloud Storage:**

```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

**Azure Blob Storage:**

```bash
export AZURE_STORAGE_ACCOUNT=myaccount
export AZURE_STORAGE_ACCESS_KEY=...
```

These are read automatically when validating cloud URIs (Python API only).
