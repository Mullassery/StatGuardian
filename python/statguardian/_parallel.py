"""
Parallel multi-file validation for StatGuard.

Validate a glob pattern or list of file paths concurrently, using a
thread pool to overlap IO-bound work.  Each file is validated
independently against the same contract.
"""

from __future__ import annotations

import glob as _glob
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from typing import Callable, Iterator, List, Optional, Tuple, Union


@dataclass
class FileResult:
    """Outcome of validating one file."""
    path: str
    report: object          # ValidationReport, or None on error
    error: Optional[str]    # exception message if validation failed

    @property
    def passed(self) -> bool:
        return self.report is not None and self.report.passed

    @property
    def failed(self) -> bool:
        return self.error is not None or (
            self.report is not None and not self.report.passed
        )


def execute_files(
    contract,
    paths: Union[str, List[str]],
    *,
    workers: int = None,
    reference_path: str = None,
    on_complete: Callable[[FileResult], None] = None,
) -> List[FileResult]:
    """
    Validate multiple Excel / Parquet / CSV / … files against one contract.

    Files are processed concurrently using a thread pool.  The GIL is
    released during the Rust validation call, so true parallelism is
    achieved for CPU-bound checks on multi-core machines.

    Args:
        contract:       DataContract compiled with ``statguardian.DataContract.from_file()``.
        paths:          Glob pattern (e.g. ``"data/2026/**/*.parquet"``) or list of paths.
        workers:        Thread pool size. Defaults to ``min(32, cpu_count + 4)``.
        reference_path: Optional reference file for drift detection (applied to all files).
        on_complete:    Optional callback called with each ``FileResult`` as it completes.

    Returns:
        List of ``FileResult`` objects, in completion order (not input order).

    Example::

        contract = statguardian.DataContract.from_file("orders.sg")
        results = statguardian.execute_files(contract, "data/orders_*.parquet")

        failed = [r for r in results if r.failed]
        print(f"{len(failed)}/{len(results)} files failed")

        for r in failed:
            print(r.path, r.error or r.report.summary())
    """
    from . import execute_file

    file_list = _resolve_paths(paths)
    if not file_list:
        return []

    results: List[FileResult] = []

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {
            pool.submit(_validate_one, execute_file, contract, path, reference_path): path
            for path in file_list
        }
        for future in as_completed(futures):
            path = futures[future]
            try:
                report = future.result()
                result = FileResult(path=path, report=report, error=None)
            except Exception as exc:
                result = FileResult(path=path, report=None, error=str(exc))

            results.append(result)
            if on_complete is not None:
                on_complete(result)

    return results


def execute_files_stream(
    contract,
    paths: Union[str, List[str]],
    *,
    workers: int = None,
    reference_path: str = None,
) -> Iterator[FileResult]:
    """
    Like ``execute_files`` but yields each ``FileResult`` as it completes.

    Useful when you want to react to results immediately (e.g. fail-fast
    on the first error) without waiting for all files to finish.

    Example::

        for result in statguardian.execute_files_stream(contract, "data/**/*.parquet"):
            if result.failed:
                print(f"FAIL {result.path}: {result.error or result.report.summary()}")
                break  # stop early
    """
    from . import execute_file

    file_list = _resolve_paths(paths)
    if not file_list:
        return

    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {
            pool.submit(_validate_one, execute_file, contract, path, reference_path): path
            for path in file_list
        }
        for future in as_completed(futures):
            path = futures[future]
            try:
                yield FileResult(path=path, report=future.result(), error=None)
            except Exception as exc:
                yield FileResult(path=path, report=None, error=str(exc))


def _validate_one(execute_file_fn, contract, path: str, reference_path: Optional[str]):
    """Worker function submitted to the thread pool."""
    return execute_file_fn(contract, path, reference_path=reference_path)


def _resolve_paths(paths: Union[str, List[str]]) -> List[str]:
    if isinstance(paths, str):
        return sorted(_glob.glob(paths, recursive=True))
    return [str(p) for p in paths]
