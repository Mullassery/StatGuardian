"""Canonical shared base for the sibling projects' local MCP/DAB connectors.

Several PyStream* projects ship a small ``_mcp_connector`` module that tries
``from statguardian._mcp_connector import BaseMCPConnector`` and falls back to
a local copy when this module is unavailable. This is the shared
implementation those imports resolve to once StatGuardian is installed.

The connector is deliberately dependency-light: standard library only. It
generates a config for the external ``dab`` (Data API Builder-style) runtime,
starts it as a subprocess, and tracks readiness.

Security defaults are intentionally narrow:

* The bind host defaults to loopback (``127.0.0.1``), and any non-loopback
  host requires an explicit ``allow_remote=True`` opt-in.
* CORS defaults to no origins unless ``allow_remote=True`` is explicitly set,
  in which case it defaults to ``["*"]`` (matching the previous consumer
  fallbacks); an explicit ``cors_origins`` always wins.
* Tool permissions default to read-only, and only widen to wildcard actions
  and roles when ``allow_remote=True`` is explicitly requested.

This connector has no authentication of its own. Callers exposing it beyond
loopback are responsible for putting authentication/network controls in
front of it.
"""

import ipaddress
import json
import logging
import subprocess
import tempfile
from abc import ABC, abstractmethod
from typing import Any, Optional

logger = logging.getLogger(__name__)

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8765


def _is_loopback(host: str) -> bool:
    """True for ``localhost`` and any IP address in a loopback range."""
    if host == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


class BaseMCPConnector(ABC):
    """Generate a ``dab`` config, run it, and manage its lifecycle.

    Args:
        project_name: Name used in log messages (e.g. ``"PyStreamPDF"``).
        port: TCP port the generated runtime binds.
        host: Bind interface. Defaults to loopback; any non-loopback value
            requires ``allow_remote=True``.
        allow_remote: Explicit acknowledgement that the connector will be
            reachable beyond localhost. This is also what enables the
            wildcard CORS/permission defaults.
        cors_origins: Explicit CORS origins. When omitted, defaults to ``[]``
            for loopback-only connectors and ``["*"]`` when ``allow_remote``
            is set.
        allowed_origins: Backwards-compatible alias for ``cors_origins`` used
            by older consumers of this shared base.
    """

    def __init__(
        self,
        project_name: str,
        port: int = DEFAULT_PORT,
        host: str = DEFAULT_HOST,
        allow_remote: bool = False,
        cors_origins: Optional[list[str]] = None,
        allowed_origins: Optional[list[str]] = None,
    ):
        if not allow_remote and not _is_loopback(host):
            raise ValueError(
                "Binding the MCP/DAB connector to a non-loopback host "
                f"(host={host!r}) requires allow_remote=True. By default the "
                "connector only binds to 127.0.0.1."
            )

        self.project_name = project_name
        self.port = port
        self.host = host
        self.allow_remote = allow_remote

        if cors_origins is None and allowed_origins is not None:
            cors_origins = list(allowed_origins)
        if cors_origins is None:
            cors_origins = [] if not allow_remote else ["*"]
        self.cors_origins = list(cors_origins)
        # Alias kept for consumers that historically read ``allowed_origins``.
        self.allowed_origins = self.cors_origins

        self.dab_process: Optional[subprocess.Popen] = None
        self._ready = False

    @abstractmethod
    def get_mcp_tools(self) -> dict[str, Any]:
        pass

    @abstractmethod
    def get_tool_handlers(self) -> Any:
        pass

    def start_mcp_connector(self) -> str:
        logger.info("Starting %s MCP...", self.project_name)
        try:
            tools = self.get_mcp_tools()
            self.handler = self.get_tool_handlers()
            config = self._generate_dab_config(tools)
            config_path = self._write_temp_config(config)
            self._start_dab_subprocess(config_path)
            self._ready = True
            return f"http://localhost:{self.port}/mcp"
        except Exception as e:
            logger.error("Failed: %s", e)
            raise

    def stop_mcp_connector(self):
        if self.dab_process:
            try:
                self.dab_process.terminate()
                self.dab_process.wait(timeout=5)
            except (subprocess.TimeoutExpired, OSError):
                pass
            self._ready = False

    def _generate_dab_config(
        self,
        tools: dict[str, Any],
        allowed_origins: Optional[list[str]] = None,
        allowed_roles: Optional[list[str]] = None,
    ) -> dict:
        """Build the runtime config, preserving the secure defaults.

        ``allowed_origins``/``allowed_roles`` are optional overrides used by
        consumers that historically customised those values (OpenAnchor).
        """
        origins = (
            list(allowed_origins)
            if allowed_origins is not None
            else list(self.cors_origins)
        )
        if allowed_roles is not None:
            roles = list(allowed_roles)
            actions = ["read"]
        else:
            roles = ["*"] if self.allow_remote else ["local"]
            actions = ["*"] if self.allow_remote else ["read"]

        return {
            "runtime": {
                "host": self.host,
                "port": self.port,
                "cors": {"origins": origins},
            },
            "entities": {
                k: {
                    "source": k,
                    "permissions": [{"actions": list(actions), "roles": roles}],
                }
                for k in tools
            },
            "rest": {"enabled": True, "path": "/api"},
            "graphql": {"enabled": True, "path": "/graphql"},
            "mcp": {"enabled": True, "path": "/mcp"},
        }

    def _write_temp_config(self, config: dict) -> str:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump(config, f)
            return f.name

    def _start_dab_subprocess(self, config_path: str):
        self.dab_process = subprocess.Popen(
            ["dab", "start", "--config", config_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def is_ready(self) -> bool:
        return self._ready
