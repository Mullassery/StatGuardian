"""
statguard CLI — validate data files against contracts from the terminal.

Supports local files, Delta Lake, and Apache Iceberg tables.
For cloud storage (S3/GCS/Azure), SQL databases, and Spark, use the Python API.

Usage:
    statguardian validate --contract contract.sg --file data.parquet [OPTIONS]
    statguardian check   --contract contract.sg

Supported file formats:
    Parquet, CSV, JSON, Avro, ORC, Arrow IPC, Delta Lake (_delta_log/), Iceberg (metadata/)

For more information: statguard --help
Full documentation: https://github.com/Mullassery/statguardian/blob/main/docs/CLI.md
"""

import argparse
import sys


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="statguard",
        description="StatGuard data quality engine",
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

    args = parser.parse_args()

    if args.command == "check":
        _cmd_check(args)
    elif args.command == "validate":
        _cmd_validate(args)
    else:
        parser.print_help()
        sys.exit(1)


def _cmd_check(args) -> None:
    from statguardian import validate_dsl
    try:
        dsl = open(args.contract).read()
        name = validate_dsl(dsl)
        print(f"✓ DSL valid — dataset: {name}")
    except Exception as e:
        print(f"✗ DSL error: {e}", file=sys.stderr)
        sys.exit(1)


def _cmd_validate(args) -> None:
    from statguardian import DataContract, execute_file
    try:
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
