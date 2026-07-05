#!/usr/bin/env python3
"""
StatGuard benchmark — compare against pandera, Great Expectations, and Pydantic.

Tests 5 checks on 100 000 rows: not_null, type, range(0-120),
regex (email), uniqueness.

Usage:
    pip install statguardian pandera polars pandas great-expectations pydantic
    python3 docs/bench/benchmark.py
"""

import time, random, statistics, warnings, io, contextlib, sys
warnings.filterwarnings("ignore")

N = 100_000
RUNS = 7
random.seed(42)

ids       = list(range(N))
emails    = [f"user{i}@example.com" for i in range(N)]
ages      = [random.randint(0, 120) for _ in range(N)]
countries = random.choices(["US","UK","DE","FR","CA","AU","JP"], k=N)
rows      = [{"id": i, "email": e, "age": a, "country": c}
             for i, e, a, c in zip(ids, emails, ages, countries)]

results: dict[str, tuple[float, float]] = {}  # name → (best_ms, median_ms)


def bench(fn, label: str) -> None:
    times = []
    for _ in range(RUNS):
        t0 = time.perf_counter()
        fn()
        times.append((time.perf_counter() - t0) * 1000)
    b, m = min(times), statistics.median(times)
    results[label] = (b, m)
    print(f"  {label:<50}  best={b:7.1f}ms  median={m:7.1f}ms")


print(f"\nBenchmark: {N:,} rows × 4 cols, best-of-{RUNS}, Apple M-series\n")

# ── StatGuard ─────────────────────────────────────────────────────────────────
try:
    import polars as pl
    import statguardian

    contract = statguardian.DataContract.from_dsl("""
dataset bench {
    schema {
        id:      int,    not_null, unique
        email:   string, regex="^[^@]+@[^@]+\\.[^@]+$"
        age:     int,    between(0, 120)
        country: string, not_null
    }
    quality { completeness(id) > 0.99 }
}
""")
    df = pl.DataFrame({"id": ids, "email": emails, "age": ages, "country": countries})
    bench(lambda: statguardian.execute(contract, df), "StatGuard 0.1 (Rust/Polars)")
except ImportError:
    print("  StatGuard: NOT INSTALLED — run: maturin develop --release", file=sys.stderr)

# ── Pydantic v2 — row-by-row ──────────────────────────────────────────────────
try:
    from pydantic import BaseModel, Field, ValidationError
    from typing import Annotated

    class UserRow(BaseModel):
        id:      int
        email:   Annotated[str, Field(pattern=r"^[^@]+@[^@]+\.[^@]+$")]
        age:     Annotated[int, Field(ge=0, le=120)]
        country: str

    def run_pydantic_row():
        errors = 0
        for row in rows:
            try:
                UserRow.model_validate(row)
            except ValidationError:
                errors += 1

    bench(run_pydantic_row, "Pydantic v2 (row-by-row, model_validate)")
except ImportError:
    print("  Pydantic: NOT INSTALLED — run: pip install pydantic", file=sys.stderr)

# ── Pydantic v2 — TypeAdapter bulk ───────────────────────────────────────────
try:
    from pydantic import TypeAdapter
    from typing import List

    ta = TypeAdapter(List[UserRow])

    def run_pydantic_bulk():
        try:
            ta.validate_python(rows)
        except ValidationError:
            pass

    bench(run_pydantic_bulk, "Pydantic v2 (TypeAdapter, bulk list)")
except ImportError:
    pass

# ── pandera ───────────────────────────────────────────────────────────────────
try:
    import pandas as pd
    import pandera.pandas as pa

    df_pd = pd.DataFrame({"id": ids, "email": emails, "age": ages, "country": countries})
    schema = pa.DataFrameSchema({
        "id":      pa.Column(int, nullable=False),
        "email":   pa.Column(str, pa.Check.str_matches(r"^[^@]+@[^@]+\.[^@]+$")),
        "age":     pa.Column(int, [pa.Check.ge(0), pa.Check.le(120)]),
        "country": pa.Column(str, nullable=False),
    })
    bench(lambda: schema.validate(df_pd), "pandera 0.31 (pandas, columnar)")
except ImportError:
    print("  pandera: NOT INSTALLED — run: pip install pandera", file=sys.stderr)

# ── Great Expectations ────────────────────────────────────────────────────────
try:
    import pandas as pd
    import great_expectations as gx

    df_pd = pd.DataFrame({"id": ids, "email": emails, "age": ages, "country": countries})
    ctx   = gx.get_context(mode="ephemeral")
    bd    = ctx.data_sources.add_pandas("d").add_dataframe_asset("a") \
               .add_batch_definition_whole_dataframe("b")
    suite = gx.ExpectationSuite(name="s")
    suite.add_expectation(gx.expectations.ExpectColumnValuesToNotBeNull(column="id"))
    suite.add_expectation(gx.expectations.ExpectColumnValuesToNotBeNull(column="country"))
    suite.add_expectation(gx.expectations.ExpectColumnValuesToBeBetween(
        column="age", min_value=0, max_value=120))
    suite.add_expectation(gx.expectations.ExpectColumnValuesToMatchRegex(
        column="email", regex=r"^[^@]+@[^@]+\.[^@]+$"))
    suite.add_expectation(gx.expectations.ExpectColumnValuesToBeUnique(column="id"))
    ctx.suites.add(suite)
    vd = ctx.validation_definitions.add(
        gx.ValidationDefinition(name="v", data=bd, suite=suite))

    def run_gx():
        with contextlib.redirect_stderr(io.StringIO()):
            vd.run(batch_parameters={"dataframe": df_pd})

    bench(run_gx, "Great Expectations 1.18 (pandas, columnar)")
except ImportError:
    print("  Great Expectations: NOT INSTALLED — pip install great-expectations", file=sys.stderr)

# ── Polars manual expressions (theoretical lower bound) ───────────────────────
try:
    import polars as pl
    df = pl.DataFrame({"id": ids, "email": emails, "age": ages, "country": countries})

    def run_polars():
        df.select([
            pl.col("id").is_null().sum().alias("id_nulls"),
            pl.col("email").str.contains(r"^[^@]+@[^@]+\.[^@]+$").not_().sum().alias("email_bad"),
            ((pl.col("age") < 0) | (pl.col("age") > 120)).sum().alias("age_bad"),
            pl.col("country").is_null().sum().alias("country_nulls"),
            pl.col("id").n_unique().alias("id_unique"),
        ])

    bench(run_polars, "Polars expressions (theoretical lower bound)")
except ImportError:
    pass

# ── Pure Python loops (absolute baseline) ────────────────────────────────────
import re
pattern = re.compile(r"^[^@]+@[^@]+\.[^@]+$")

def run_python():
    [0 <= a <= 120 for a in ages]
    [bool(pattern.match(e)) for e in emails]
    len(set(ids))
    sum(1 for i in ids if i is not None)

bench(run_python, "Pure Python loops (absolute baseline)")

# ── Report ────────────────────────────────────────────────────────────────────
sg_ms = results.get("StatGuard 0.1 (Rust/Polars)", (None, None))[0]

print(f"\n{'='*78}")
print(f"  {N:,} rows × 4 cols | best-of-{RUNS} | Apple M-series | Python 3.13")
print(f"  Checks: not_null · type · range(0-120) · regex(email) · uniqueness")
print(f"{'='*78}")
print(f"  {'Tool':<50}  {'Best':>8}  {'vs StatGuard':>14}")
print(f"  {'-'*50}  {'-'*8}  {'-'*14}")

for name, (best, _) in sorted(results.items(), key=lambda x: x[1][0]):
    if name == "StatGuard 0.1 (Rust/Polars)":
        ratio = "baseline"
    elif sg_ms:
        ratio = f"{best/sg_ms:.1f}× slower"
    else:
        ratio = "—"
    print(f"  {name:<50}  {best:>7.1f}ms  {ratio:>14}")

print(f"{'='*78}")
print(f"\nAll times are best-of-{RUNS} (ms). Lower is better.")
print(f"See BENCHMARKS.md for methodology and scaling projections.")
