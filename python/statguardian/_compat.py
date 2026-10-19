"""Dataframe compatibility helpers for StatGuardian.

The Rust engine validates Polars DataFrames.  This module accepts the
dataframe objects users actually have (Pandas, Polars, PyArrow, and any
other eager dataframe supported by Narwhals) and converts them at the
boundary.  Polars inputs are passed through untouched so existing users do
not pay a conversion cost.
"""

from __future__ import annotations

from typing import Any, Optional

try:  # pragma: no cover - exercised when the extension is unavailable
    from ._statguardian import execute as _execute
except ImportError:  # pragma: no cover
    _execute = None


def _is_polars(df: Any) -> bool:
    try:
        import polars as pl
    except ImportError:  # pragma: no cover
        return False
    return isinstance(df, pl.DataFrame)


def to_polars(df: Any, *, argument: str = "dataframe") -> Any:
    """Convert an eager dataframe/table to a Polars DataFrame.

    Parameters
    ----------
    df:
        A Polars DataFrame (returned unchanged), a Pandas DataFrame, a
        PyArrow Table, or any eager object supported by Narwhals.
    argument:
        Name used in error messages.

    Raises
    ------
    TypeError
        If the object is not a supported eager dataframe/table.
    ImportError
        If a non-Polars input is supplied without Narwhals installed.
    """
    if df is None:
        return None

    try:
        import polars as pl
    except ImportError as exc:  # pragma: no cover
        raise ImportError(
            "StatGuardian requires polars; install it with `pip install polars`."
        ) from exc

    if isinstance(df, pl.DataFrame):
        return df

    # PyArrow is a base dependency of StatGuardian, so accept Tables even
    # when the optional Narwhals extra is not installed.
    try:
        import pyarrow as pa

        if isinstance(df, pa.Table):
            return pl.from_arrow(df)
    except ImportError:  # pragma: no cover
        pass

    try:
        import narwhals as nw
    except ImportError as exc:
        raise ImportError(
            f"{argument} is {type(df).__name__}, not a Polars DataFrame. "
            "Install Narwhals to convert Pandas/PyArrow/other eager dataframes: "
            "`pip install 'statguardian[pandas]'` (or `pip install narwhals`)."
        ) from exc

    try:
        frame = nw.from_native(df, eager_only=True)
    except TypeError as exc:
        raise TypeError(
            f"{argument} must be an eager dataframe or table, got {type(df).__name__}. "
            "Lazy frames are not supported by execute(); collect them first."
        ) from exc

    return pl.from_arrow(frame.to_arrow())


def execute(contract: Any, df: Any, reference: Optional[Any] = None) -> Any:  # noqa: UP045
    """Execute a contract against Polars, Pandas, PyArrow or Narwhals input.

    This is the public ``statguardian.execute`` entry point.  The compiled
    extension remains Polars-native; conversion happens here.
    """
    if _execute is None:  # pragma: no cover
        raise ImportError(
            "StatGuardian's compiled extension is not built. "
            "Run `maturin develop` or install a wheel."
        )
    return _execute(
        contract,
        to_polars(df, argument="df"),
        to_polars(reference, argument="reference") if reference is not None else None,
    )


__all__ = ["execute", "to_polars"]
