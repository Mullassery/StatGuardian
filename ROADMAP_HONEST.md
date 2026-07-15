# Statguardian Roadmap

**Current Version:** v1.0.0  
**Last Updated:** July 2026  
**Status:** Production-ready for core validation; advanced features in development

---

## Known Limitations (v1.0.0)

### 🔴 Blocking Issues
None identified.

### 🟡 Experimental Features
- **Iceberg support:** Listed in README but **not implemented**
  - [ ] Snapshot ID time-travel not wired
  - [ ] Iceberg-specific schema features not tested
  - **Impact:** Remove from README; document workaround (export to Parquet first)
  - **Fix timeline:** v1.1.0 (Q3 2026)

- **Custom DSL rules:** Limited to common patterns
  - ✅ Basic rules (not_null, unique, positive, max/min)
  - ❌ Complex regex patterns
  - ❌ Multi-column rules
  - ❌ Time-series rules
  - **Impact:** Use Python fallback for complex validation
  - **Fix timeline:** v1.2.0 (Q4 2026)

### 🟢 Shipping/Stable (v1.0.0)
- ✅ Contract DSL (basic rules)
- ✅ Schema validation (Parquet, CSV, JSON, Avro, Arrow)
- ✅ Distribution drift detection
- ✅ Delta Lake time-travel validation
- ✅ Polars DataFrame validation
- ✅ Quality scoring

---

## 🔒 Security Issues (See SECURITY_AUDIT.md)

### CRITICAL — v1.0.1
- [ ] **Audit SQL injection patterns** (9 instances found)
- [ ] **Run Rust safety audit** (Cargo audit, miri)

### HIGH — v1.0.1
- [ ] **Pin all dependency versions** (0 pinned, 39 floating)

### HIGH — v1.1.0
- [ ] **Secure AWS credential handling guide**
- [ ] **DSL input validation** (prevent DoS with malformed contracts)

---

## TODOs in Code
None found in Python; Rust safety needs audit.

---

## Roadmap

### v1.0.1 (Q3 2026) — Documentation + Examples
- [ ] Add more DSL examples
- [ ] Document Iceberg limitation in README
- [ ] Add performance benchmarks

### v1.1.0 (Q3 2026) — Iceberg Support
- [ ] Iceberg snapshot time-travel
- [ ] Partition pruning optimization
- [ ] Iceberg-specific schema validation

### v1.2.0 (Q4 2026) — Advanced DSL Rules
- [ ] Regex pattern validation
- [ ] Multi-column rules (e.g., foreign key checks)
- [ ] Time-series anomaly detection
- [ ] Custom Python rule integration

### v1.3.0 (Q4 2026) — Performance
- [ ] >1B row dataset validation (currently untested)
- [ ] Distributed validation (Spark/Dask)
- [ ] Streaming validation (Kafka topics)

### v2.0.0 (Q1 2027) — Real-Time Monitoring
- [ ] Continuous validation hooks
- [ ] Alerting on contract violations
- [ ] Historical violation tracking
- [ ] Automated remediation suggestions

---

## Not Planned
- GUI for DSL writing (CLI only)
- Direct Snowflake/BigQuery validation (use Delta Lake export first)
