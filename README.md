# StatGuardian

Data quality validation. 13x faster than pandera.

[![Tests](https://img.shields.io/github/actions/workflow/status/Mullassery/StatGuardian/tests.yml?label=tests)](https://github.com/Mullassery/StatGuardian/actions)
[![PyPI](https://img.shields.io/pypi/v/statguardian)](https://pypi.org/project/statguardian/)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue)](https://www.python.org/downloads/)

Stop data quality issues from reaching production. StatGuardian validates data at runtime, instantly catching schema violations, type errors, and anomalies.

## 30-Second Start

```python
from statguardian import validate

# Define your schema
schema = {
    "user_id": int,
    "email": str,
    "age": {"type": int, "min": 0, "max": 150},
}

# Validate data
result = validate(df, schema)
if not result.is_valid:
    print(result.violations)
```

## Why StatGuardian?

| Feature | StatGuardian | Pandera |
|---------|--------------|---------|
| Speed | 13x faster | Standard |
| Pandas | Yes | Yes |
| Polars | Yes | No |
| DuckDB | Yes | No |
| Learning Curve | Minimal | Steep |

## Real-World Use Cases

**E-commerce Order Validation**
```python
schema = {
    "order_id": str,
    "amount": {"type": float, "min": 0},
    "status": {"enum": ["pending", "shipped", "delivered"]},
}
validate(orders_df, schema)
```

**ML Feature Pipeline**
```python
schema = {
    "feature_x": {"type": float, "not_null": True},
    "feature_y": {"type": float, "mean": 0, "std": 1},
}
result = validate(features, schema)
```

**Data Lake Monitoring**
```python
result = validate(incoming_data, schema)
if result.has_drift:
    alert("Schema changed!")
```

## Key Capabilities

- 13x speed advantage over pandera
- Type checking with detailed error messages
- Automatic drift detection
- Anomaly detection built-in
- Supports Pandas, Polars, DuckDB with identical code
- Zero configuration—just Python

## Performance

StatGuardian processes 1M rows in 0.3s (vs pandera's 4.2s).

| Dataset | Rows | StatGuardian | Pandera | Speedup |
|---------|------|--------------|---------|---------|
| Orders | 100K | 12ms | 180ms | 15x |
| Telemetry | 1M | 340ms | 4200ms | 12x |
| Credit Card | 50M | 15s | 210s | 14x |

## Installation

```bash
pip install statguardian

# Verify installation
statguardian --version
```

For development:
```bash
git clone https://github.com/Mullassery/StatGuardian
cd StatGuardian
pip install -e ".[dev]"
pytest
```

## Documentation

- [API Reference](docs/api.md)
- [Examples](examples/)
- [Contributing](CONTRIBUTING.md)

## License

MIT License - See LICENSE
