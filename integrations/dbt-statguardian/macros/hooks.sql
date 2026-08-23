{#
  Wire both hooks into the consuming project's dbt_project.yml:

    on-run-start:
      - "{{ statguardian.on_run_start() }}"
    on-run-end:
      - "{{ statguardian.on_run_end() }}"

  Table creation has to happen in on-run-start, not on-run-end: this
  package's own reporting models (stg_statguardian__*) and the generic
  tests (statguardian.contract_passed) select from these tables as part
  of the same `dbt build` DAG, which runs *before* on-run-end fires. If
  table creation only happened in on-run-end, the very first `dbt build`
  in a fresh warehouse would error out — those models/tests would compile
  against tables that don't exist yet on that first pass.

  on-run-end then only inserts this invocation's results, once `results`
  is populated — mirroring what elementary's on-run-end hook does for its
  own tables, but split across the two hooks so it works from run one.
#}
{% macro on_run_start() %}
  {% if execute %}
    {% do statguardian.create_statguardian_schema() %}
    {% do statguardian.create_run_results_table() %}
    {% do statguardian.create_test_results_table() %}
    {% do statguardian.create_contract_validations_table() %}
  {% endif %}
{% endmacro %}


{% macro on_run_end() %}
  {% if execute %}
    {% do statguardian.insert_run_results(results) %}
    {% do statguardian.insert_test_results(results) %}
  {% endif %}
{% endmacro %}


{% macro create_statguardian_schema() %}
  {% set schema_name = statguardian.get_schema() %}
  {% set ddl %}
    create schema if not exists {{ schema_name }}
  {% endset %}
  {% do run_query(ddl) %}
{% endmacro %}
