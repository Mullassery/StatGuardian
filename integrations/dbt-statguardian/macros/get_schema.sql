{% macro get_schema() %}
  {{ return(var('statguardian_schema', target.schema ~ '_statguardian')) }}
{% endmacro %}

{% macro get_run_results_relation() %}
  {{ return(statguardian.get_schema() ~ '.dbt_run_results') }}
{% endmacro %}

{% macro get_test_results_relation() %}
  {{ return(statguardian.get_schema() ~ '.dbt_test_results') }}
{% endmacro %}

{% macro get_contract_validations_relation() %}
  {{ return(statguardian.get_schema() ~ '.contract_validations') }}
{% endmacro %}
