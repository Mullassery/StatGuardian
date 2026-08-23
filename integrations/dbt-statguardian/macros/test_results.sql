{% macro create_test_results_table() %}
  {% set relation = statguardian.get_test_results_relation() %}
  {% set ddl %}
    create table if not exists {{ relation }} (
      invocation_id     {{ dbt.type_string() }},
      unique_id         {{ dbt.type_string() }},
      test_name         {{ dbt.type_string() }},
      test_type         {{ dbt.type_string() }},
      model_unique_id   {{ dbt.type_string() }},
      column_name       {{ dbt.type_string() }},
      status            {{ dbt.type_string() }},
      failures          {{ dbt.type_bigint() }},
      execution_time    {{ dbt.type_float() }},
      message           {{ dbt.type_string() }},
      generated_at      {{ dbt.type_timestamp() }}
    )
  {% endset %}
  {% do run_query(ddl) %}
{% endmacro %}


{% macro insert_test_results(results) %}
  {% set relation = statguardian.get_test_results_relation() %}
  {% for result in results %}
    {% if result.node.resource_type == 'test' %}
      {% set node = result.node %}
      {% set is_generic = node.test_metadata is defined and node.test_metadata %}
      {% set test_type = node.test_metadata.name if is_generic else 'singular' %}
      {% set model_unique_id = node.depends_on.nodes[0] if node.depends_on and node.depends_on.nodes else '' %}
      {% set column_name = node.column_name if node.column_name is defined and node.column_name else '' %}
      {% set failures = result.failures if result.failures is not none else 0 %}
      {% set message = (result.message or '') | replace("'", "''") %}
      {% set insert_sql %}
        insert into {{ relation }}
          (invocation_id, unique_id, test_name, test_type, model_unique_id,
           column_name, status, failures, execution_time, message, generated_at)
        values (
          '{{ invocation_id }}',
          '{{ node.unique_id }}',
          '{{ node.name }}',
          '{{ test_type }}',
          '{{ model_unique_id }}',
          '{{ column_name }}',
          '{{ result.status }}',
          {{ failures }},
          {{ result.execution_time }},
          '{{ message[:2000] }}',
          {{ dbt.current_timestamp() }}
        )
      {% endset %}
      {% do run_query(insert_sql) %}
    {% endif %}
  {% endfor %}
{% endmacro %}
