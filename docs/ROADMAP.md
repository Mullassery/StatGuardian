# StatGuardian Roadmap

> **This document contains unverified aspirational claims.** Auditing this
> file against the actual codebase (2026-09) found that the "v2.1 — Workflow
> Integration" section below marks a REST API and n8n/Power Automate/
> Temporal/Airflow integrations as **shipped (✅) when no such code exists
> anywhere in this repository** — corrected inline below. For an audited,
> honest status of what is actually built and tested, see
> [`docs/ROADMAP_HONEST.md`](ROADMAP_HONEST.md) and
> [`docs/SECURITY_AUDIT.md`](SECURITY_AUDIT.md). Everything else in this file
> beyond that one section is unverified forward-looking planning, not a
> record of what's shipped.

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

**v2.1 — Workflow Integration (status corrected 2026-09, was previously marked ✅ complete)**
- ✅ CLI: `statguardian validate`, `dbt validate`, `check` (confirmed in `python/statguardian/_cli.py`)
- ❌ **REST API (Port 8008) — does not exist.** No HTTP server, no web framework
  dependency, no port-8008 reference anywhere in this codebase. This was
  fabricated in a prior version of this document.
- ❌ **n8n, Power Automate, Temporal, Airflow, UiPath integration — do not
  exist.** No code, no dependency, no documentation for any of these tools
  anywhere in this repository. Also fabricated.
- Quality gate automation exists only in the form of the CLI's process exit
  code and the dbt integration (`integrations/dbt-statguardian`) — there is
  no separate "quality gate" feature beyond that.

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
- Compliance reporting (GDPR, SOX, HIPAA)
- Access control & audit logging
- Data quality SLA monitoring

Note: data lineage tracking (versioned lineage graphs, impact-chain analysis,
change history) shipped ahead of schedule — see `statguardian.get_lineage_graph`,
`save_lineage_version`, `get_lineage_version`, `get_lineage_history`.

📅 **v4.0 (Q4 2026)** — Intelligence Layer
- Data profiling ML model
- Quality score prediction
- Automatic remediation
- Enterprise integrations

## Integration Points

Aspirational — none of the items in this section are implemented today
except where noted.

- **Data Platforms:** Snowflake, BigQuery, Redshift, PostgreSQL (via `execute_sql`,
  implemented), Delta and Iceberg (implemented, native table readers)
- **Workflow Tools:** n8n, Power Automate, Temporal, Airflow, UiPath — none
  implemented; not started
- **Frameworks:** Pandas, Polars (both implemented), PySpark (`execute_spark`,
  implemented), DuckDB (implemented, via `execute_sql()`'s `sql-duckdb` extra)

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
https://github.com/Mullassery/statguardian/issues
