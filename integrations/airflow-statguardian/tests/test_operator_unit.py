"""Unit tests that do not require an Airflow installation."""

from types import SimpleNamespace

from airflow_statguardian import validation_failed


def test_passing_report_does_not_fail():
    report = SimpleNamespace(passed=True, violation_count=0)
    assert validation_failed(report) is False


def test_failing_report_fails():
    report = SimpleNamespace(passed=False, violation_count=2)
    assert validation_failed(report) is True


def test_warning_only_fails_when_requested():
    report = SimpleNamespace(passed=True, violation_count=1)
    assert validation_failed(report) is False
    assert validation_failed(report, fail_on_warning=True) is True
