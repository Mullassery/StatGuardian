{% macro create_run_results_table() %}
  {% set relation = statguardian.get_run_results_relation() %}
  {% set ddl %}
    create table if not exists {{ relation }} (
      invocation_id     {{ dbt.type_string() }},
      unique_id         {{ dbt.type_string() }},
      model_name        {{ dbt.type_string() }},
      resource_type     {{ dbt.type_string() }},
      status             {{ dbt.type_string() }},
      execution_time     {{ dbt.type_float() }},
      thread_id          {{ dbt.type_string() }},
      rows_affected      {{ dbt.type_bigint() }},
      message            {{ dbt.type_string() }},
      generated_at       {{ dbt.type_timestamp() }}
    )
  {% endset %}
  {% do run_query(ddl) %}
{% endmacro %}


{% macro insert_run_results(results) %}
  {% set relation = statguardian.get_run_results_relation() %}
  {% for result in results %}
    {% if result.node.resource_type in ('model', 'seed', 'snapshot') %}
      {% set rows_affected = result.adapter_response.get('rows_affected') if result.adapter_response else none %}
      {% set message = (result.message or '') | replace("'", "''") %}
      {% set insert_sql %}
        insert into {{ relation }}
          (invocation_id, unique_id, model_name, resource_type, status,
           execution_time, thread_id, rows_affected, message, generated_at)
        values (
          '{{ invocation_id }}',
          '{{ result.node.unique_id }}',
          '{{ result.node.name }}',
          '{{ result.node.resource_type }}',
          '{{ result.status }}',
          {{ result.execution_time }},
          '{{ result.thread_id }}',
          {{ rows_affected if rows_affected is not none else 'null' }},
          '{{ message[:2000] }}',
          {{ dbt.current_timestamp() }}
        )
      {% endset %}
      {% do run_query(insert_sql) %}
    {% endif %}
  {% endfor %}
{% endmacro %}
