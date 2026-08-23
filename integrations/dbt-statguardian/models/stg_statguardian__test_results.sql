{{ config(materialized='view') }}

select
    invocation_id,
    unique_id,
    test_name,
    test_type,
    model_unique_id,
    column_name,
    status,
    failures,
    execution_time,
    message,
    generated_at
from {{ statguardian.get_test_results_relation() }}
