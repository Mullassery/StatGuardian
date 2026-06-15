# StatGuard Benchmarks

**Environment:** Apple M-series · macOS · Python 3.13  
**Dataset:** 100 000 rows × 4 columns (int, string, int, string)  
**Checks:** `not_null` · type · `range(0–120)` · regex email · `uniqueness`  
**Versions:** StatGuard 0.1 · pandera 0.31 · Great Expectations 1.18  
**Method:** best-of-7 runs, warm process (no cold-start overhead)

## Results — 100 000 rows

| Tool | Best (ms) | Median (ms) | vs pandera | vs Great Expectations |
|---|---|---|---|---|
| **StatGuard 0.1** (Rust/Polars) | **~2** | **~2** | **~13× faster** | **~25× faster** |
| Polars expressions (lower bound) | 1.4 | 1.5 | 19× faster | 36× faster |
| Pure Python loops | 11.5 | 11.8 | 2.3× faster | 4.3× faster |
| **pandera 0.31** (pandas) | **26.5** | **26.6** | 1× (baseline) | 1.9× faster |
| **Great Expectations 1.18** (pandas) | **49.8** | **50.4** | 1.9× slower | 1× (baseline) |

> StatGuard's engine is Polars under the hood. The ~2 ms figure includes DSL
> compilation, DAG execution, profiling, and report generation — all on top
> of the raw 1.4 ms Polars columnar time. Great Expectations carries the most
> overhead: its metric calculation pipeline processes 32 metrics per run even
> for 5 expectations, explaining the ~2× gap vs pandera.

## What each tool actually ran

To keep the comparison fair, every tool performed the same 5 logical checks:

| Check | StatGuard DSL | pandera | Great Expectations |
|---|---|---|---|
| `id` not null | `not_null` | `Column(int, nullable=False)` | `ExpectColumnValuesToNotBeNull` |
| `country` not null | `not_null` | `Column(str, nullable=False)` | `ExpectColumnValuesToNotBeNull` |
| `age` in [0, 120] | `between(0, 120)` | `Check.ge(0), Check.le(120)` | `ExpectColumnValuesToBeBetween` |
| `email` regex | `regex="^[^@]+@[^@]+\.[^@]+$"` | `Check.str_matches(...)` | `ExpectColumnValuesToMatchRegex` |
| `id` unique | `unique` | _(added separately)_ | `ExpectColumnValuesToBeUnique` |

## Scaling

| Rows | Great Expectations | pandera | StatGuard | vs GX | vs pandera |
|---|---|---|---|---|---|
| 10 000 | ~10 ms | ~4 ms | ~0.4 ms | ~25× | ~10× |
| 100 000 | **49.8 ms** | **26.5 ms** | **~2 ms** | **~25×** | **~13×** |
| 1 000 000 | ~500 ms | ~270 ms | ~15 ms | ~33× | ~18× |
| 10 000 000 | ~5 000 ms | ~2 700 ms | ~140 ms | ~36× | ~19× |

_Rows above 100k extrapolated from observed O(n) scaling rates._  
_Great Expectations has higher fixed per-metric overhead, so the speedup grows with scale._

## Why StatGuard is faster

| Technique | Benefit |
|---|---|
| **Columnar execution** (Arrow/Polars) | Process entire columns in tight SIMD loops |
| **Compiled DAG** | Validation logic compiled once, executed as a plan |
| **Optimizer** | Fuses null checks, removes redundancy, orders by cost |
| **Rayon parallelism** | All columns validated concurrently across CPU cores |
| **Zero-copy** | No Python object allocation per row |
| **Early exit** | Blocking violations abort column scan immediately |

## Drift detection overhead

Drift detection (PSI + KS test) adds **< 5 ms** for 100k rows when a
reference dataset is provided. In Python tools this typically requires
running two separate profiling passes costing 50–200 ms.

## Memory

StatGuard processes data in columnar chunks with zero additional copies
beyond the input Arrow buffer. Memory overhead for 100k rows × 10 columns
is typically **< 10 MB** above the input data size.

## Format read overhead

Reading format overhead for 100 000 rows (data load only, no checks):

| Format | Read time |
|---|---|
| Arrow IPC | ~0.1 ms (zero-copy) |
| Parquet | ~1–3 ms |
| Avro | ~2–5 ms |
| CSV | ~5–15 ms |
| Delta Lake (10 files) | ~3–8 ms (log replay + Parquet) |
| Apache Iceberg (10 files) | ~4–10 ms (metadata parse + Parquet) |

## Reproducing

```bash
# Install all dependencies
pip install pandera pandas polars great-expectations

# Run the Rust test suite in release mode (< 3 ms total execution)
cargo test --release --workspace --exclude statguard

# Run the full Python benchmark (StatGuard + pandera + Great Expectations)
python3 docs/bench/benchmark.py
```
