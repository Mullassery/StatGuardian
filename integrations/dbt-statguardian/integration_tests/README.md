# statguardian dbt package — integration tests

A minimal dbt project (DuckDB target, no external warehouse needed) that
exercises the package end to end.

```bash
cd integration_tests
pip install "statguardian[dbt]" dbt-duckdb
cp profiles.yml.example ~/.dbt/profiles.yml   # or export DBT_PROFILES_DIR=.
dbt deps
dbt build                     # runs on-run-end, creates statguardian_test.* tables
statguardian dbt validate --project-dir . --write-results
dbt test                      # statguardian.contract_passed now has a row to check
```

`orders.sql` seeds three in-memory rows against `contracts/orders.sg`; all
three pass the contract, so both `statguardian dbt validate` and `dbt test`
should exit 0.
