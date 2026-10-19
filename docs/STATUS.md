# Project status

**Version:** 2.8.0 · **Updated:** 2026-10-19

## Working today

- Declarative `.sg` contracts: `schema`, `quality`, `stats` (drift),
  `anomalies`, and optional `stream` blocks; `@blocking`/`@warning` severity.
- Rust-native validation/statistics with a Python API.
- DataFrame inputs: Polars (no copy), Pandas (extra), PyArrow, Narwhals.
- Data formats: Parquet, CSV, JSON, Avro, ORC, Arrow IPC, Delta Lake, Iceberg
  (Delta/Iceberg native, no Spark).
- Drift detection via `--reference`; anomaly rules (outliers, duplicates,
  cardinality explosion).
- Structured reports: summary, JSON, Prometheus.
- HTTP service (`serve`) with `/validate`; `/detect-changes` returns 501.
- dbt and Airflow integrations in `integrations/`.

## Verified gaps

| Gap | Impact | Status |
|---|---|---|
| PyPI 2.8.0 publishes only a macOS arm64 wheel | Linux/Windows/Intel users must build from source | Open |
| GitHub releases stop at v2.5.0 (tags at v2.6.0) | No release feed for 2.6–2.8 | Open |
| PyPI license metadata is `None` | License scanners/trust signals | Open |
| No publish workflow | Releases are manual | Open |
| CI tests Python 3.10–3.12 only | 3.9 declared but untested in CI | Open |
| Classifier says "Production/Stable" while the README says active development | Expectation mismatch | Reconcile |
| No docs portal / Data Docs | Discovery/onboarding | Mitigated by [docs index](README.md) |
| Dashboards/alerting | Out of scope | By design |

## Verify

```bash
statguardian --version
statguardian check examples/basic_validation.sg
statguardian validate --contract examples/basic_validation.sg --file data.parquet
```

See [Contract DSL](contract-dsl.md), [Comparison](COMPARISON.md), and
[`docs/ROADMAP_HONEST.md`](ROADMAP_HONEST.md).
