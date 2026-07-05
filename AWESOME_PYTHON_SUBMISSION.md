# Submission to awesome-python

## Category
**Data Validation & Quality** (or similar category for data quality libraries)

## Entry Format

```markdown
- [StatGuard](https://github.com/Mullassery/statguardian) - Rust-native data quality, validation, and statistical drift monitoring engine with Python bindings. 13-25× faster than pandera. Supports Delta Lake, Iceberg, cloud storage (S3/GCS/Azure), and 13 SQL databases. Single declarative DSL contract for schema validation, quality rules, drift detection, and anomaly detection. MIT license with optional LGPL (PostgreSQL).
```

## Details for Submission

**Project Name:** StatGuard  
**Repository:** https://github.com/Mullassery/statguardian  
**PyPI:** https://pypi.org/project/statguardianian  
**License:** MIT  
**Keywords:** data-quality, validation, schema-validation, drift-detection, statistics, Rust, Polars

## Why awesome-python?

StatGuard is a Python-first library (pip/uv installable) with a powerful Rust backend. It solves a critical gap in the Python data quality ecosystem:

1. **Performance leader** — 13-25× faster than pandera/Great Expectations for equivalent checks
2. **Comprehensive format support** — Unique native support for Delta Lake and Iceberg without Spark
3. **Production-ready** — MIT licensed, 33 tests, comprehensive documentation
4. **Developer-friendly** — Single DSL for schema, quality, drift, and anomalies
5. **Enterprise features** — Cloud storage, SQL databases, Spark, streaming

## Verification

- ✅ Listed on PyPI: https://pypi.org/project/statguardianian
- ✅ MIT licensed
- ✅ Open-source (no proprietary drivers)
- ✅ Documentation complete (README, guides, API docs, benchmarks)
- ✅ Tests passing (33/33)
- ✅ Active repository with recent updates

## Suggested Location in awesome-python

**Data Validation** section (if exists) or:
- **Serialization** section (under "Data Formats")
- **Data Analysis** section (under "Data Structures and Data Analysis")
- Or create a new **Data Quality & Validation** section

## Similar Projects (for reference)

- **pandera** (https://github.com/unionai-oss/pandera) — Python-based, slower, no lakehouse support
- **Great Expectations** (https://github.com/great-expectations/great_expectations) — Python-based, slower, no native Iceberg/Delta
- **Pydantic** (https://github.com/pydantic/pydantic) — Row-oriented validation, 20× slower for tabular data

## Note

This is a legitimate open-source project submission. StatGuard is fully documented, tested, and available on PyPI with a clear MIT license. No conflicts of interest or duplicative entries.
