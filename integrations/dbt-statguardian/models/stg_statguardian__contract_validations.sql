{{ config(materialized='view') }}

select
    unique_id,
    model_name,
    relation_name,
    contract_path,
    passed,
    health_score,
    grade,
    violation_count,
    report_json,
    validated_at
from {{ statguardian.get_contract_validations_relation() }}
