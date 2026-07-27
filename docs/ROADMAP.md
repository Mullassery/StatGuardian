# StatGuardian Roadmap

**Current Version:** v2.1.0

## Vision

StatGuardian provides Rust-native data quality, drift detection, and anomaly detection with Python-first API for enterprise data pipelines.

## Completed Milestones

✅ **v1.0** — Foundation
- Schema validation (Pandera-like DSL)
- Data expectations & rules engine
- Statistical drift detection
- Anomaly detection algorithms

✅ **v2.0** — Advanced Features
- Cross-column conditional assertions
- PII detection & masking
- Schema evolution tracking
- HTML report generation

✅ **v2.1 (July 2026)** — Workflow Integration
- CLI: `statguardian validate`, `detect-drift`, `detect-anomalies`, `check-schema`
- REST API (Port 8008) for automation
- n8n, Power Automate, Temporal, Airflow integration
- Quality gate automation

## In Progress

⏳ **v2.2 (Aug 2026)** — ML-Powered Detection
- Machine learning anomaly detection
- Drift prediction models
- Seasonal adjustment for time series
- Adaptive thresholds

## Planned

📅 **v3.0 (Sep 2026)** — Distributed Validation
- Streaming validation for big data
- Parallel processing optimization
- Delta Lake & Iceberg integration
- Incremental validation

📅 **v3.5 (Oct 2026)** — Governance
- Data lineage tracking
- Compliance reporting (GDPR, SOX, HIPAA)
- Access control & audit logging
- Data quality SLA monitoring

📅 **v4.0 (Q4 2026)** — Intelligence Layer
- Data profiling ML model
- Quality score prediction
- Automatic remediation
- Enterprise integrations

## Integration Points

- **Data Platforms:** Snowflake, BigQuery, Redshift, PostgreSQL, Delta, Iceberg
- **Workflow Tools:** n8n, Power Automate, Temporal, Airflow, UiPath
- **Frameworks:** Pandas, Polars, PySpark, DuckDB

## Priority Features

1. **ML Anomaly Detection** (Q3 2026) — Advanced pattern recognition
2. **Streaming Validation** (Q3 2026) — Real-time data quality
3. **Compliance Reporting** (Q4 2026) — Regulatory compliance
4. **Governance Dashboard** (Q4 2026) — Enterprise monitoring

## Known Limitations

- DSL parsing limited to single tables (cross-database coming v3.0)
- Large dataset processing requires tuning
- ML models need 1000+ historical records

## Community

Contribute:
https://github.com/Mullassery/StatGuardian/issues
