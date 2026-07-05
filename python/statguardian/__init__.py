"""
StatGuard — High-performance Data Quality & Drift Monitoring Engine
===================================================================

A Rust-native engine with a Python-first API for:
- Schema validation (Pandera-like)
- Data expectations & rules (Great Expectations-like)
- Statistical drift detection (Evidently AI / WhyLogs-like)
- Anomaly detection
- Cross-column conditional assertions in the DSL
- PII detection, schema evolution, HTML reports
- Parallel multi-file validation

Quick start::

    import polars as pl
    import statguardian

    contract = statguardian.DataContract.from_dsl(\"\"\"
    dataset orders {
        schema {
            order_id: string, not_null, unique
            amount:   float,  positive
            status:   string, not_null, enum=["pending","paid","cancelled"]
        }
        quality {
            completeness(order_id) > 0.999
            @blocking: assert amount > 0.0 when status == "paid"
        }
        stats {
            amount.mean drift < 0.15
        }
    }
    \"\"\")

    df = pl.read_parquet("orders.parquet")
    report = statguardian.execute(contract, df)
    print(report.summary())
"""

from ._statguard import (
    DataContract,
    ValidationReport,
    # Core execution
    execute,
    execute_file,
    execute_streaming,
    # Delta Lake
    execute_delta,
    compare_delta_versions,
    # Apache Iceberg
    execute_iceberg,
    list_iceberg_snapshots,
    # Utilities
    validate_dsl,
    __version__,
)

# Python-layer connectors (open-source only — MIT / Apache-2.0)
from ._connectors import (
    execute_sql,    # PostgreSQL, MySQL, SQLite, BigQuery, Snowflake, Redshift, ...
    execute_spark,  # PySpark DataFrames via Arrow bridge
    execute_cloud,  # s3://, gs://, az:// — thin wrapper around execute_file
)

# PII detection
from ._pii import scan_pii, pii_report, PiiFinding

# Schema evolution
from ._evolution import (
    detect_schema_changes,
    schema_evolution_report,
    assert_no_breaking_changes,
    SchemaChange,
)

# HTML report
from ._html import to_html

# Custom Python validators
from ._validators import (
    validator,
    run_custom_validators,
    clear_validators,
    list_validators,
)

# Parallel multi-file validation
from ._parallel import (
    execute_files,
    execute_files_stream,
    FileResult,
)

# GPU / cuDF adapter
from ._gpu import execute_cudf, is_cudf_available

# Referential integrity
from ._integrity import (
    check_referential_integrity,
    check_all_foreign_keys,
    integrity_report,
    IntegrityViolation,
)

__all__ = [
    # Core
    "DataContract",
    "ValidationReport",
    "execute",
    "execute_file",
    "execute_streaming",
    # Lakehouse
    "execute_delta",
    "compare_delta_versions",
    "execute_iceberg",
    "list_iceberg_snapshots",
    # Cloud + SQL + Spark
    "execute_sql",
    "execute_spark",
    "execute_cloud",
    # PII detection
    "scan_pii",
    "pii_report",
    "PiiFinding",
    # Schema evolution
    "detect_schema_changes",
    "schema_evolution_report",
    "assert_no_breaking_changes",
    "SchemaChange",
    # HTML report
    "to_html",
    # Custom validators
    "validator",
    "run_custom_validators",
    "clear_validators",
    "list_validators",
    # Parallel multi-file
    "execute_files",
    "execute_files_stream",
    "FileResult",
    # GPU / cuDF
    "execute_cudf",
    "is_cudf_available",
    # Referential integrity
    "check_referential_integrity",
    "check_all_foreign_keys",
    "integrity_report",
    "IntegrityViolation",
    # Utilities
    "validate_dsl",
    "__version__",
]
