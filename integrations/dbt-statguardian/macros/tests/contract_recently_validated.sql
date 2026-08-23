{#
  Usage:

    models:
      - name: orders
        tests:
          - statguardian.contract_recently_validated:
              max_age_hours: 24

  Guards against a stale/never-run contract silently passing `dbt build`
  forever because `statguardian dbt validate` stopped being invoked in CI.
#}
{#
  `model` here is the compiled Relation for the node under test, not the
  manifest node — it has no `.unique_id`. Join on relation_name instead,
  same as statguardian.contract_passed.
#}
{% test contract_recently_validated(model, max_age_hours=24) %}
  {% set relation = statguardian.get_contract_validations_relation() %}
  {% set relation_name = model.database ~ '.' ~ model.schema ~ '.' ~ model.identifier %}
  with latest as (
    select relation_name, max(validated_at) as last_validated_at
    from {{ relation }}
    where relation_name = '{{ relation_name }}'
    group by relation_name
  )
  select *
  from latest
  where last_validated_at < {{ dbt.dateadd('hour', -1 * max_age_hours, dbt.current_timestamp()) }}
{% endtest %}
