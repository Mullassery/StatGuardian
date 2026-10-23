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

# ML Anomaly detection (CRITICAL: reduces false positives by ~70%)
from ._anomaly_detection import (
    AdaptiveThresholdDetector,
    AnomalyDetectionEngine,
    AnomalyResult,
    SimpleMovingAverageDetector,
)

# Dataframe compatibility (Pandas/PyArrow/Narwhals -> Polars)
from ._compat import execute

# Python-layer connectors (open-source only — MIT / Apache-2.0)
from ._connectors import (
    execute_cloud,  # s3://, gs://, az:// — thin wrapper around execute_file
    execute_spark,  # PySpark DataFrames via Arrow bridge
    execute_sql,  # PostgreSQL, MySQL, SQLite, BigQuery, Snowflake, Redshift, ...
)

# Schema evolution
from ._evolution import (
    SchemaChange,
    assert_no_breaking_changes,
    detect_schema_changes,
    schema_evolution_report,
)

# GPU / cuDF adapter
from ._gpu import execute_cudf, is_cudf_available

# HTML report
from ._html import to_html

# Referential integrity
from ._integrity import (
    IntegrityViolation,
    check_all_foreign_keys,
    check_referential_integrity,
    integrity_report,
)

# Data lineage tracking and versioning
from ._lineage import (
    LineageChange,
    LineageEdge,
    LineageGraph,
    LineageNode,
    LineageVersion,
    create_lineage_edge,
    create_lineage_node,
    get_lineage_graph,
    get_lineage_history,
    get_lineage_version,
    save_lineage_version,
)

# Parallel multi-file validation
from ._parallel import (
    FileResult,
    execute_files,
    execute_files_stream,
)

# PII detection
from ._pii import PiiFinding, pii_report, scan_pii
from ._statguardian import (
    DataContract,
    StreamReport,
    ValidationReport,
    __version__,
    compare_delta_versions,
    # Core execution
    # Delta Lake
    execute_delta,
    execute_file,
    # Apache Iceberg
    execute_iceberg,
    # StreamGuard — incremental NDJSON validation
    execute_stream_ndjson,
    execute_streaming,
    list_iceberg_snapshots,
    # Utilities
    validate_dsl,
)

# Custom Python validators
from ._validators import (
    MergedReport,
    clear_validators,
    list_validators,
    merge_violations,
    run_custom_validators,
    validator,
)

# Shared, dependency-light MCP/DAB connector base used by the PyStream* family.
# Exported here so consumers do not have to import a private module.
from ._mcp_connector import BaseMCPConnector

__all__ = [
    "AdaptiveThresholdDetector",
    # ML Anomaly Detection (v1.1.0+)
    "AnomalyDetectionEngine",
    "AnomalyResult",
    # Shared MCP/DAB connector base
    "BaseMCPConnector",
    # Core
    "DataContract",
    "FileResult",
    "IntegrityViolation",
    "LineageChange",
    "LineageEdge",
    "LineageGraph",
    # Data lineage tracking
    "LineageNode",
    "LineageVersion",
    "MergedReport",
    "PiiFinding",
    "SchemaChange",
    "SimpleMovingAverageDetector",
    "StreamReport",
    "ValidationReport",
    "__version__",
    "assert_no_breaking_changes",
    "check_all_foreign_keys",
    # Referential integrity
    "check_referential_integrity",
    "clear_validators",
    "compare_delta_versions",
    "create_lineage_edge",
    "create_lineage_node",
    # Schema evolution
    "detect_schema_changes",
    "execute",
    "execute_cloud",
    # GPU / cuDF
    "execute_cudf",
    # Lakehouse
    "execute_delta",
    "execute_file",
    # Parallel multi-file
    "execute_files",
    "execute_files_stream",
    "execute_iceberg",
    "execute_spark",
    # Cloud + SQL + Spark
    "execute_sql",
    # StreamGuard
    "execute_stream_ndjson",
    "execute_streaming",
    "get_lineage_graph",
    "get_lineage_history",
    "get_lineage_version",
    "integrity_report",
    "is_cudf_available",
    "list_iceberg_snapshots",
    "list_validators",
    "merge_violations",
    "pii_report",
    "run_custom_validators",
    "save_lineage_version",
    # PII detection
    "scan_pii",
    "schema_evolution_report",
    # HTML report
    "to_html",
    # Utilities
    "validate_dsl",
    # Custom validators
    "validator",
]
