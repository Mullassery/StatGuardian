# Queue status — 2026-10-19

This file records what was shipped, what is blocked/failed, and what is rejected for this repository. It complements `AUDIT_FINDINGS.md`.

## Shipped
- Airflow operator; local HTTP `/validate` service; Docker Compose deployment.
- Pandas/PyArrow/Narwhals dataframe compatibility in `execute()` (Polars stays copy-free).

## Blocked / failed
- Runtime Docker validation (daemon unavailable here).
- Streaming bounded-memory regression; reproducible benchmarks.

## Rejected / deferred
- Dashboards/alerting platform.

## Policy

Any future feature or upgrade must check `AUDIT_FINDINGS.md` and this file before implementation. A blocked item is retried only when its external dependency is available; a rejected item needs new user evidence or a design change to reopen.
