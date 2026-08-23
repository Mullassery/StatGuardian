# dbt Integration

StatGuardian ships a dbt package (`integrations/dbt-statguardian/`) plus a
`statguardian dbt` CLI subcommand. Together they let you attach a `.sg`
contract to a dbt model and gate `dbt build` on it — the dbt package half
handles metadata capture and exposes results as a normal dbt test; the CLI
half is what actually executes the contract, since dbt/Jinja has no way to
call into the StatGuard engine itself.

This mirrors how [elementary](https://github.com/elementary-data/dbt-data-reliability)
splits its dbt package (capture + SQL-native tests) from its `edr` CLI
(Python-side analysis/alerting).

---

## Install

```bash
pip install "statguardian[dbt]"
```

`packages.yml`:

```yaml
packages:
  - git: "https://github.com/Mullassery/statguardian.git"
    subdirectory: "integrations/dbt-statguardian"
    revision: main
```

`dbt_project.yml`:

```yaml
on-run-start:
  - "{{ statguardian.on_run_start() }}"
on-run-end:
  - "{{ statguardian.on_run_end() }}"
```

Both hooks matter: `on-run-start` creates the package's tables before the
model DAG runs, so the reporting views and `statguardian.contract_passed`
have something to compile against on the very first `dbt build` in a fresh
warehouse; `on-run-end` inserts that invocation's results once they exist.

`dbt deps` after adding the package.

## Attach a contract to a model

```yaml
# models/schema.yml
models:
  - name: orders
    config:
      meta:
        statguardian_contract: "contracts/orders.sg"
    tests:
      - statguardian.contract_passed
      - statguardian.contract_recently_validated:
          max_age_hours: 24
```

`statguardian_contract` is a path to a `.sg` DSL file, relative to the dbt
project root — see the main [README](../README.md#dsl-reference) or
`examples/*.sg` for contract syntax.

## Run

```bash
dbt build
statguardian dbt validate --project-dir . --write-results
dbt test
```

- `dbt build` materializes `orders` and (via the `on-run-end` hook) logs this
  invocation's run/test results into `<schema>_statguardian.dbt_run_results`
  / `dbt_test_results`.
- `statguardian dbt validate` reads `target/manifest.json`, finds `orders`
  (and any other model with `meta.statguardian_contract`), resolves the
  warehouse connection from `profiles.yml`, and runs
  `statguardian.execute_sql()` against the model's materialized table.
  `--write-results` inserts the outcome into
  `<schema>_statguardian.contract_validations`.
- `dbt test` runs `statguardian.contract_passed`, which fails if the latest
  row in `contract_validations` for `orders` has `passed = false`.

Exit codes: `statguardian dbt validate` exits `0` if every contract passed,
`1` if any failed (or, with `--fail-on-warning`, if any had violations),
`2` on a setup error (bad manifest, missing profile, unsupported adapter).
Put it right after `dbt build` in CI.

### CLI options

| Flag | Default | Purpose |
|---|---|---|
| `--project-dir` | `.` | dbt project root |
| `--profiles-dir` | `~/.dbt` | Directory containing `profiles.yml` |
| `--profile` | from `dbt_project.yml` | Profile name |
| `--target` | profile's default | Target name |
| `--write-results` | off | Persist results for `statguardian.contract_passed` to read |
| `--results-schema` | `<target schema>_statguardian` | Override the write-back schema |
| `--fail-on-warning` | off | Fail on any violation, not just contract failure |

### Supported adapters

`postgres`, `redshift`, `snowflake`, `bigquery`, `duckdb` — the same
open-source-only connector set as `statguardian.execute_sql()`. Other
adapters (Databricks, ClickHouse, Trino, ...) aren't wired into the
`profiles.yml` → connection-string resolver yet; validate those manually
with the Python API using the model's known `database.schema.alias`.

## Reporting models

The package ships plain SQL views over the captured tables:

- `stg_statguardian__run_results`, `stg_statguardian__test_results` — raw
  per-invocation dbt run/test outcomes.
- `stg_statguardian__contract_validations` — raw per-run contract outcomes
  (written by the CLI).
- `statguardian_contract_summary` — latest contract result per model,
  joined with that model's last dbt build time. Select from it like any
  other dbt model once the package is installed and `dbt run -s statguardian`
  has built it.

## Config vars

```yaml
vars:
  statguardian_schema: "data_quality"   # default: <target.schema>_statguardian
```

## End-to-end example

`integrations/dbt-statguardian/integration_tests/` is a runnable DuckDB
project exercising the whole flow — see its README.
