"""Pandas/PyArrow/Narwhals compatibility for statguardian.execute().

These tests drive the public `execute` path (not the internal adapter) so a
regression in the wrapper or in pyo3-polars conversion is visible.
"""

import pytest

pytest.importorskip("polars")
pytest.importorskip("pandas")
pytest.importorskip("narwhals")

import pandas as pd
import polars as pl
import pyarrow as pa

import statguardian


@pytest.fixture()
def contract():
    return statguardian.DataContract.from_dsl(
        """
        dataset orders {
            schema {
                order_id: string, not_null, unique
                amount: float, positive
            }
        }
        """
    )


@pytest.fixture()
def pandas_frame():
    return pd.DataFrame({"order_id": ["a", "b"], "amount": [1.0, 2.0]})


def test_execute_accepts_pandas(contract, pandas_frame):
    report = statguardian.execute(contract, pandas_frame)
    assert report.passed


def test_execute_pandas_and_polars_agree(contract, pandas_frame):
    pandas_report = statguardian.execute(contract, pandas_frame)
    polars_report = statguardian.execute(contract, pl.from_pandas(pandas_frame))
    assert pandas_report.passed == polars_report.passed


def test_execute_accepts_arrow_table(contract, pandas_frame):
    report = statguardian.execute(contract, pa.Table.from_pandas(pandas_frame))
    assert report.passed


def test_execute_accepts_pandas_reference(contract, pandas_frame):
    report = statguardian.execute(contract, pandas_frame, reference=pandas_frame)
    assert report.passed


def test_execute_rejects_unsupported_object(contract):
    with pytest.raises(TypeError, match="eager dataframe or table"):
        statguardian.execute(contract, {"order_id": "a"})


def test_execute_rejects_lazy_frame(contract, pandas_frame):
    with pytest.raises((TypeError, ImportError), match=r"eager|Narwhals|Lazy"):
        statguardian.execute(contract, pl.from_pandas(pandas_frame).lazy())
