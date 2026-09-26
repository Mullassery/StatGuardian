#!/usr/bin/env python3
"""
StatGuardian vs pandera — real data benchmark (NYC 311 Service Requests, live Socrata API).

Usage:
    pip install statguardian pandera polars pandas requests
    python3 docs/bench/nyc311_vs_pandera.py
"""
import json, time, statistics, sys, os

import requests

CACHE = os.path.join(os.path.dirname(__file__), "_nyc311_200k_cache.json")
if os.path.exists(CACHE):
    with open(CACHE) as f:
        raw = json.load(f)
else:
    url = "https://data.cityofnewyork.us/resource/erm2-nwe9.json"
    cols = "unique_key,created_date,complaint_type,borough,incident_zip,status,latitude,longitude"
    raw = []
    for offset in range(0, 200_000, 50_000):
        params = {"$limit": 50_000, "$offset": offset, "$select": cols, "$order": "unique_key"}
        r = requests.get(url, params=params, timeout=60)
        r.raise_for_status()
        chunk = r.json()
        raw.extend(chunk)
        if len(chunk) < 50_000:
            break
    with open(CACHE, "w") as f:
        json.dump(raw, f)

N = len(raw)
RUNS = 7

def get(r, k, cast=None, default=None):
    v = r.get(k, default)
    if v is None or v == "":
        return default
    if cast:
        try:
            return cast(v)
        except (ValueError, TypeError):
            return default
    return v

unique_key    = [get(r, "unique_key") for r in raw]
complaint     = [get(r, "complaint_type") for r in raw]
borough       = [get(r, "borough") for r in raw]
incident_zip  = [get(r, "incident_zip") for r in raw]
status        = [get(r, "status") for r in raw]
latitude      = [get(r, "latitude", float) for r in raw]
longitude     = [get(r, "longitude", float) for r in raw]

VALID_BOROUGH = ["BRONX", "BROOKLYN", "MANHATTAN", "QUEENS", "STATEN ISLAND", "Unspecified"]
VALID_STATUS  = ["In Progress", "Pending", "Assigned", "Closed", "Started", "Open"]

results = {}

def bench(fn, label, extra=None):
    times = []
    out = None
    for _ in range(RUNS):
        t0 = time.perf_counter()
        out = fn()
        times.append((time.perf_counter() - t0) * 1000)
    b, m = min(times), statistics.median(times)
    results[label] = {"best_ms": b, "median_ms": m, "extra": extra}
    print(f"  {label:<45} best={b:8.1f}ms  median={m:8.1f}ms  {extra or ''}")

print(f"\nDataset: NYC 311 Service Requests, live pull via data.cityofnewyork.us Socrata API")
print(f"Rows: {N:,}  Cols: 7  Real nulls (lat/lon/zip/borough), real categorical noise, best-of-{RUNS}\n")

# ---- StatGuardian ----
try:
    import polars as pl
    import statguardian

    contract = statguardian.DataContract.from_dsl(f"""
dataset service_requests {{
    schema {{
        unique_key:   string, not_null, unique
        complaint_type: string, not_null
        borough:      string, enum=["BRONX","BROOKLYN","MANHATTAN","QUEENS","STATEN ISLAND","Unspecified"]
        status:       string, not_null, enum=["In Progress","Pending","Assigned","Closed","Started","Open"]
        incident_zip: string, regex="^[0-9]{{5}}$"
        latitude:     float, between(-90, 90)
        longitude:    float, between(-180, 180)
    }}
    quality {{
        completeness(unique_key) > 0.999
    }}
}}
""")
    df = pl.DataFrame({
        "unique_key": unique_key, "complaint_type": complaint, "borough": borough,
        "status": status, "incident_zip": incident_zip,
        "latitude": latitude, "longitude": longitude,
    })

    def run_sg():
        return statguardian.execute(contract, df)

    report = run_sg()
    n_violations = report.summary().count("violation") if hasattr(report, "summary") else None
    bench(run_sg, "StatGuardian 2.5.0 (Rust/Polars)",
          extra=f"passed={getattr(report,'passed',None)}")
except Exception as e:
    print(f"  StatGuardian FAILED: {type(e).__name__}: {e}", file=sys.stderr)
    raise

# ---- pandera ----
try:
    import pandas as pd
    import pandera.pandas as pa

    df_pd = pd.DataFrame({
        "unique_key": unique_key, "complaint_type": complaint, "borough": borough,
        "status": status, "incident_zip": incident_zip,
        "latitude": latitude, "longitude": longitude,
    })

    schema = pa.DataFrameSchema({
        "unique_key": pa.Column(str, nullable=False, unique=True),
        "complaint_type": pa.Column(str, nullable=False),
        "borough": pa.Column(str, pa.Check.isin(VALID_BOROUGH), nullable=True),
        "status": pa.Column(str, pa.Check.isin(VALID_STATUS), nullable=False),
        "incident_zip": pa.Column(str, pa.Check.str_matches(r"^[0-9]{5}$"), nullable=True),
        "latitude": pa.Column(float, [pa.Check.ge(-90), pa.Check.le(90)], nullable=True),
        "longitude": pa.Column(float, [pa.Check.ge(-180), pa.Check.le(180)], nullable=True),
    })

    def run_pandera():
        try:
            return schema.validate(df_pd, lazy=True)
        except pa.errors.SchemaErrors as e:
            return e

    out = run_pandera()
    bench(run_pandera, "pandera 0.26.1 (pandas, columnar, lazy)",
          extra=f"failure_cases={len(out.failure_cases) if hasattr(out,'failure_cases') else 0}")
except Exception as e:
    print(f"  pandera FAILED: {type(e).__name__}: {e}", file=sys.stderr)
    raise

print("\n--- Summary ---")
sg = results.get("StatGuardian 2.5.0 (Rust/Polars)")
pd_ = results.get("pandera 0.26.1 (pandas, columnar, lazy)")
if sg and pd_:
    speedup = pd_["median_ms"] / sg["median_ms"]
    print(f"StatGuardian median: {sg['median_ms']:.1f}ms")
    print(f"pandera median:      {pd_['median_ms']:.1f}ms")
    print(f"Speedup:             {speedup:.2f}x")

with open("results.json", "w") as f:
    json.dump(results, f, indent=2)
