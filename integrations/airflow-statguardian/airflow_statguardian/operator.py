"""StatGuardian validation as an Airflow task.

The operator is deliberately thin: it calls the same public Python API as
``statguardian validate`` and fails the task when the report does not pass.
Data does not move through Airflow; contract and data paths are templated so
they can come from params/XCom.
"""

from __future__ import annotations

from typing import Any, Optional

try:  # Airflow is intentionally optional at import time for unit tests/docs.
    from airflow.models import BaseOperator
except ImportError:  # pragma: no cover - exercised only without Airflow
    class BaseOperator:  # type: ignore[no-redef]
        """Placeholder so the module imports without Airflow installed."""

        def __init__(self, *args: Any, **kwargs: Any) -> None:
            raise ImportError(
                "airflow-statguardian requires apache-airflow; install it in "
                "your Airflow environment"
            )


def validation_failed(report: Any, fail_on_warning: bool = False) -> bool:
    """Return True when a StatGuardian report should fail the Airflow task."""
    if not getattr(report, "passed", False):
        return True
    if fail_on_warning and getattr(report, "violation_count", 0) > 0:
        return True
    return False


class StatGuardianValidateOperator(BaseOperator):
    """Validate a data file/table against a StatGuardian ``.sg`` contract."""

    template_fields = ("contract_path", "data_path", "reference_path")
    ui_color = "#2f6f4e"

    def __init__(
        self,
        *,
        task_id: str,
        contract_path: str,
        data_path: str,
        reference_path: Optional[str] = None,
        fail_on_warning: bool = False,
        **kwargs: Any,
    ) -> None:
        super().__init__(task_id=task_id, **kwargs)
        self.contract_path = contract_path
        self.data_path = data_path
        self.reference_path = reference_path
        self.fail_on_warning = fail_on_warning

    def execute(self, context: Any) -> dict:  # noqa: ARG002
        from airflow.exceptions import AirflowException
        from statguardian import DataContract, execute_file

        contract = DataContract.from_file(self.contract_path)
        report = execute_file(contract, self.data_path, self.reference_path)
        summary = report.summary()
        if validation_failed(report, fail_on_warning=self.fail_on_warning):
            raise AirflowException(
                "StatGuardian validation failed (%s violation(s)): %s"
                % (getattr(report, "violation_count", "unknown"), summary)
            )
        return {
            "summary": summary,
            "passed": getattr(report, "passed", False),
            "violations": getattr(report, "violation_count", 0),
        }
