# airflow-statguardian

Apache Airflow operator for [StatGuardian](https://github.com/Mullassery/StatGuardian)
data-quality contracts.

```bash
pip install -e integrations/airflow-statguardian
```

```python
from airflow import DAG
from datetime import datetime
from airflow_statguardian import StatGuardianValidateOperator

with DAG("orders_quality", start_date=datetime(2026, 1, 1), schedule="@daily") as dag:
    check_orders = StatGuardianValidateOperator(
        task_id="check_orders",
        contract_path="/opt/airflow/contracts/orders.sg",
        data_path="/opt/airflow/data/orders.parquet",
        fail_on_warning=False,
    )
```

The task calls `statguardian.execute_file()` directly (no subprocess) and fails
with `AirflowException` when the report does not pass. `contract_path`,
`data_path`, and `reference_path` are templated fields.

`fail_on_warning=True` fails on any violation, matching
`statguardian validate --fail-on-warning`.

## Scope

- Works with local files and the formats StatGuardian's `execute_file` supports
  (Parquet/CSV/JSON/Avro/ORC/Arrow IPC/Delta/Iceberg directories).
- Warehouse/SQL validation should use `execute_sql` in a Python task until this
  operator grows a connection-aware variant.
