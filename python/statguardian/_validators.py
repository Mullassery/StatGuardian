"""
Custom Python validator plugin system for StatGuard.

Register validation functions that run alongside the Rust engine
for checks that require arbitrary Python logic.
"""

from __future__ import annotations

from typing import Any, Callable, Dict, List, Optional, Tuple

# Global registry: column_name (or "*" for all columns) → list of entries
_REGISTRY: Dict[str, List[dict]] = {}


def validator(
    column: str,
    *,
    name: str = None,
    severity: str = "error",
):
    """
    Decorator that registers a custom validation function for a column.

    The decorated function receives a list of column values and must return
    either:
    - ``(failing_row_indices, message)`` — a tuple of failing indices and a
      descriptive message, or
    - ``None`` — to indicate the check passed.

    Args:
        column:   Column name to validate, or ``"*"`` to run against every column.
        name:     Check name used in violation output (defaults to function name).
        severity: Severity level: ``"info"``, ``"warning"``, ``"error"``,
                  or ``"blocking"``.

    Example::

        @statguardian.validator("amount")
        def no_round_numbers(values):
            failing = [i for i, v in enumerate(values)
                       if v is not None and v == int(v) and v > 1000]
            if failing:
                return failing, f"{len(failing)} suspiciously round large amount(s)"

        @statguardian.validator("email")
        def no_example_domains(values):
            bad = [i for i, v in enumerate(values)
                   if v and "example.com" in v]
            return bad, f"{len(bad)} example.com email(s)" if bad else None
    """
    def decorator(fn: Callable) -> Callable:
        entry = {
            "fn": fn,
            "name": name or fn.__name__,
            "severity": severity,
        }
        _REGISTRY.setdefault(column, []).append(entry)
        return fn
    return decorator


def run_custom_validators(df, *, raise_on_error: bool = False) -> List[dict]:
    """
    Run all registered custom validators against a Polars DataFrame.

    Returns a list of violation dicts compatible with StatGuard's report
    format.  Pass the result to ``statguardian.merge_violations(report, extra)``
    to include them in an existing ValidationReport.

    Args:
        df:             Polars DataFrame to validate.
        raise_on_error: If True, raise ``RuntimeError`` when a validator
                        raises an exception instead of recording it as a
                        violation.
    """
    violations = []

    for column, validators in _REGISTRY.items():
        cols = df.columns if column == "*" else [column]

        for col_name in cols:
            if col_name not in df.columns:
                continue

            values = df[col_name].to_list()

            for v in validators:
                try:
                    result = v["fn"](values)
                    if result is None:
                        continue
                    if isinstance(result, (list, tuple)) and len(result) == 2:
                        failing_rows, message = result
                    else:
                        failing_rows, message = [], str(result)

                    if failing_rows:
                        violations.append({
                            "column": col_name,
                            "check": v["name"],
                            "severity": v["severity"],
                            "message": message,
                            "row_indices": list(failing_rows),
                            "observed": None,
                            "expected": None,
                        })
                except Exception as exc:
                    if raise_on_error:
                        raise RuntimeError(
                            f"Custom validator '{v['name']}' on column '{col_name}' raised: {exc}"
                        ) from exc
                    violations.append({
                        "column": col_name,
                        "check": v["name"],
                        "severity": "error",
                        "message": f"validator raised: {exc}",
                        "row_indices": [],
                        "observed": None,
                        "expected": None,
                    })

    return violations


def clear_validators(column: str = None) -> None:
    """
    Remove registered validators.

    Args:
        column: If given, remove only validators for that column.
                If None, clear all registered validators.
    """
    global _REGISTRY
    if column is None:
        _REGISTRY.clear()
    else:
        _REGISTRY.pop(column, None)


def list_validators() -> Dict[str, List[str]]:
    """Return a summary of all registered validators as {column: [names]}."""
    return {col: [v["name"] for v in vs] for col, vs in _REGISTRY.items()}
