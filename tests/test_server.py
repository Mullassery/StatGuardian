"""HTTP service tests (fastapi-gated; no Rust extension needed)."""

import importlib.util
import sys
from pathlib import Path

import pytest

pytest.importorskip("fastapi")
from fastapi.testclient import TestClient

_SERVER = Path(__file__).resolve().parents[1] / "python" / "statguardian" / "server.py"
_spec = importlib.util.spec_from_file_location("statguardian_server_under_test", _SERVER)
assert _spec and _spec.loader
_server = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = _server
_spec.loader.exec_module(_server)


class _Report:
    def __init__(self, passed):
        self.passed = passed
        self.violation_count = 0 if passed else 1

    def summary(self):
        return "ok" if self.passed else "amount < 0"


def _app(passed=True):
    return _server.create_app(contract=object(), execute_fn=lambda _c, _df: _Report(passed))


def test_healthz_and_validate_passing():
    client = TestClient(_app(True))
    assert client.get("/healthz").json()["status"] == "ok"
    response = client.post(
        "/validate",
        json={"entity": {"attributes": {"amount": 10}}, "contract_id": "default"},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["passed"] is True
    assert body["quality_score"] == 1.0


def test_validate_failure_reports_issue():
    client = TestClient(_app(False))
    response = client.post(
        "/validate",
        json={"entity": {"attributes": {"amount": -1}}},
    )
    assert response.status_code == 200
    body = response.json()
    assert body["passed"] is False
    assert "amount < 0" in body["issues"][0]


def test_unbound_contract_is_503():
    client = TestClient(_server.create_app(execute_fn=lambda _c, _df: _Report(True)))
    assert client.post("/validate", json={"entity": {"attributes": {}}}).status_code == 503


def test_detect_changes_is_explicitly_501():
    client = TestClient(_app(True))
    response = client.post("/detect-changes", json={"entity": {"attributes": {}}})
    assert response.status_code == 501


def test_validate_uses_pandas_dataframe():
    """The HTTP gate builds a one-row pandas frame.

    pandas is declared in the `api` extra so a clean
    ``pip install "statguardian[api]"`` provides it.
    """
    import pandas as pd

    seen = {}

    def execute_fn(_contract, frame):
        seen["type"] = type(frame)
        return _Report(True)

    client = TestClient(_server.create_app(contract=object(), execute_fn=execute_fn))
    response = client.post("/validate", json={"entity": {"attributes": {"amount": 10}}})
    assert response.status_code == 200, response.text
    assert seen["type"] is pd.DataFrame
