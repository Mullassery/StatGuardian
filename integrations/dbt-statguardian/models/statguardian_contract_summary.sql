{{ config(materialized='view') }}

with ranked as (
    select
        *,
        row_number() over (
            partition by unique_id order by validated_at desc
        ) as rn
    from {{ ref('stg_statguardian__contract_validations') }}
),

latest_run as (
    select
        unique_id,
        max(generated_at) as last_run_at
    from {{ ref('stg_statguardian__run_results') }}
    group by unique_id
)

select
    ranked.unique_id,
    ranked.model_name,
    ranked.relation_name,
    ranked.contract_path,
    ranked.passed,
    ranked.health_score,
    ranked.grade,
    ranked.violation_count,
    ranked.validated_at,
    latest_run.last_run_at as model_last_built_at
from ranked
left join latest_run on latest_run.unique_id = ranked.unique_id
where ranked.rn = 1
