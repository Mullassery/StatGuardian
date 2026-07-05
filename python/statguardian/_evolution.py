"""
Schema evolution detection for StatGuard.

Compares two Polars DataFrames (or two schema dicts) and surfaces
structural changes: added columns, removed columns, and type changes.
Use this to gate pipelines when upstream data producers make breaking changes.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional

try:
    import polars as pl
    _POLARS = True
except ImportError:
    _POLARS = False


@dataclass
class SchemaChange:
    """One structural change between two schema versions."""
    change_type: str            # "added" | "removed" | "retyped"
    column: str
    old_dtype: Optional[str] = None
    new_dtype: Optional[str] = None
    severity: str = "warning"   # "info" | "warning" | "error"

    def __str__(self) -> str:
        if self.change_type == "added":
            return f"[{self.severity.upper()}] Column added:   {self.column!r} ({self.new_dtype})"
        if self.change_type == "removed":
            return f"[{self.severity.upper()}] Column removed: {self.column!r} (was {self.old_dtype})"
        return (
            f"[{self.severity.upper()}] Column retyped: {self.column!r} "
            f"{self.old_dtype} → {self.new_dtype}"
        )


def detect_schema_changes(
    current: "pl.DataFrame | Dict[str, str]",
    reference: "pl.DataFrame | Dict[str, str]",
    added_severity: str = "info",
    removed_severity: str = "error",
    retyped_severity: str = "warning",
) -> List[SchemaChange]:
    """
    Compare two DataFrames (or raw schema dicts) and return all schema changes.

    Args:
        current:            The new / current DataFrame or {col: dtype} dict.
        reference:          The baseline DataFrame or {col: dtype} dict.
        added_severity:     Severity for newly added columns (default "info").
        removed_severity:   Severity for removed columns (default "error").
        retyped_severity:   Severity for retyped columns (default "warning").

    Returns:
        List of SchemaChange objects, empty if schemas are identical.

    Example::

        old_df = pl.read_parquet("yesterday.parquet")
        new_df = pl.read_parquet("today.parquet")

        changes = statguardian.detect_schema_changes(new_df, old_df)
        for c in changes:
            print(c)
    """
    if not _POLARS and not isinstance(current, dict):
        raise ImportError("polars is required unless you pass schema dicts directly")

    def to_schema(obj) -> Dict[str, str]:
        if isinstance(obj, dict):
            return {k: str(v) for k, v in obj.items()}
        return {col: str(dtype) for col, dtype in obj.schema.items()}

    old = to_schema(reference)
    new = to_schema(current)
    changes: List[SchemaChange] = []

    for col in new:
        if col not in old:
            changes.append(SchemaChange(
                "added", col,
                new_dtype=new[col],
                severity=added_severity,
            ))

    for col in old:
        if col not in new:
            changes.append(SchemaChange(
                "removed", col,
                old_dtype=old[col],
                severity=removed_severity,
            ))
        elif old[col] != new[col]:
            changes.append(SchemaChange(
                "retyped", col,
                old_dtype=old[col],
                new_dtype=new[col],
                severity=retyped_severity,
            ))

    return changes


def schema_evolution_report(changes: List[SchemaChange]) -> str:
    """Format schema changes as a human-readable report."""
    if not changes:
        return "Schema unchanged."
    lines = [f"Schema evolution — {len(changes)} change(s):", ""]
    for c in sorted(changes, key=lambda x: (x.severity, x.column)):
        lines.append(f"  {c}")
    return "\n".join(lines)


def assert_no_breaking_changes(
    current: "pl.DataFrame | Dict[str, str]",
    reference: "pl.DataFrame | Dict[str, str]",
) -> None:
    """
    Raise ValueError if there are any removed or retyped columns.

    Use this as a pipeline gate::

        statguardian.assert_no_breaking_changes(today_df, yesterday_df)
    """
    changes = detect_schema_changes(current, reference)
    breaking = [c for c in changes if c.severity == "error"]
    if breaking:
        msg = "\n".join(str(c) for c in breaking)
        raise ValueError(f"Breaking schema changes detected:\n{msg}")
