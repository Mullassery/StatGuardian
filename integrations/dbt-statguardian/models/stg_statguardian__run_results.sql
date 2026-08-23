{{ config(materialized='view') }}

select
    invocation_id,
    unique_id,
    model_name,
    resource_type,
    status,
    execution_time,
    thread_id,
    rows_affected,
    message,
    generated_at
from {{ statguardian.get_run_results_relation() }}
