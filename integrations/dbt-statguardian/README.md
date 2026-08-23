# statguardian dbt package

A dbt package for [StatGuardian](https://github.com/Mullassery/statguardian),
in the spirit of [elementary](https://github.com/elementary-data/dbt-data-reliability):
the dbt package captures run/test metadata and exposes results as native dbt
tests; a companion CLI (`statguardian dbt validate`) does the part dbt itself
can't — running StatGuard's `.sg` contracts against your models' actual
warehouse tables, since dbt/Jinja has no way to invoke the StatGuard engine.

## What's here

- **`on-run-end` capture** — logs every model/seed/snapshot and test result
  from each `dbt build`/`dbt run`/`dbt test` into
  `<schema>_statguardian.dbt_run_results` / `dbt_test_results`.
- **`statguardian.contract_passed`** — a generic dbt test that fails if the
  most recent `statguardian dbt validate` run recorded a failing contract
  for that model.
- **`statguardian.contract_recently_validated`** — a generic dbt test that
  fails if no contract validation has run recently (catches a CI step that
  silently stopped running).
- **Reporting models** (`stg_statguardian__*`, `statguardian_contract_summary`)
  — plain SQL views over the captured tables, selectable like any other dbt
  model.

## Install

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

Both hooks are required: `on-run-start` creates the package's tables so its
own reporting models and generic tests have something to compile against
in the *same* `dbt build`; `on-run-end` inserts that run's results once
they're known.

## Attach a contract to a model

```yaml
# models/schema.yml
models:
  - name: orders
    config:
      meta:
        statguardian_contract: "contracts/orders.sg"   # path relative to project root
    tests:
      - statguardian.contract_passed
```

## Run it

```bash
pip install "statguardian[dbt]"

dbt build
statguardian dbt validate --project-dir . --write-results
dbt test
```

`statguardian dbt validate`:
1. reads `target/manifest.json` for models with `meta.statguardian_contract`
2. resolves each model's warehouse relation and connection from `profiles.yml`
3. runs `statguardian.execute_sql()` against the materialized table
4. prints a pass/fail summary and writes `target/statguardian_results.json`
5. with `--write-results`, also inserts into
   `<schema>_statguardian.contract_validations` so `statguardian.contract_passed`
   has something to check on the next `dbt test`
6. exits non-zero if any contract failed — drop it into CI right after `dbt build`

Supported profile adapter types today: `postgres`, `redshift`, `snowflake`,
`bigquery`, `duckdb` (same open-source-only connector set as
`statguardian.execute_sql`). Other adapters: run validation manually via the
Python API.

## Config vars

| Var | Default | Purpose |
|---|---|---|
| `statguardian_schema` | `{{ target.schema }}_statguardian` | Schema the package's tables live in |

```yaml
vars:
  statguardian_schema: "data_quality"
```

See `integration_tests/` for a runnable end-to-end example against DuckDB.
