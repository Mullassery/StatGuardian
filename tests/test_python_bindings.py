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
