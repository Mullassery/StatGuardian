"""Test Python bindings for StatGuardian Rust core.

Unlike a pure import-only smoke test, `test_execute_end_to_end_through_python_bindings`
below actually drives the documented quick-start path (DataContract.from_dsl
-> execute -> report) through the compiled pyo3 extension, so a broken
binding (wrong function signature, renamed export, botched DataFrame
conversion, etc.) shows up as a real test failure rather than passing
silently.
"""

import pytest

try:
    import statguardian
    import polars as pl
    _IMPORT_ERROR = None
except ImportError as e:
    statguardian = None
    pl = None
    _IMPORT_ERROR = e

pytestmark = pytest.mark.skipif(
    _IMPORT_ERROR is not None,
    reason=f"statguardian bindings not built yet (run maturin develop): {_IMPORT_ERROR}",
)


def test_statguardian_import():
    """Verify Python bindings are accessible."""
    assert statguardian is not None


def test_statguardian_version():
    """Verify version is set."""
    assert hasattr(statguardian, "__version__")
    assert isinstance(statguardian.__version__, str)
    assert statguardian.__version__


def test_execute_end_to_end_through_python_bindings():
    """Real functional check: DataContract.from_dsl -> execute -> report,
    through the actual compiled extension, not a mock."""
    contract = statguardian.DataContract.from_dsl(
        """
        dataset orders {
            schema {
                order_id: string, not_null, unique
                amount:   float,  positive
            }
            quality {
                completeness(order_id) > 0.99
            }
        }
        """
    )

    clean_df = pl.DataFrame(
        {
            "order_id": ["a", "b", "c"],
            "amount": [10.0, 20.0, 30.0],
        }
    )
    report = statguardian.execute(contract, clean_df)
    assert report.passed
    assert report.row_count == 3
    assert report.violation_count == 0

    dirty_df = pl.DataFrame(
        {
            "order_id": ["a", "a", "c"],  # duplicate -> uniqueness violation
            "amount": [10.0, -5.0, 30.0],  # negative -> positive() violation
        }
    )
    dirty_report = statguardian.execute(contract, dirty_df)
    assert dirty_report.violation_count > 0
    assert len(dirty_report.violations()) == dirty_report.violation_count


# ---------------------------------------------------------------------------
# StreamGuard: execute_stream_ndjson + `statguardian stream` CLI
# ---------------------------------------------------------------------------

import json
import subprocess
import sys

_STREAM_DSL = """
dataset events {
    schema {
        id: int, not_null, between(0, 120)
        name: string, not_null
        score: float, positive
    }
}
"""


def _write(tmp_path, name, text):
    p = tmp_path / name
    p.write_text(text)
    return str(p)


def _run_cli(*args):
    return subprocess.run(
        [sys.executable, "-c", "from statguardian._cli import main; main()", *args],
        capture_output=True,
        text=True,
    )


def test_execute_stream_ndjson_clean(tmp_path):
    contract = statguardian.DataContract.from_dsl(_STREAM_DSL)
    path = _write(
        tmp_path,
        "clean.ndjson",
        '{"id":1,"name":"a","score":1.0}\n'
        '{"id":2,"name":"b","score":2.0}\n'
        '{"id":3,"name":"c","score":3.0}\n',
    )
    report = statguardian.execute_stream_ndjson(contract, path)
    assert isinstance(report, statguardian.StreamReport)
    assert report.passed
    assert report.records_validated == 3
    assert report.findings() == []
    assert report.malformed_count == 0
    assert report.exit_code() == 0


def test_execute_stream_ndjson_nonblocking_needs_fail_on_warning(tmp_path):
    contract = statguardian.DataContract.from_dsl(_STREAM_DSL)
    path = _write(
        tmp_path,
        "bad.ndjson",
        '{"id":1,"name":"a","score":1.0}\n'
        '{"id":999,"name":"b","score":-2.0}\n',  # out of range + non-positive
    )
    report = statguardian.execute_stream_ndjson(contract, path)
    assert report.findings()  # between/positive are Severity::Error, not Blocking
    assert report.passed and report.exit_code() == 0
    assert report.exit_code(True) == 1  # fail_on_warning


def test_execute_stream_ndjson_malformed_mapping_and_fail_fast(tmp_path):
    contract = statguardian.DataContract.from_dsl(_STREAM_DSL)
    path = _write(
        tmp_path,
        "malformed.ndjson",
        '{"id":1,"name":"a","score":1.0}\n'
        "not json\n"
        '{"id":999,"name":"c","score":-1.0}\n',
    )
    report = statguardian.execute_stream_ndjson(contract, path, malformed="collect")
    mal = report.malformed()
    assert len(mal) == 1 and mal[0]["kind"] == "Json"
    # The skipped malformed line must not shift global record ids: line 3 -> index 2
    assert {f["record"] for f in report.findings()} == {2}

    with pytest.raises(Exception):
        statguardian.execute_stream_ndjson(contract, path, malformed="fail_fast")


def test_execute_stream_ndjson_missing_column_is_blocking(tmp_path):
    contract = statguardian.DataContract.from_dsl(_STREAM_DSL)
    path = _write(tmp_path, "missing.ndjson", '{"id":1,"name":"a"}\n')
    report = statguardian.execute_stream_ndjson(contract, path)
    assert not report.passed
    assert report.exit_code() == 1
    assert "Blocking" in {f["severity"] for f in report.findings()}


def test_execute_stream_ndjson_limits(tmp_path):
    contract = statguardian.DataContract.from_dsl(_STREAM_DSL)
    path = _write(
        tmp_path,
        "many.ndjson",
        "".join('{"id":%d,"name":"n","score":1.0}\n' % i for i in range(10)),
    )
    report = statguardian.execute_stream_ndjson(
        contract, path, batch_size=4, max_records=5
    )
    assert report.records_validated == 5
    assert report.truncated


def test_stream_cli_exit_codes_and_stream_separation(tmp_path):
    contract_path = _write(tmp_path, "events.sg", _STREAM_DSL)
    clean = _write(tmp_path, "clean.ndjson", '{"id":1,"name":"a","score":1.0}\n')
    missing = _write(tmp_path, "missing.ndjson", '{"id":1,"name":"a"}\n')
    malformed = _write(tmp_path, "malformed.ndjson", "nope\n")
    bad = _write(tmp_path, "bad.ndjson", '{"id":999,"name":"a","score":-1.0}\n')

    clean_run = _run_cli("stream", "--contract", contract_path, "--file", clean)
    assert clean_run.returncode == 0 and clean_run.stderr == ""

    fow_run = _run_cli(
        "stream", "--contract", contract_path, "--file", bad, "--fail-on-warning"
    )
    assert fow_run.returncode == 1

    block_run = _run_cli("stream", "--contract", contract_path, "--file", missing)
    assert block_run.returncode == 1

    fail_run = _run_cli(
        "stream", "--contract", contract_path, "--file", malformed,
        "--malformed", "fail_fast",
    )
    assert fail_run.returncode == 2

    json_run = _run_cli(
        "stream", "--contract", contract_path, "--file", bad, "--format", "json"
    )
    json.loads(json_run.stdout)  # machine report on stdout

