"""
statguardian dbt integration.

dbt (Jinja/SQL) cannot invoke the StatGuard Rust engine directly, so this
module is the runtime counterpart to integrations/dbt-statguardian/: it reads
a compiled dbt project's manifest.json, finds models tagged with a contract
via `meta.statguardian_contract`, resolves each model's warehouse relation
and connection from profiles.yml, and runs statguardian.execute_sql()
against it — the same way `edr` sits alongside the elementary dbt package.

Usage:
    dbt build
    statguardian dbt validate --project-dir . --write-results
    dbt test   # picks up statguardian.contract_passed results written above
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Iterator


class DbtIntegrationError(Exception):
    pass


def _render_env_vars(value: Any) -> Any:
    """Resolve dbt's `{{ env_var('X') }}` / `{{ env_var('X', 'default') }}`
    calls in profiles.yml without pulling in dbt-core as a dependency."""
    import os
    import re

    if isinstance(value, dict):
        return {k: _render_env_vars(v) for k, v in value.items()}
    if isinstance(value, list):
        return [_render_env_vars(v) for v in value]
    if not isinstance(value, str):
        return value

    pattern = re.compile(
        r"\{\{\s*env_var\(\s*['\"]([^'\"]+)['\"]\s*(?:,\s*['\"]([^'\"]*)['\"]\s*)?\)\s*\}\}"
    )

    def _sub(match: "re.Match[str]") -> str:
        name, default = match.group(1), match.group(2)
        if name in os.environ:
            return os.environ[name]
        if default is not None:
            return default
        raise DbtIntegrationError(
            f"profiles.yml references env_var('{name}') but it is not set "
            "and no default was given."
        )

    return pattern.sub(_sub, value)


def load_manifest(project_dir: Path) -> dict:
    manifest_path = project_dir / "target" / "manifest.json"
    if not manifest_path.exists():
        raise DbtIntegrationError(
            f"{manifest_path} not found — run `dbt build` or `dbt compile` first."
        )
    with open(manifest_path) as f:
        return json.load(f)


def load_dbt_project(project_dir: Path) -> dict:
    import yaml

    path = project_dir / "dbt_project.yml"
    if not path.exists():
        raise DbtIntegrationError(f"{path} not found — is --project-dir correct?")
    with open(path) as f:
        return yaml.safe_load(f)


def load_profile(profiles_dir: Path, profile_name: str) -> dict:
    import yaml

    path = profiles_dir / "profiles.yml"
    if not path.exists():
        raise DbtIntegrationError(f"{path} not found — is --profiles-dir correct?")
    with open(path) as f:
        profiles = yaml.safe_load(f)
    if profile_name not in profiles:
        raise DbtIntegrationError(
            f"Profile '{profile_name}' not found in {path} "
            f"(available: {[k for k in profiles if k != 'config']})"
        )
    return _render_env_vars(profiles[profile_name])


def resolve_target(profile: dict, target_name: str | None) -> dict:
    outputs = profile.get("outputs", {})
    target_name = target_name or profile.get("target")
    if not target_name:
        raise DbtIntegrationError("No target specified and profile has no default 'target'.")
    if target_name not in outputs:
        raise DbtIntegrationError(
            f"Target '{target_name}' not found in profile outputs "
            f"(available: {list(outputs)})"
        )
    return outputs[target_name]


def build_connection_string(target: dict) -> str:
    """Translate a dbt profiles.yml target into the SQLAlchemy-style
    connection string statguardian.execute_sql() expects."""
    adapter_type = target.get("type")

    if adapter_type in ("postgres", "postgresql"):
        return (
            f"postgresql://{target['user']}:{target.get('password', '')}"
            f"@{target['host']}:{target.get('port', 5432)}/{target['dbname']}"
        )

    if adapter_type == "redshift":
        return (
            f"redshift+psycopg2://{target['user']}:{target.get('password', '')}"
            f"@{target['host']}:{target.get('port', 5439)}/{target['dbname']}"
        )

    if adapter_type == "snowflake":
        account = target["account"]
        user = target["user"]
        password = target.get("password", "")
        database = target["database"]
        return f"snowflake://{user}:{password}@{account}/{database}"

    if adapter_type == "bigquery":
        project = target["project"]
        dataset = target.get("dataset", "")
        return f"bigquery://{project}/{dataset}"

    if adapter_type == "duckdb":
        return f"duckdb:///{target['path']}"

    raise DbtIntegrationError(
        f"statguardian dbt validate does not yet support dbt adapter type "
        f"{adapter_type!r}. Supported: postgres, redshift, snowflake, "
        f"bigquery, duckdb. Run validation manually with "
        f"statguardian.execute_sql() for other warehouses."
    )


def contracted_models(manifest: dict) -> Iterator[tuple[str, dict, str]]:
    """Yield (unique_id, node, contract_path) for every model whose
    schema.yml sets `meta.statguardian_contract: path/to/contract.sg`."""
    for unique_id, node in manifest.get("nodes", {}).items():
        if node.get("resource_type") != "model":
            continue
        meta = node.get("config", {}).get("meta") or node.get("meta") or {}
        contract_path = meta.get("statguardian_contract")
        if contract_path:
            yield unique_id, node, contract_path


def relation_for_node(node: dict) -> str:
    return f"{node['database']}.{node['schema']}.{node['alias']}"


def _write_result_row(conn_str: str, schema: str, row: dict) -> None:
    """Insert one validation result into {schema}.contract_validations.
    The table itself is created by the dbt package's on-run-end hook
    (statguardian.create_contract_validations_table); this only inserts."""
    import sqlalchemy

    engine = sqlalchemy.create_engine(conn_str)
    table = f"{schema}.contract_validations"
    columns = (
        "unique_id, model_name, relation_name, contract_path, passed, "
        "health_score, grade, violation_count, report_json, validated_at"
    )
    stmt = sqlalchemy.text(
        f"insert into {table} ({columns}) values "
        "(:unique_id, :model_name, :relation_name, :contract_path, :passed, "
        ":health_score, :grade, :violation_count, :report_json, :validated_at)"
    )
    with engine.begin() as conn:
        conn.execute(stmt, row)


def cmd_dbt_validate(args) -> None:
    from statguardian import DataContract, execute_sql

    project_dir = Path(args.project_dir).resolve()
    import os

    if args.profiles_dir:
        profiles_dir = Path(args.profiles_dir).resolve()
    elif os.environ.get("DBT_PROFILES_DIR"):
        profiles_dir = Path(os.environ["DBT_PROFILES_DIR"]).resolve()
    else:
        profiles_dir = Path.home() / ".dbt"

    try:
        manifest = load_manifest(project_dir)
        dbt_project = load_dbt_project(project_dir)
        profile_name = args.profile or dbt_project.get("profile")
        if not profile_name:
            raise DbtIntegrationError("No --profile given and dbt_project.yml has no 'profile' key.")
        profile = load_profile(profiles_dir, profile_name)
        target = resolve_target(profile, args.target)
        conn_str = build_connection_string(target)
        statguardian_schema_var = (dbt_project.get("vars") or {}).get("statguardian_schema")
        default_target_schema = target.get("schema") or ("main" if target.get("type") == "duckdb" else "public")
        results_schema = (
            args.results_schema
            or statguardian_schema_var
            or f"{default_target_schema}_statguardian"
        )
    except DbtIntegrationError as e:
        print(f"✗ {e}", file=sys.stderr)
        sys.exit(2)

    models = list(contracted_models(manifest))
    if not models:
        print(
            "[statguardian dbt] No models found with meta.statguardian_contract "
            "— nothing to validate."
        )
        sys.exit(0)

    results = []
    any_failed = False

    for unique_id, node, contract_rel_path in models:
        contract_path = project_dir / contract_rel_path
        model_name = node["name"]

        if not contract_path.exists():
            print(f"✗ ERROR | model={model_name} | contract file not found: {contract_path}", file=sys.stderr)
            any_failed = True
            continue

        relation = relation_for_node(node)
        try:
            contract = DataContract.from_file(str(contract_path))
            report = execute_sql(contract, conn_str, f"SELECT * FROM {relation}")
        except Exception as e:
            print(f"✗ ERROR | model={model_name} | {e}", file=sys.stderr)
            any_failed = True
            continue

        symbol = "✓" if report.passed else "✗"
        status = "PASS" if report.passed else "FAIL"
        print(
            f"{symbol} {status} | model={model_name} | contract={contract_rel_path} "
            f"| score={report.health_score:.2f} ({report.grade}) "
            f"| violations={report.violation_count}"
        )

        failed = not report.passed or (args.fail_on_warning and report.violation_count > 0)
        any_failed = any_failed or failed

        from datetime import datetime, timezone

        results.append(
            {
                "unique_id": unique_id,
                "model_name": model_name,
                "relation_name": relation,
                "contract_path": contract_rel_path,
                "passed": report.passed,
                "health_score": report.health_score,
                "grade": report.grade,
                "violation_count": report.violation_count,
                "report_json": report.to_json(),
                "validated_at": datetime.now(timezone.utc).isoformat(),
            }
        )

    if args.write_results and results:
        try:
            for row in results:
                _write_result_row(conn_str, results_schema, row)
            print(f"\n[statguardian dbt] Wrote {len(results)} result(s) to {results_schema}.contract_validations")
        except Exception as e:
            print(f"✗ Failed to write results to warehouse: {e}", file=sys.stderr)
            any_failed = True

    out_path = project_dir / "target" / "statguardian_results.json"
    out_path.write_text(json.dumps(results, indent=2, default=str))
    print(f"[statguardian dbt] Wrote {out_path}")

    sys.exit(1 if any_failed else 0)
