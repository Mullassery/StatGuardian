# OKF Integration Guide for StatGuardian

**Status:** Phase 1a - Core Platform Integration  
**Version:** v2.1  
**Date:** 2026-07-20

---

## Overview

StatGuardian now natively supports Google's Open Knowledge Format (OKF) for storing and sharing data quality contracts, drift baselines, and validation rules as portable, shareable markdown documents.

This transforms StatGuardian from a **point solution** (validate data locally) into a **knowledge system** (build global library of quality standards).

---

## Strategic Value

### Before OKF
- Quality contracts live in code/configs (hard to share, version control messy)
- Drift baselines are computed per-run (no historical baseline library)
- Rule effectiveness is invisible (don't know which rules actually catch issues)
- Anomaly patterns are scattered (no central learning)

### After OKF
- Contracts stored as git-tracked markdown (shareable across org)
- Baseline distributions documented and versioned (community baseline library)
- Rule success rates tracked and visible (learn from history)
- Anomaly patterns collected and actionable (early warning system)

---

## Quick Start

### 1. Initialize OKF Contract Catalog

```python
from pathlib import Path
from statguardian.okf_contracts import OKFContractCatalog

# Create catalog (auto-creates directory structure)
catalog = OKFContractCatalog(Path("./quality_contracts"))
```

### 2. Save Data Quality Contract

```python
catalog.save_contract(
    contract_name="customers",
    schema={
        "customer_id": {"type": "integer", "nullable": False},
        "email": {"type": "string", "nullable": False},
        "created_at": {"type": "timestamp", "nullable": False},
    },
    rules=[
        {
            "name": "no_nulls",
            "type": "completeness",
            "severity": "error",
            "description": "No null values allowed in key columns",
        },
        {
            "name": "unique_email",
            "type": "uniqueness",
            "severity": "error",
            "description": "Email must be unique",
        },
    ],
    baseline={
        "row_count": 1000000,
        "null_percentage": 0.0,
        "mean_records_per_day": 5000,
    },
    metadata={"tags": ["crm", "production"], "owner": "analytics-team"},
)
```

### 3. Track Rule Effectiveness

```python
from statguardian.okf_contracts import OKFRuleEffectiveness

tracker = OKFRuleEffectiveness(Path("./quality_contracts"))

# Record rule execution
tracker.record_rule_execution(
    contract_name="customers",
    rule_name="unique_email",
    passed=False,  # Validation failed
    caught_issues=47,  # Found 47 duplicates
)

# Get historical success rate
success_rate = tracker.get_success_rate("customers", "unique_email")
print(f"Rule 'unique_email' catches issues {success_rate*100:.1f}% of the time")
```

### 4. Detect Anomaly Patterns

```python
from statguardian.okf_contracts import OKFAnomalyPattern

anomalies = OKFAnomalyPattern(Path("./quality_contracts"))

# Record detected anomaly
anomalies.record_anomaly(
    contract_name="customers",
    anomaly_type="drift",
    severity="warning",
    description="Column 'status' distribution shifted 12% compared to baseline",
    affected_rows=120000,
)

# Get recurring patterns
recurring = anomalies.get_recurring_anomalies("customers", min_occurrences=3)
for pattern in recurring:
    print(f"{pattern['anomaly_type']}: {pattern['occurrences']} times, "
          f"{pattern['total_affected_rows']} affected rows")
```

---

## Directory Structure

```
quality_contracts/
├── contracts/                    # Data quality contracts
│   ├── customers.md             # Customer schema + rules
│   ├── orders.md                # Order schema + rules
│   └── products.md              # Product schema + rules
├── baselines/                    # Statistical baselines (generated from contracts)
├── rules/                        # Rule execution history
│   ├── customers_no_nulls.md
│   ├── customers_unique_email.md
│   └── ...
└── anomalies/                    # Detected anomalies
    ├── customers_drift_*.md
    └── customers_duplicate_*.md
```

---

## OKF Document Format

### Contract Example (`contracts/customers.md`)

```yaml
---
type: data-contract
contract_name: customers
version: 1.0.0
schema:
  customer_id:
    type: integer
    nullable: false
  email:
    type: string
    nullable: false
  status:
    type: string
    enum: [active, inactive]
rules:
  - name: no_nulls
    type: completeness
    severity: error
  - name: unique_email
    type: uniqueness
    severity: error
baseline:
  row_count: 1000000
  columns: 4
  null_percentage: 0.0
tags: [crm, production]
owner: analytics-team
timestamp: 2026-07-20T12:30:00
---

# Data Quality Contract: Customers

## Schema
```json
{
  "customer_id": {"type": "integer", "nullable": false},
  "email": {"type": "string", "nullable": false}
}
```

## Quality Rules
- **no_nulls**: No null values allowed in key columns
- **unique_email**: Email addresses must be globally unique
```

---

## Integration with PyStreamMCP

StatGuardian's OKF contracts are **consumed by PyStreamMCP** for quality gates:

```python
# In PyStreamMCP query planning
quality_gate = statguardian_catalog.get_contract("customers")

if quality_gate:
    # Apply quality rules before including context in LLM prompt
    validation_result = validate_against_contract(context_data, quality_gate)
    if not validation_result.passed:
        # Replace with higher-quality source or cached results
        return fallback_strategy()
```

---

## Integration with PyReverseETL

StatGuardian validates data **before activation** in PyReverseETL:

```python
# In PyReverseETL activation pipeline
quality_contract = statguardian_catalog.get_contract("customer_segments")

result = validate_data(segment_data, quality_contract)
if result.passed:
    # Safe to activate to destination
    activate_to_destination(segment_data)
else:
    # Log anomaly, route to dead-letter queue
    record_quality_failure(result.violations)
```

---

## Use Cases

### 1. Contract Marketplace

Share validated contracts across organization:

```bash
# In git, quality_contracts/ is version-controlled
git add quality_contracts/contracts/customers.md
git commit -m "Update customer contract with new validation rules"
git push

# Other teams can now import this contract
from statguardian import load_contract_from_github("team/repo/customers.md")
```

### 2. Baseline Repository

Build community baselines for common data types:

```python
# Every data validation run updates baselines
updated_baseline = {
    "row_count": 1050000,
    "mean_value": 45.2,
    "std_deviation": 12.3,
    "p95_value": 68.1,
}
catalog.save_baseline("customers", updated_baseline)

# Future validations use this as reference
baseline = catalog.get_baseline("customers")
if current_mean > baseline["mean_value"] * 1.2:
    alert("Significant drift detected")
```

### 3. Rule Effectiveness Tracking

Learn which validation rules catch real issues:

```python
# Track success rate over time
success_rates = {}
for rule in contract.rules:
    rate = tracker.get_success_rate(contract.contract_name, rule.name)
    if rate:
        success_rates[rule.name] = rate

# Recommend high-value rules to other teams
high_value_rules = [r for r, rate in success_rates.items() if rate > 0.8]
```

### 4. Anomaly Early Warning

Detect patterns before data quality crisis:

```python
recurring = anomalies.get_recurring_anomalies("orders")

for pattern in recurring:
    if pattern["occurrences"] >= 5:
        send_alert(f"Critical pattern detected: {pattern['anomaly_type']} "
                  f"recurring {pattern['occurrences']} times")
```

---

## API Reference

### OKFContractCatalog

```python
from statguardian.okf_contracts import OKFContractCatalog

catalog = OKFContractCatalog(Path("./quality_contracts"))

# Search
contracts = catalog.search_contracts("customers")  # By name/tag
all_contracts = catalog.search_contracts("*")      # All

# Get specific contract
contract = catalog.get_contract("customers")

# Get baseline
baseline = catalog.get_baseline("customers")

# Save contract
path = catalog.save_contract(
    contract_name="...",
    schema={...},
    rules=[...],
    baseline={...},
    metadata={"tags": [...]}
)

# Update baseline
catalog.save_baseline("customers", new_baseline)

# Reload from disk
catalog.reload()
```

### OKFRuleEffectiveness

```python
from statguardian.okf_contracts import OKFRuleEffectiveness

tracker = OKFRuleEffectiveness(Path("./quality_contracts"))

# Record execution
tracker.record_rule_execution(
    contract_name="...",
    rule_name="...",
    passed=True/False,
    caught_issues=0
)

# Get success rate
rate = tracker.get_success_rate("customers", "unique_email")
```

### OKFAnomalyPattern

```python
from statguardian.okf_contracts import OKFAnomalyPattern

tracker = OKFAnomalyPattern(Path("./quality_contracts"))

# Record anomaly
tracker.record_anomaly(
    contract_name="...",
    anomaly_type="drift|duplicate|outlier|...",
    severity="warning|error|critical",
    description="...",
    affected_rows=0
)

# Get recurring patterns
recurring = tracker.get_recurring_anomalies("customers", min_occurrences=2)
```

---

## Testing

OKF contract tests are in `tests/test_okf_contracts.py`:

```bash
pytest tests/test_okf_contracts.py -v
```

18 tests covering:
- Contract loading and properties
- Catalog search and filtering
- Baseline management
- Rule execution tracking
- Anomaly detection and pattern recognition

---

## Next Steps

1. **Initialize catalog** in your data pipeline
2. **Export existing contracts** to OKF format
3. **Track rule effectiveness** for 2+ weeks
4. **Share high-value contracts** with other teams via git
5. **Monitor anomaly patterns** and create alerts

---

## References

- **Google OKF Spec:** https://github.com/GoogleCloudPlatform/knowledge-catalog
- **StatGuardian Docs:** https://github.com/Mullassery/Statguardian
- **Integration with PyStreamMCP:** See PyStreamMCP/OKF_INTEGRATION.md
