# Install

StatGuardian is a Rust-native Python extension (built with maturin). It requires
**Python 3.9+**.

## PyPI

```bash
pip install statguardian
```

> **Platform note (2.8.0):** PyPI currently publishes a **macOS arm64** wheel
> (`cp38-abi3-macosx_11_0_arm64`) plus an sdist. On Linux x86_64, Windows, or
> macOS Intel, build from source (below) until wheels are published for those
> platforms.

Verify the CLI:

```bash
statguardian --version
statguardian --help
```

Verify the library:

```bash
python -c "import statguardian; print(statguardian.__version__)"
```

## From source (Python + Rust)

Requires a Rust toolchain (see `rust-toolchain.toml`) and `maturin`.

```bash
git clone https://github.com/Mullassery/StatGuardian
cd statguardian
python -m venv .venv && . .venv/bin/activate
pip install maturin
maturin develop --release
```

## Optional extras

| Extra | Provides |
|---|---|
| `pandas` | Pandas input (`pandas==2.1.0`, `narwhals`) |
| `api` | HTTP service (`fastapi`, `uvicorn`) |
| `sql-postgres`, `sql-mysql`, `sql-sqlite`, `sql-duckdb`, … | SQL connectors |
| `dev` | pytest/coverage |

```bash
pip install "statguardian[pandas]"
pip install "statguardian[api]"
```

PyArrow is a base dependency; Polars is passed through. Delta Lake and Iceberg
inputs are parsed natively (no Spark needed).

## DataFrame inputs

- Polars — no copy.
- Pandas — install the `pandas` extra.
- PyArrow Tables and other eager Narwhals-compatible frames — supported.

## Troubleshooting

- **No wheel for your platform** — build from source (above).
- **Import error after upgrading polars** — the extension pins a compatible
  `pyo3-polars`/polars range; use the polars version allowed by the installed
  wheel or rebuild from source.
- **Pandas validation fails to install** — the `pandas` extra pins
  `pandas==2.1.0`; use an isolated environment if your project needs another
  version.

See [Project status](STATUS.md) and [Contract DSL](contract-dsl.md).
