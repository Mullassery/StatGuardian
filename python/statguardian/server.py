"""Minimal HTTP service for StatGuardian contract validation.

This is the service shape PyReverseETL's ``StatGuardianClient`` expects
(``/validate`` returning passed/quality_score/issues/schema_version). It is
intentionally small and contract-scoped: one contract per process, configured
at startup. It is not a multi-tenant control plane.
"""

from __future__ import annotations

from typing import Any, Callable

try:
    from fastapi import FastAPI, HTTPException
    from pydantic import BaseModel
except ImportError as exc:  # pragma: no cover - optional extra
    raise ImportError(
        'StatGuardian HTTP service requires FastAPI; install with '
        '`pip install "statguardian[api]"`'
    ) from exc


class EntityPayload(BaseModel):
    entity: dict[str, Any]
    contract_id: str = "default"
    strict: bool = True


class ValidateResponse(BaseModel):
    passed: bool
    quality_score: float
    issues: list[str]
    schema_version: str


class DetectChangesPayload(BaseModel):
    current: dict[str, Any]
    reference: dict[str, Any]
    added_severity: str = "info"
    removed_severity: str = "error"
    retyped_severity: str = "warning"


def _load_contract(path: str):
    from statguardian import DataContract

    return DataContract.from_file(path)


def _default_execute(contract, frame):
    from statguardian import execute

    return execute(contract, frame)


def create_app(
    contract_path: str | None = None,
    *,
    contract: Any = None,
    execute_fn: Callable[[Any, Any], Any] | None = None,
) -> FastAPI:
    """Build the service. ``contract``/``execute_fn`` are injectable for tests."""
    if contract is None and contract_path:
        contract = _load_contract(contract_path)
    runner = execute_fn or _default_execute
    schema_version = contract_path or "unbound"

    app = FastAPI(title="StatGuardian", version="1")

    @app.get("/healthz")
    def healthz() -> dict[str, str]:
        return {"status": "ok", "contract": "bound" if contract is not None else "unbound"}

    @app.post("/validate", response_model=ValidateResponse)
    def validate(payload: EntityPayload) -> ValidateResponse:
        if contract is None:
            raise HTTPException(status_code=503, detail="no contract configured")
        attributes = payload.entity.get("attributes")
        if not isinstance(attributes, dict):
            raise HTTPException(status_code=400, detail="entity.attributes must be an object")
        try:
            import pandas as pd

            report = runner(contract, pd.DataFrame([attributes]))
        except Exception as exc:  # surface a real validation/engine error
            raise HTTPException(status_code=400, detail=str(exc)) from exc

        passed = bool(getattr(report, "passed", False))
        summary = getattr(report, "summary", lambda: "")()
        return ValidateResponse(
            passed=passed,
            quality_score=1.0 if passed else 0.0,
            issues=[] if passed else [str(summary or "validation failed")],
            schema_version=schema_version,
        )

    @app.post("/detect-changes")
    def detect_changes(payload: DetectChangesPayload) -> dict[str, Any]:
        from statguardian._evolution import detect_schema_changes, schema_evolution_report

        allowed = {"info", "warning", "error"}
        for name, value in (
            ("added_severity", payload.added_severity),
            ("removed_severity", payload.removed_severity),
            ("retyped_severity", payload.retyped_severity),
        ):
            if value not in allowed:
                raise HTTPException(
                    status_code=400,
                    detail=f"{name} must be one of {sorted(allowed)}",
                )
        try:
            changes = detect_schema_changes(
                payload.current,
                payload.reference,
                added_severity=payload.added_severity,
                removed_severity=payload.removed_severity,
                retyped_severity=payload.retyped_severity,
            )
        except Exception as exc:
            raise HTTPException(status_code=400, detail=str(exc)) from exc

        serialized = [
            {
                "change_type": change.change_type,
                "column": change.column,
                "old_dtype": change.old_dtype,
                "new_dtype": change.new_dtype,
                "severity": change.severity,
            }
            for change in changes
        ]
        return {
            "changes": serialized,
            "breaking": any(change.severity == "error" for change in changes),
            "report": schema_evolution_report(changes),
        }

    return app
