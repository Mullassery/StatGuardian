# StatGuardian Roadmap (v2.1 → v3.0+)

**Data Quality Engine for AI-Ready Pipelines**

Vision: Become the universal data quality layer that prevents silent failures in data pipelines and ensures LLM input/output quality.

---

## Integration Architecture

StatGuardian validates data at critical boundaries:
- **Data Ingestion** — Catch schema/type/range errors early
- **Pipeline Processing** — Drift detection, statistical anomalies
- **ML Input** — Pre-flight checks before model serving
- **LLM Context** (v2.2+) — Validate RAG retrieval quality
- **LLM Output** (v2.2+) — Factuality scoring (via openanchor bridge)
- **Data Warehouse** — Quality SLAs for downstream consumers

---

## Release Timeline

### ✅ v2.1.0 (June 2026) — CURRENT
**Status:** Stable, production-ready

**Core Features:**
- ✅ Schema validation (Pandera-like DSL)
- ✅ Data expectations & rules engine
- ✅ Statistical drift detection
- ✅ Anomaly detection (isolation forest, z-score)
- ✅ Audit logging (JSON-lines format)
- ✅ Slack/PagerDuty alerts
- ✅ 59+ tests

---

### 🟡 v2.2.0 (August 2026, 8 weeks) — LLM & RAG QUALITY
**New Integration:** openanchor v0.2+

**Features:**

1. **RAG Quality Validation** (new module: `quality/rag.py`)
   - Validate retrieval relevance (cosine similarity thresholds)
   - Check for hallucination markers (contradiction with source)
   - Measure context diversity (avoid redundant chunks)
   - Estimate answer groundedness

2. **LLM Input Quality Gates** (integration with openanchor)
   - Pre-flight validation of context before LLM call
   - Flag missing/null critical fields
   - Check schema against LLM expectations
   - Prevent garbage-in-garbage-out at prompt boundary

3. **LLM Output Validation**
   - Response schema validation (JSON structure)
   - Consistency checks (output matches input constraints)
   - Toxicity/safety scoring
   - Factuality markers (needs external API, configurable)

4. **Quality Metrics Dashboard**
   - Per-source quality scorecard
   - Trend analysis (quality over time)
   - SLA compliance tracking
   - Quality vs latency tradeoff visualization

5. **AI-Ready Contracts** (enhanced DSL)
   - Specify quality thresholds for AI systems
   - Contract violations trigger quality flags
   - Metadata: which fields are "critical for LLM"

**Tests:** 72 (13 new)  
**Deliverables:**
- `quality/rag.py` — RAG validation
- `quality/llm_gates.py` — input/output validation
- `integration/openanchor_bridge.py` — bi-directional feedback
- CLI: `statguardian quality-report` + dashboard endpoints

**Integration Points:**
```
Data Source
    ↓
StatGuardian (v2.2)
    ├→ Schema validation
    ├→ Drift detection
    └→ RAG quality checks
    ↓
RAG Pipeline → Retriever → Context Window
    ↓
OpenAnchor (v0.2)
    ├→ statguardian: "context quality = 0.92"
    └→ openanchor: quality score included in attribution
    ↓
LLM (with budget enforcement)
    ↓
Response Quality
```

---

### ✅ v2.3.0 (October 2026, 4 weeks) — OBSERVABILITY & EXPORT
**Dependencies:** openanchor 0.3+

**Features:**
1. **OpenTelemetry Export** (matching openanchor)
   - Quality metrics as OTEL spans
   - Integration with Datadog, Grafana, Honeycomb
   - Trace-based quality scoring

2. **Quality Lineage** (audit trail)
   - Track which quality checks passed/failed
   - Root cause analysis (which rule triggered?)
   - Historical quality trending

3. **Feedback Loop** (ML)
   - Collect user feedback on data quality
   - Retrain anomaly detector with feedback
   - Auto-adjust thresholds based on FP/FN rate

**Tests:** 81 (9 new)

---

### 📋 v3.0.0 (Q4 2026, 12 weeks) — ENTERPRISE & AI PIPELINES
**Dependencies:** openanchor 1.0+, PyStreamMCP 1.0+

**Features:**

1. **Multi-Tenant Governance**
   - Per-tenant quality SLAs
   - Quota-based quality checks (fair-share)
   - Quality budgets (like token budgets in openanchor)

2. **Active Learning for Quality**
   - Suggest new validation rules based on failures
   - Adaptive thresholds (learn from historical data)
   - Cost-aware validation (skip expensive checks if quality already high)

3. **LLM-Optimized Indexing** (partnership with PyStreamMCP)
   - Quality metrics influence retrieval ranking
   - High-quality sources prioritized in context selection
   - PyStreamMCP cost budget respects statguardian quality gates

4. **End-to-End Contracts**
   - Data quality SLA → LLM quality expectation → cost budget
   - Single "Quality-Cost-Latency" contract for entire pipeline
   - Automatic tradeoff recommendations

5. **Benchmarking & Comparison**
   - Compare quality across datasets
   - Vendor comparison (different data providers)
   - Best-in-class metrics per data domain

**Tests:** 95+ (14+ new)  
**Deliverables:**
- Full enterprise features
- Multi-tenant example deployment
- End-to-end tutorial: data quality → LLM quality → cost optimization

---

## Cross-Project Dependencies

```
                  v2.2+          v2.3+                v3.0+
                    ↓              ↓                     ↓
StatGuardian ←→ OpenAnchor → PyStreamMCP ← statguardian
                    ↑              ↑                     ↑
                 (quality)    (cost+quality)        (integration)
                 insights      enforcement          maturity
```

**Data Flow v3.0:**
```
RAG Source
    ↓
StatGuardian: "quality = 0.92, has hallucination markers"
    ↓
PyStreamMCP: "retrieve 5 chunks, cost budget = 2K tokens"
    ↓
OpenAnchor: "cost vs quality tradeoff: 0.88 quality at -20% cost"
    ↓
User chooses: "prefer 0.88 quality, lower cost"
    ↓
StatGuardian + PyStreamMCP + OpenAnchor execute optimized plan
```

---

## Success Metrics

| Milestone | Metric | Target |
|-----------|--------|--------|
| v2.2 | LLM-related tests | 15+ |
| v2.2 | openanchor integrations working | 3+ |
| v2.3 | OTEL export verified | 2+ backends |
| v3.0 | Multi-tenant deployments | 1+ |
| v3.0 | PyStreamMCP + OpenAnchor integration tests | 10+ |

---

## Key Decisions

1. **v2.2 is the LLM pivot** — Without LLM quality gates, statguardian remains a generic pipeline tool. v2.2 makes it LLM-first.

2. **openanchor integration (v2.2) is critical** — Bidirectional feedback loop ensures quality insights inform cost decisions.

3. **v3.0 maturity requires PyStreamMCP 1.0** — Multi-tenant + end-to-end contracts need stable cost budget APIs.

---

## Notes

- Statguardian v2.1 is production-ready as-is for traditional pipelines.
- v2.2 adds the "LLM quality" dimension that's unique in the market.
- v3.0 unlocks the full synergy: quality + cost + latency all governed together.
