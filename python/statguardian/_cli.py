"""
statguardian CLI — validate data files against contracts from the terminal.

Supports local files, Delta Lake, and Apache Iceberg tables.
For cloud storage (S3/GCS/Azure), SQL databases, and Spark, use the Python API.

Usage:
    statguardian validate --contract contract.sg --file data.parquet [OPTIONS]
    statguardian stream   --contract contract.sg --file events.ndjson [OPTIONS]
    statguardian check   --contract contract.sg

Supported file formats:
    Parquet, CSV, JSON, Avro, ORC, Arrow IPC, Delta Lake (_delta_log/), Iceberg (metadata/)

For more information: statguardian --help
Full documentation: https://github.com/Mullassery/statguardian/blob/main/docs/CLI.md
"""

import argparse
import sys

from . import __version__


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="statguardian",
        description="statguardian data quality engine",
    )
    parser.add_argument(
        "--version",
        action="version",
        version=f"statguardian {__version__}",
    )
    sub = parser.add_subparsers(dest="command")

    # statguardian validate
    validate = sub.add_parser(
        "validate",
        help="Validate a data file against a contract",
        description="Validate data against a StatGuard contract. Supports Parquet, CSV, JSON, Avro, ORC, Arrow IPC, Delta Lake, and Iceberg."
    )
    validate.add_argument(
        "--contract",
        required=True,
        metavar="PATH",
        help="Path to .sg DSL contract file"
    )
    validate.add_argument(
        "--file",
        required=True,
        metavar="PATH",
        help="Path to data file or table directory (auto-detected format: parquet/csv/json/avro/orc/ipc/_delta_log/metadata)"
    )
    validate.add_argument(
        "--reference",
        default=None,
        metavar="PATH",
        help="Optional reference data file for drift detection (same formats as --file)"
    )
    validate.add_argument(
        "--format",
        choices=["summary", "json", "prometheus"],
        default="summary",
        help="Output format (default: summary). Use 'json' for parsing, 'prometheus' for scraping."
    )
    validate.add_argument(
        "--fail-on-warning",
        action="store_true",
        help="Exit with code 1 if any violation found (default: only errors cause failure)"
    )

    # statguardian check
    check = sub.add_parser(
        "check",
        help="Syntax-check a DSL contract file",
        description="Check a StatGuard contract for syntax errors without validating data."
    )
    check.add_argument(
        "--contract",
        required=True,
        metavar="PATH",
        help="Path to .sg DSL contract file"
    )

    # statguardian stream
    stream = sub.add_parser(
        "stream",
        help="Incrementally validate an NDJSON file (StreamGuard)",
        description="Validate newline-delimited JSON (NDJSON) record-by-record with "
                     "bounded memory. Records are buffered into micro-batches and run "
                     "through the same engine as `validate`."
    )
    stream.add_argument("--contract", required=True, metavar="PATH",
                        help="Path to .sg DSL contract file")
    stream.add_argument("--file", required=True, metavar="PATH",
                        help="Path to an NDJSON file (one JSON object per line)")
    stream.add_argument("--batch-size", type=int, default=1000, metavar="N",
                        help="Records per validation micro-batch (default: 1000)")
    stream.add_argument("--max-records", type=int, default=None, metavar="N",
                        help="Stop after reading N records (default: unbounded)")
    stream.add_argument("--max-record-bytes", type=int, default=None, metavar="N",
                        help="Per-line byte cap (default: 8 MiB)")
    stream.add_argument("--malformed", choices=["fail_fast", "skip", "collect"], default="collect",
                        help="Handling for malformed/oversized lines (default: collect)")
    stream.add_argument("--collect-max", type=int, default=100, metavar="N",
                        help="Max malformed diagnostics retained when collecting (default: 100)")
    stream.add_argument("--format", choices=["summary", "json", "prometheus"], default="summary",
                        help="Output format (default: summary)")
    stream.add_argument("--fail-on-warning", action="store_true",
                        help="Exit with code 1 if any finding is present (default: only blocking)")
    stream.add_argument("--allow-raw-diagnostics", action="store_true",
                        help="Attach raw line previews to malformed diagnostics (off by default)")

    # statguardian dbt
    dbt = sub.add_parser(
        "dbt",
        help="Validate dbt models against StatGuard contracts",
        description="Companion CLI for integrations/dbt-statguardian/. Reads a compiled "
                     "dbt project's manifest.json and runs StatGuard contracts against "
                     "models tagged with meta.statguardian_contract."
    )
    dbt_sub = dbt.add_subparsers(dest="dbt_command")

    dbt_validate = dbt_sub.add_parser(
        "validate",
        help="Run StatGuard contracts against contracted dbt models",
    )
    dbt_validate.add_argument("--project-dir", default=".", metavar="PATH",
                               help="dbt project directory (default: current directory)")
    dbt_validate.add_argument("--profiles-dir", default=None, metavar="PATH",
                               help="Directory containing profiles.yml (default: $DBT_PROFILES_DIR or ~/.dbt)")
    dbt_validate.add_argument("--profile", default=None,
                               help="Profile name (default: read from dbt_project.yml)")
    dbt_validate.add_argument("--target", default=None,
                               help="Target name (default: profile's default target)")
    dbt_validate.add_argument("--write-results", action="store_true",
                               help="Write results into <schema>_statguardian.contract_validations "
                                    "so dbt's statguardian.contract_passed test can see them")
    dbt_validate.add_argument("--results-schema", default=None,
                               help="Override the schema results are written to "
                                    "(default: <target schema>_statguardian)")
    dbt_validate.add_argument("--fail-on-warning", action="store_true",
                               help="Exit non-zero if any violation found, not just contract failures")

    args = parser.parse_args()

    if args.command == "check":
        _cmd_check(args)
    elif args.command == "validate":
        _cmd_validate(args)
    elif args.command == "stream":
        _cmd_stream(args)
    elif args.command == "dbt":
        if args.dbt_command == "validate":
            from statguardian._dbt import cmd_dbt_validate
            cmd_dbt_validate(args)
        else:
            dbt.print_help()
            sys.exit(1)
    else:
        parser.print_help()
        sys.exit(1)


def _cmd_check(args) -> None:
    from statguardian import validate_dsl
    from statguardian._dsl_validator import validate_dsl_contract
    try:
        dsl = open(args.contract).read()
        is_valid, error = validate_dsl_contract(dsl)
        if not is_valid:
            print(f"✗ DSL error: {error}", file=sys.stderr)
            sys.exit(1)
        name = validate_dsl(dsl)
        print(f"✓ DSL valid — dataset: {name}")
    except Exception as e:
        print(f"✗ DSL error: {e}", file=sys.stderr)
        sys.exit(1)


def _cmd_stream(args) -> None:
    from statguardian import DataContract, execute_stream_ndjson
    from statguardian._dsl_validator import validate_dsl_contract
    try:
        dsl = open(args.contract).read()
        is_valid, error = validate_dsl_contract(dsl)
        if not is_valid:
            print(f"✗ Contract error: {error}", file=sys.stderr)
            sys.exit(2)
        contract = DataContract.from_file(args.contract)
        report = execute_stream_ndjson(
            contract,
            args.file,
            batch_size=args.batch_size,
            max_records=args.max_records,
            max_record_bytes=args.max_record_bytes,
            malformed=args.malformed,
            collect_max=args.collect_max,
            allow_raw_diagnostics=args.allow_raw_diagnostics,
        )
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        sys.exit(2)

    # Diagnostics go to stderr; the machine report goes to stdout.
    for m in report.malformed():
        print(f"[malformed record {m['record']}: {m['kind']}] {m['reason']}", file=sys.stderr)

    try:
        if args.format == "json":
            print(report.to_json_pretty())
        elif args.format == "prometheus":
            print(report.to_prometheus())
        else:
            print(report.summary())
    except BrokenPipeError:
        # Downstream (e.g. `head`) closed the pipe; exit quietly without a traceback.
        try:
            sys.stdout.close()
        except Exception:
            pass
        sys.exit(2)

    sys.exit(report.exit_code(args.fail_on_warning))


def _cmd_validate(args) -> None:
    from statguardian import DataContract, execute_file
    from statguardian._dsl_validator import validate_dsl_contract
    try:
        dsl = open(args.contract).read()
        is_valid, error = validate_dsl_contract(dsl)
        if not is_valid:
            print(f"✗ Contract error: {error}", file=sys.stderr)
            sys.exit(2)
        contract = DataContract.from_file(args.contract)
        report   = execute_file(contract, args.file, args.reference)
    except Exception as e:
        print(f"✗ Error: {e}", file=sys.stderr)
        sys.exit(2)

    if args.format == "json":
        print(report.to_json_pretty())
    elif args.format == "prometheus":
        print(report.to_prometheus())
    else:
        print(report.summary())

    fail = not report.passed
    if args.fail_on_warning:
        fail = fail or report.violation_count > 0

    sys.exit(1 if fail else 0)
