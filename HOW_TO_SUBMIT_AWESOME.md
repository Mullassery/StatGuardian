# How to Submit StatGuard to Awesome Lists

This guide helps you submit StatGuard to awesome-python and awesome-rust.

---

## awesome-python Submission

### Repository
**https://github.com/sindresorhus/awesome**

### Steps

1. **Fork the repository**
   ```bash
   git clone https://github.com/sindresorhus/awesome.git
   cd awesome
   ```

2. **Edit the README.md**
   - Find the "Data Validation" or "Data Analysis" section
   - Add StatGuard entry (see format below)
   - Keep entries **alphabetically sorted** within sections

3. **Entry Format**
   ```markdown
   - [statguardian](https://github.com/Mullassery/statguardian) - Rust-native data quality, validation, and statistical drift monitoring engine with Python bindings. 13-25× faster than pandera. Supports Delta Lake, Iceberg, cloud storage (S3/GCS/Azure), and 13 SQL databases. Single declarative DSL contract for schema validation, quality rules, drift detection, and anomaly detection. MIT license.
   ```

4. **Commit and Push**
   ```bash
   git checkout -b add/statguard
   git add README.md
   git commit -m "Add StatGuard - data quality validation engine"
   git push origin add/statguard
   ```

5. **Create Pull Request**
   - Go to https://github.com/sindresorhus/awesome/pulls
   - Click "New Pull Request"
   - Select your branch
   - Fill in PR template:
     - **Title:** Add StatGuard
     - **Description:** Data quality validation library with 13-25× speedup over Python alternatives. Supports Delta Lake, Iceberg, cloud storage, and 13 SQL databases.

### Section Recommendation
- Look for "Data" or "Data Analysis" section first
- If none exists, create under "Serialization" or "Data Processing"
- **Do NOT** create new sections without discussion

---

## awesome-rust Submission

### Repository
**https://github.com/rust-unofficial/awesome-rust**

### Steps

1. **Fork the repository**
   ```bash
   git clone https://github.com/rust-unofficial/awesome-rust.git
   cd awesome-rust
   ```

2. **Edit the README.md**
   - Find **Libraries** → **Python** or **Data processing** section
   - Add StatGuard entry
   - Keep entries **alphabetically sorted**

3. **Entry Format**
   ```markdown
   - [statguardian](https://github.com/Mullassery/statguardian) — High-performance data quality and drift detection. Rust + PyO3 + Polars. 13-25× faster than Python libraries. Native support for Delta Lake, Iceberg, cloud storage, SQL databases. [MIT](https://github.com/Mullassery/statguardian/blob/main/LICENSE).
   ```

4. **Commit and Push**
   ```bash
   git checkout -b add/statguard
   git add README.md
   git commit -m "Add StatGuard - data quality engine with Python bindings"
   git push origin add/statguard
   ```

5. **Create Pull Request**
   - Go to https://github.com/rust-unofficial/awesome-rust/pulls
   - Click "New Pull Request"
   - Select your branch
   - Fill in PR template:
     - **Title:** Add StatGuard
     - **Description:** Production-grade data quality validation engine. Rust core with Python bindings via PyO3. 13-25× faster than pandera/Great Expectations. Supports Delta Lake, Iceberg, cloud storage, SQL databases.

### Section Recommendation
Look for:
1. **Libraries** → **Python** (best fit for Python bindings)
2. **Libraries** → **Data processing**
3. **Libraries** → **Data format** (if others exist)

---

## PR Template for Both

```markdown
# Add StatGuard

**Library:** StatGuard (Rust data quality engine with Python bindings)

**Link:** https://github.com/Mullassery/statguardian

**Description:**
High-performance data quality, validation, and statistical drift monitoring engine with a Python-first API. Built in Rust 2021 with PyO3 bindings. 13-25× faster than pandera and Great Expectations for equivalent checks.

**Key features:**
- Schema validation (types, nulls, ranges, regex, uniqueness, enums)
- Quality rules (completeness, consistency, validity)
- Statistical drift detection (PSI, KS test)
- Anomaly detection (outliers, duplicates, cardinality explosions)
- Lakehouse support (Delta Lake, Apache Iceberg without Spark)
- Cloud storage (S3, GCS, Azure)
- SQL databases (13 OSS connectors: PostgreSQL, MySQL, BigQuery, Snowflake, Redshift, etc.)
- Apache Spark integration
- Streaming support with micro-batch windows

**License:** MIT

**Benchmarks:**
- StatGuard: ~2 ms (100k rows, 5 checks)
- pandera: 26.5 ms (13× slower)
- Great Expectations: 50.4 ms (25× slower)

**Production ready:** ✅ Stable v0.1, 33 tests passing, comprehensive documentation

**Why include:**
- [awesome-python] Best-in-class performance for data quality validation in Python ecosystem
- [awesome-rust] Excellent example of Rust + Python integration via PyO3/maturin
```

---

## Tips for Approval

### For awesome-python
- Emphasize **performance leader** position
- Highlight **unique features** (Delta, Iceberg, no Spark)
- Mention **production use** (benchmarks, documentation)
- Show **Python-first** nature (PyPI available, Pythonic API)

### For awesome-rust
- Emphasize **PyO3/maturin** architecture
- Highlight **real performance gains** (13-25× faster)
- Show **quality metrics** (tests, documentation)
- Note **complexity handled** (Polars, Arrow, HyperLogLog, PEG)

---

## Common Issues & Solutions

### "This is too niche"
**Response:** Data quality is critical for any data pipeline (dbt, Airflow, ML). StatGuard fills a gap in the Python ecosystem with a production-grade solution.

### "Performance claims need verification"
**Response:** Benchmarks are in [BENCHMARKS.md](https://github.com/Mullassery/statguardian/blob/main/BENCHMARKS.md) with reproducible methodology. Results measured on Apple M-series, 100k rows, 5 checks (null, type, range, regex, uniqueness). Runs are best-of-7.

### "Too many features / too complex"
**Response:** Single DSL covers all use cases (schema, quality, drift, anomalies). Users install only what they need (core, cloud, SQL, Spark are optional features).

### "Duplication with pandera/GX"
**Response:** Complementary, not duplicative:
- **pandera** — Slower, Python-only, no Delta/Iceberg/Spark
- **GX** — Slower, Python-only, no native Delta/Iceberg
- **StatGuard** — 13-25× faster, Rust core, Delta/Iceberg/Spark/Cloud/SQL

---

## After Submission

1. **Monitor PR** — Respond to reviewer questions promptly
2. **Address feedback** — Most PRs need small tweaks (formatting, details)
3. **Announce merge** — Once merged, announce on social media / GitHub Discussions

---

## Timeline Expectation

- **awesome-python** — 3-7 days for review
- **awesome-rust** — 3-14 days for review

Both repos have volunteer maintainers, so patience and professionalism in responses help.

---

## References

- [awesome-python submission guidelines](https://github.com/sindresorhus/awesome/blob/main/contributing.md)
- [awesome-rust contribution guidelines](https://github.com/rust-unofficial/awesome-rust/blob/master/CONTRIBUTING.md)

---

**Good luck with your submissions! 🚀**
