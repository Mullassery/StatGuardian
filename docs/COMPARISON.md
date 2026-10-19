# Comparison

StatGuardian validates data at runtime against a declarative `.sg` contract. The
tools below overlap on schema/statistical validation; they differ in interface
and deployment shape.

Sources verified on **2026-10-19**:

- Great Expectations (GX Core) — <https://greatexpectations.io/>
- Pandera — <https://github.com/unionai-oss/pandera>

Not assessed in this pass: Soda, Amazon Deequ, Monte Carlo, Elementary. This is
not a benchmark.

## Pick by problem

| Your problem | Start with | Why |
|---|---|---|
| One compact contract for schema + drift + anomalies, executed in Rust | **StatGuardian** | `.sg` DSL, Rust core, blocking severity |
| Native Delta Lake / Iceberg validation without Spark | **StatGuardian** | Native metadata parsing |
| Rich expectation library, Data Docs, orchestrator actions | Great Expectations | Mature framework and ecosystem |
| Pythonic dataframe schemas in pandas/polars/pyspark | Pandera | `DataFrameSchema` / `DataFrameModel` |
| HTTP validation endpoint | **StatGuardian** (`serve`) | `/validate` service |
| Dashboards/alerting platform | neither | Out of scope for StatGuardian |

## Matrix

| Capability | StatGuardian | Great Expectations | Pandera |
|---|---|---|---|
| Declarative contract DSL | ✅ `.sg` | Python expectations | Python schemas |
| Rust-native execution | ✅ | — | — |
| Pandas + Polars + PyArrow/Narwhals | ✅ | ✅ | ✅ |
| Drift (PSI/KS) via contract | ✅ | expectations/plugins | statistical checks |
| Anomaly rules | ✅ outliers/duplicates/cardinality | expectations | custom checks |
| Blocking/warning severity | ✅ `@blocking`/`@warning` | actions | schema-level |
| Native Delta/Iceberg (no Spark) | ✅ | connectors | — |
| HTTP validation service | ✅ | via deployment | — |
| Data Docs / UI | — | ✅ | — |
| License | Apache-2.0 | Apache-2.0 | MIT |

Legend: ✅ stated in reviewed official docs; — not stated (not proof of absence).

## Differentiation

StatGuardian's niche is a **small contract language** with **Rust execution**,
native lakehouse inputs, and severity-gated blocking, returning structured
reports. Great Expectations is broader as a framework with Data Docs and
actions; Pandera is the lighter Python-schema option.

## What StatGuardian does not do

- Dashboards/alerting infrastructure.
- A general-purpose test framework.
- Guaranteed cross-dataset performance; benchmark your workload.
