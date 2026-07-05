# statguardian vs Other Data Quality Tools

## Benchmark Comparison

| Feature | statguardian | Pandera | Great Expectations | Soda | Freshness |
|---------|-----------|---------|-------------------|------|-----------|
| **Speed** | 13–25× faster | 1× (baseline) | 1–2× | 2–3× | 3–5× |
| **Schema Validation** | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Drift Detection** | ✓ (PSI + KS) | ✗ | ✓ | ✓ | ✗ |
| **Anomaly Detection** | ✓ | ✗ | Limited | ✓ | ✗ |
| **Declarative DSL** | ✓ | ✗ | ✗ | ✓ | ✓ |
| **Delta Lake Support** | ✓ | Limited | ✓ | ✓ | Limited |
| **Iceberg Support** | ✓ | ✗ | ✗ | ✗ | ✗ |

## Performance

statguardian achieves 13–25× speedup through:
- **Rust engine**: Compiled, type-safe execution
- **Columnar processing**: Leverages Polars + Arrow for SIMD optimization
- **Parallel execution**: Rayon-backed multithreading

See [BENCHMARKS.md](BENCHMARKS.md) for detailed methodology and test cases.
