{#
  This table is written by the companion CLI (`statguardian dbt validate
  --write-results`), not by dbt itself — dbt/Jinja has no way to invoke the
  Rust/Python StatGuard engine directly. The table is declared here so the
  dbt package can create it up front (on-run-end, before the CLI ever runs)
  and so the generic test / reporting models below have a stable contract
  for its shape.
#}
{% macro create_contract_validations_table() %}
  {% set relation = statguardian.get_contract_validations_relation() %}
  {% set ddl %}
    create table if not exists {{ relation }} (
      unique_id         {{ dbt.type_string() }},
      model_name        {{ dbt.type_string() }},
      relation_name     {{ dbt.type_string() }},
      contract_path     {{ dbt.type_string() }},
      passed            {{ dbt.type_boolean() }},
      health_score      {{ dbt.type_float() }},
      grade             {{ dbt.type_string() }},
      violation_count   {{ dbt.type_bigint() }},
      report_json       {{ dbt.type_string() }},
      validated_at       {{ dbt.type_timestamp() }}
    )
  {% endset %}
  {% do run_query(ddl) %}
{% endmacro %}
