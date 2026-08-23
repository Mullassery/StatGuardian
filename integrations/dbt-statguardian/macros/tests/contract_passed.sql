{#
  Usage, in a model's schema.yml:

    models:
      - name: orders
        tests:
          - statguardian.contract_passed

  Fails the dbt test if the most recent `statguardian dbt validate` run
  recorded a failing contract for this model. Run `statguardian dbt
  validate --write-results` before `dbt test` so there's a row to check —
  if none exists yet, the test is a no-op pass rather than a hard failure,
  so first-time setup doesn't break `dbt build`.
#}
{#
  `model` here is the compiled Relation for the node under test, not the
  manifest node — it has no `.unique_id`. statguardian dbt validate writes
  relation_name as "{database}.{schema}.{alias}" (see relation_for_node()
  in python/statguardian/_dbt.py), so that's what this joins on instead.
#}
{% test contract_passed(model) %}
  {% set relation = statguardian.get_contract_validations_relation() %}
  {% set relation_name = model.database ~ '.' ~ model.schema ~ '.' ~ model.identifier %}
  with latest as (
    select relation_name, passed, validated_at,
           row_number() over (partition by relation_name order by validated_at desc) as rn
    from {{ relation }}
    where relation_name = '{{ relation_name }}'
  )
  select *
  from latest
  where rn = 1
    and passed = false
{% endtest %}
