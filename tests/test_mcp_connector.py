"""Tests for the canonical shared MCP connector base.

Consumers (PyStreamMCP, PyStreamPDF, and others) import
``statguardian._mcp_connector.BaseMCPConnector`` and fall back to local copies
when it is absent. These tests pin the shared surface and, importantly, the
secure defaults.
"""

import json
import os
from typing import Any

import pytest

from statguardian._mcp_connector import BaseMCPConnector


class _Connector(BaseMCPConnector):
    def __init__(self, *args, tools=None, handlers=None, **kwargs):
        super().__init__(*args, **kwargs)
        self._tools = tools if tools is not None else {"tool_a": {}, "tool_b": {}}
        self._handlers = handlers if handlers is not None else object()

    def get_mcp_tools(self) -> dict[str, Any]:
        return self._tools

    def get_tool_handlers(self) -> Any:
        return self._handlers


def test_abstract_base_cannot_be_instantiated():
    with pytest.raises(TypeError):
        BaseMCPConnector("demo")


def test_loopback_read_only_defaults():
    connector = _Connector("Demo")

    assert connector.project_name == "Demo"
    assert connector.port == 8765
    assert connector.host == "127.0.0.1"
    assert connector.allow_remote is False
    assert connector.cors_origins == []
    assert connector.allowed_origins == []
    assert connector.dab_process is None
    assert connector.is_ready() is False

    config = connector._generate_dab_config({"tool_a": {}})
    assert config["runtime"] == {
        "host": "127.0.0.1",
        "port": 8765,
        "cors": {"origins": []},
    }
    assert config["entities"]["tool_a"]["permissions"] == [
        {"actions": ["read"], "roles": ["local"]}
    ]
    assert config["mcp"] == {"enabled": True, "path": "/mcp"}
    assert config["rest"] == {"enabled": True, "path": "/api"}
    assert config["graphql"] == {"enabled": True, "path": "/graphql"}


@pytest.mark.parametrize(
    "host",
    ["0.0.0.0", "::", "192.168.1.10", "mcp.example.com", "127.0.0.1.evil.com"],
)
def test_non_loopback_host_requires_explicit_opt_in(host):
    with pytest.raises(ValueError, match="allow_remote=True"):
        _Connector("Demo", host=host)


@pytest.mark.parametrize("host", ["127.0.0.1", "127.0.0.5", "localhost", "::1"])
def test_loopback_hosts_do_not_require_opt_in(host):
    connector = _Connector("Demo", host=host)
    assert connector.host == host


def test_allow_remote_enables_wildcard_defaults_explicitly():
    connector = _Connector("Demo", host="0.0.0.0", allow_remote=True)

    assert connector.cors_origins == ["*"]
    config = connector._generate_dab_config({"tool_a": {}})
    assert config["runtime"]["host"] == "0.0.0.0"
    assert config["runtime"]["cors"]["origins"] == ["*"]
    assert config["entities"]["tool_a"]["permissions"] == [
        {"actions": ["*"], "roles": ["*"]}
    ]


def test_explicit_cors_origins_are_used_without_wildcarding():
    connector = _Connector(
        "Demo", cors_origins=["https://example.com"], allow_remote=False
    )

    config = connector._generate_dab_config({"tool_a": {}})
    assert config["runtime"]["cors"]["origins"] == ["https://example.com"]
    # Explicit CORS does not widen permissions.
    assert config["entities"]["tool_a"]["permissions"][0]["actions"] == ["read"]


def test_explicit_cors_origins_override_allow_remote_default():
    connector = _Connector(
        "Demo",
        host="0.0.0.0",
        allow_remote=True,
        cors_origins=["https://example.com"],
    )

    config = connector._generate_dab_config({"tool_a": {}})
    assert config["runtime"]["cors"]["origins"] == ["https://example.com"]


def test_allowed_origins_alias_is_honoured():
    connector = _Connector("Demo", allowed_origins=["http://127.0.0.1:9000"])

    assert connector.cors_origins == ["http://127.0.0.1:9000"]
    assert connector.allowed_origins == ["http://127.0.0.1:9000"]
    config = connector._generate_dab_config({"tool_a": {}})
    assert config["runtime"]["cors"]["origins"] == ["http://127.0.0.1:9000"]


def test_allowed_roles_override_keeps_read_only_actions():
    connector = _Connector("Demo")

    config = connector._generate_dab_config(
        {"tool_a": {}}, allowed_roles=["admin", "user"]
    )
    assert config["entities"]["tool_a"]["permissions"] == [
        {"actions": ["read"], "roles": ["admin", "user"]}
    ]


def test_dab_config_covers_every_tool():
    connector = _Connector("Demo")
    config = connector._generate_dab_config({"alpha": {}, "beta": {}})

    assert set(config["entities"]) == {"alpha", "beta"}
    assert all(
        entity["source"] == name for name, entity in config["entities"].items()
    )


def test_write_temp_config_creates_valid_json():
    connector = _Connector("Demo")

    path = connector._write_temp_config({"runtime": {"host": "127.0.0.1"}})
    try:
        with open(path, encoding="utf-8") as f:
            assert json.load(f) == {"runtime": {"host": "127.0.0.1"}}
    finally:
        os.remove(path)


def test_start_and_stop_lifecycle(monkeypatch):
    connector = _Connector("Demo", port=9123)
    started = {}

    class _Process:
        def __init__(self):
            self.terminated = False
            self.waited = False

        def terminate(self):
            self.terminated = True

        def wait(self, timeout=None):
            self.waited = True

    process = _Process()

    def fake_start(config_path):
        started["config_path"] = config_path
        connector.dab_process = process

    monkeypatch.setattr(connector, "_write_temp_config", lambda config: "/tmp/fake.json")
    monkeypatch.setattr(connector, "_start_dab_subprocess", fake_start)

    url = connector.start_mcp_connector()

    assert url == "http://localhost:9123/mcp"
    assert started["config_path"] == "/tmp/fake.json"
    assert connector.handler is connector._handlers
    assert connector.is_ready() is True

    connector.stop_mcp_connector()
    assert process.terminated is True
    assert process.waited is True
    assert connector.is_ready() is False


def test_failed_start_leaves_connector_not_ready(monkeypatch):
    class _Failing(_Connector):
        def get_mcp_tools(self):
            raise RuntimeError("boom")

    connector = _Failing("Demo")
    monkeypatch.setattr(connector, "_start_dab_subprocess", lambda path: None)

    with pytest.raises(RuntimeError, match="boom"):
        connector.start_mcp_connector()

    assert connector.is_ready() is False
