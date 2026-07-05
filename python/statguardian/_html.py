"""
HTML report generation for StatGuard ValidationReport objects.

Produces a self-contained, single-file HTML report with no external
dependencies — safe to email, commit to CI artefacts, or open offline.
"""

from __future__ import annotations

import html
import json
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass  # ValidationReport is a Rust PyObject; type hint only


def to_html(report) -> str:
    """
    Generate a self-contained HTML report from a ValidationReport.

    Args:
        report: A statguardian.ValidationReport object.

    Returns:
        A complete HTML document as a string.

    Example::

        report = statguardian.execute(contract, df)
        with open("report.html", "w") as f:
            f.write(statguardian.to_html(report))
    """
    data = json.loads(report.to_json())
    return _render(data)


def _badge(passed: bool) -> str:
    if passed:
        return '<span style="background:#16a34a;color:#fff;padding:3px 10px;border-radius:4px;font-weight:700">PASS ✓</span>'
    return '<span style="background:#dc2626;color:#fff;padding:3px 10px;border-radius:4px;font-weight:700">FAIL ✗</span>'


def _grade_color(grade: str) -> str:
    return {"A": "#16a34a", "B": "#65a30d", "C": "#d97706", "D": "#ea580c", "F": "#dc2626"}.get(grade, "#6b7280")


def _sev_color(sev: str) -> str:
    return {"Blocking": "#dc2626", "Error": "#ea580c", "Warning": "#d97706", "Info": "#6b7280"}.get(sev, "#6b7280")


def _render(d: dict) -> str:
    h = d.get("health", {})
    grade = h.get("grade", "?")
    score = h.get("score", 0)
    violations = d.get("violations", [])
    drift = d.get("drift_results", [])
    profiles = d.get("column_profiles", [])

    # ── Violations table ──────────────────────────────────────────────────────
    viol_rows = ""
    for v in violations:
        sev = v.get("severity", "")
        viol_rows += (
            f"<tr>"
            f"<td>{html.escape(v.get('column',''))}</td>"
            f"<td>{html.escape(v.get('check',''))}</td>"
            f"<td style='color:{_sev_color(sev)};font-weight:600'>{html.escape(sev)}</td>"
            f"<td>{html.escape(v.get('message',''))}</td>"
            f"</tr>"
        )
    viol_html = (
        f"<table><thead><tr><th>Column</th><th>Check</th><th>Severity</th><th>Message</th></tr></thead>"
        f"<tbody>{viol_rows}</tbody></table>"
        if violations else "<p style='color:#6b7280'>No violations.</p>"
    )

    # ── Drift table ───────────────────────────────────────────────────────────
    drift_rows = ""
    for dr in drift:
        passed_icon = "✓" if dr.get("passed") else "✗"
        color = "#16a34a" if dr.get("passed") else "#dc2626"
        drift_rows += (
            f"<tr>"
            f"<td>{html.escape(dr.get('column',''))}</td>"
            f"<td>{html.escape(dr.get('stat',''))}</td>"
            f"<td>{dr.get('reference_value', ''):.4f}</td>"
            f"<td>{dr.get('current_value', ''):.4f}</td>"
            f"<td>{dr.get('drift', 0):.4f}</td>"
            f"<td>{dr.get('psi') or '—'}</td>"
            f"<td style='color:{color};font-weight:700'>{passed_icon}</td>"
            f"</tr>"
        )
    drift_html = (
        f"<table><thead><tr><th>Column</th><th>Stat</th><th>Reference</th>"
        f"<th>Current</th><th>Drift</th><th>PSI</th><th>Passed</th></tr></thead>"
        f"<tbody>{drift_rows}</tbody></table>"
        if drift else "<p style='color:#6b7280'>No drift analysis (no reference dataset provided).</p>"
    )

    # ── Column profiles ───────────────────────────────────────────────────────
    profile_rows = ""
    for p in profiles:
        profile_rows += (
            f"<tr>"
            f"<td>{html.escape(p.get('name',''))}</td>"
            f"<td>{p.get('mean','')}</td><td>{p.get('std','')}</td>"
            f"<td>{p.get('p95','')}</td><td>{p.get('null_rate','')}</td>"
            f"<td>{p.get('distinct_count','')}</td>"
            f"</tr>"
        )
    profiles_html = (
        f"<table><thead><tr><th>Column</th><th>Mean</th><th>Std</th>"
        f"<th>P95</th><th>Null rate</th><th>Distinct</th></tr></thead>"
        f"<tbody>{profile_rows}</tbody></table>"
        if profiles else "<p style='color:#6b7280'>No column profiles.</p>"
    )

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>StatGuard — {html.escape(d.get('dataset',''))} — {d.get('executed_at','')[:10]}</title>
<style>
  body{{font-family:system-ui,sans-serif;margin:0;padding:24px 32px;color:#1f2937;background:#f9fafb}}
  h1{{font-size:1.5rem;margin:0 0 4px}}
  h2{{font-size:1.1rem;margin:28px 0 8px;border-bottom:1px solid #e5e7eb;padding-bottom:4px}}
  .meta{{color:#6b7280;font-size:.875rem;margin-bottom:20px}}
  .cards{{display:flex;gap:16px;flex-wrap:wrap;margin-bottom:24px}}
  .card{{background:#fff;border:1px solid #e5e7eb;border-radius:8px;padding:16px 20px;min-width:140px}}
  .card .label{{font-size:.75rem;color:#6b7280;text-transform:uppercase;letter-spacing:.05em}}
  .card .value{{font-size:1.75rem;font-weight:700;margin-top:4px}}
  table{{width:100%;border-collapse:collapse;background:#fff;border:1px solid #e5e7eb;border-radius:6px;overflow:hidden;font-size:.875rem}}
  th{{background:#f3f4f6;text-align:left;padding:8px 12px;font-weight:600;border-bottom:1px solid #e5e7eb}}
  td{{padding:7px 12px;border-bottom:1px solid #f3f4f6}}
  tr:last-child td{{border-bottom:none}}
  tr:hover td{{background:#f9fafb}}
</style>
</head>
<body>
<h1>StatGuard Validation Report</h1>
<div class="meta">
  Dataset: <strong>{html.escape(d.get('dataset',''))}</strong> &nbsp;·&nbsp;
  Executed: {d.get('executed_at','')[:19].replace('T',' ')} UTC &nbsp;·&nbsp;
  Duration: {d.get('duration_ms',0)} ms &nbsp;·&nbsp;
  Rows: {d.get('row_count',0):,} &nbsp;·&nbsp;
  ID: <code style="font-size:.8rem">{d.get('id','')}</code>
</div>

<div class="cards">
  <div class="card">
    <div class="label">Status</div>
    <div class="value" style="font-size:1rem;padding-top:8px">{_badge(d.get('passed',False))}</div>
  </div>
  <div class="card">
    <div class="label">Health score</div>
    <div class="value" style="color:{_grade_color(grade)}">{score:.0%}</div>
  </div>
  <div class="card">
    <div class="label">Grade</div>
    <div class="value" style="color:{_grade_color(grade)}">{grade}</div>
  </div>
  <div class="card">
    <div class="label">Violations</div>
    <div class="value">{len(violations)}</div>
  </div>
  <div class="card">
    <div class="label">Drift checks</div>
    <div class="value">{sum(1 for dr in drift if not dr.get('passed'))}/{len(drift)} failed</div>
  </div>
</div>

<h2>Violations ({len(violations)})</h2>
{viol_html}

<h2>Drift results ({len(drift)})</h2>
{drift_html}

<h2>Column profiles ({len(profiles)})</h2>
{profiles_html}

<p style="margin-top:32px;color:#9ca3af;font-size:.8rem">
  Generated by <strong>StatGuard</strong> · <a href="https://github.com/Mullassery/statguardian" style="color:#6b7280">github.com/Mullassery/statguardian</a>
</p>
</body>
</html>"""
