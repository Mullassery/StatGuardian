# Audit findings — 2026-10-19

This file records the repository-specific findings from the portfolio audit
conducted on 2026-10-19. It is documentation only; no code or release artifacts
were changed in this pass unless explicitly noted under "Implementation status".

Sources: the repository's own README/source/workflows, GitHub metadata and
Actions runs, PyPI metadata, and official competitor documentation. Claims are
marked as verified, inferred, or open questions. "Absent" means not found in the
inspected repository/docs, not proof that it cannot exist.

## What it is

Declarative data-quality engine with a `.sg` DSL: schema validation, drift (PSI/KS), anomaly detection, Pandas/Polars/Delta/Iceberg, lineage, PII, and dbt integration.

## Verified strengths

- Rust engine; `.sg` DSL; Pandas/Polars; Delta/Iceberg; drift/anomaly; blocking severities; structured reports; streaming crate; lineage/dbt/PII modules; 9 test files.
- Rust speed + one dependency + DataFrame/lakehouse support is a credible wedge.

## Verified gaps and user-facing risks

- macOS arm64-only wheel; no Airflow integration (docs previously claimed it); no dashboards/alerting.
- No committed benchmark vs GX/Soda/Pandera; `.sg` is a new DSL with adoption cost; streaming has a bounded-memory regression.
- Three overlapping roadmap docs, two with fabricated claims (now disclaimed).

## Direct competitors

- Great Expectations (mature, Data Docs, integrations), Soda Core (50+ checks, warehouses, Airflow/Dagster/Prefect), Pandera, Deequ.

## Ranked next actions

- P1: publish Linux/Windows wheels (S/M).
- P1: Airflow integration + CI recipe (M).
- P2: reproducible benchmark vs GX/Soda/Pandera (M).
- P2: consolidate roadmap docs (S).

## Do not build

- Dashboards/alerting platform; integrate with orchestrators/observability.

## Evidence gaps / validation needed

- No performance benchmark; no migration path from GX/Soda contracts.

## Recommended disposition

Invest; integrations and honest docs are the adoption gates.

## Implementation status

Implemented the missing Apache Airflow integration in this pass:

- New `integrations/airflow-statguardian` package with
  `StatGuardianValidateOperator` (templated contract/data/reference paths,
  optional `fail_on_warning`).
- Calls the public `statguardian.execute_file()` API directly; raises
  `AirflowException` on failure.
- Unit tests cover the pass/fail decision without requiring Airflow.
- Airflow itself is not installed in this environment, so the operator's
  DAG-level behavior is not integration-tested.

Still open: publishing wheels, wiring the PyReverseETL quality gate, and
n8n/Power Automate/Temporal/UiPath integrations.
