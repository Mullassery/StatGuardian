# StatGuardian Prioritization Framework

**Date:** 2026-07-17  
**Purpose:** Make consistent, defensible roadmap decisions  
**Audience:** Product, Engineering, Executive Leadership

---

## Priority System

### P0: CRITICAL (Do First)
**Definition:** Foundational features that block everything else

**Characteristics:**
- Unlocks multiple P1 and P2 features
- Creates network effects
- Enables core business model
- Must complete before scaling

**Current P0 Items:**
1. **v2.2: Lineage** ✅ DONE
2. **v3.0: Streaming validation** (Sep 25 - Nov 6)

**Decision Rule:** If it blocks 3+ other features → P0

---

### P1: HIGH (Do Soon)
**Definition:** Core differentiators that drive adoption

**Characteristics:**
- Enables significant business value
- Clear market demand
- Doesn't block other features
- Can parallelize with P0

**Current P1 Items:**
1. **v2.3: ML drift detection** (Aug 21 - Sep 18)
2. **v5.0: Data contracts** (Feb 28 - Apr 20)
3. **v6.0: Autonomous agents** (Q3 2027+)

**Decision Rule:** If it differentiates from competitors → P1

---

### P2: MEDIUM (Do Next)
**Definition:** Important but not urgent features

**Characteristics:**
- Nice to have, not must-have
- Can wait 1-2 quarters
- Moderate business value
- Depends on P0/P1 completion

**Current P2 Items:**
1. **v3.1: SLA enforcement** (Nov 7 - Dec 5)
2. **v4.0: Automated remediation** (Dec 6 - Jan 30)
3. **v5.1: Enterprise governance** (Apr 20 - Jun 15)
4. **DBT test generation** (Dec 6 - Feb 28)

**Decision Rule:** If it can wait 1 quarter without losing customers → P2

---

### P3: LOW (Do Later)
**Definition:** Nice-to-have features with minimal urgency

**Characteristics:**
- Low market demand
- Long-tail use cases
- <5% customer requests
- Can defer indefinitely

**Examples:** Advanced dashboards, API v2, client SDKs

**Decision Rule:** If <10% of customers ask for it → P3

---

## Prioritization Scorecard

Use this 5-factor framework to score new features:

### Factor 1: Strategic Alignment (0-30 points)
**Question:** Does this advance core product vision?

**Vision Pillars:**
- Understand (20 pts): Lineage, schemas, changes
- Predict (20 pts): ML, anomalies, trends
- Act (20 pts): Real-time validation, automation
- Ecosystem (10 pts): Integrations, extensibility

**Scoring:**
- Pillar 1 aligned: 15 pts
- Pillar 2 aligned: 10 pts
- Pillar 3 aligned: 5 pts
- No alignment: 0 pts

**Example:**
- v3.0 (streaming): Pillar 3 = 20 pts
- DBT tests: Pillar 1 + Pillar 3 = 20 pts

---

### Factor 2: Market Demand (0-30 points)
**Question:** How many customers need this?

**Scoring:**
- 50%+ of customers asking: 30 pts
- 25-50% asking: 20 pts
- 10-25% asking: 10 pts
- <10% asking: 0 pts

**Data Sources:**
- Customer surveys
- GitHub issues / feature requests
- Sales pipeline conversations
- Usage analytics

**Example:**
- Streaming validation: 40% asking = 20 pts
- ML drift detection: 60% asking = 30 pts
- Advanced dashboards: 5% asking = 0 pts

---

### Factor 3: Competitive Pressure (0-20 points)
**Question:** What if we don't do this?

**Scoring:**
- Direct competitor has feature: 20 pts
- Competitor roadmapped, not yet shipped: 10 pts
- Indirect competitor has feature: 5 pts
- No competitor pressure: 0 pts

**Competitive Landscape:**
- **Great Expectations:** Testing framework (strong), but no streaming
- **Soda:** Observability focus, but no automation
- **Monte Carlo:** Anomaly detection, but no contracts
- **Our advantage:** Real-time + ML + Contracts (defensible)

**Example:**
- Streaming: Competitors weak here = 5 pts (low pressure)
- ML drift: Monte Carlo has it = 10 pts
- Contracts: No one has it = 0 pts (advantage)

---

### Factor 4: Engineering Effort (0-20 points)
**Question:** How hard is this to build? (Inverse scoring)

**Scoring:**
- <2 weeks: 20 pts (easy)
- 2-4 weeks: 15 pts
- 4-8 weeks: 10 pts
- 8-12 weeks: 5 pts
- 12+ weeks: 0 pts

**Rationale:** Easy wins should rank higher

**Example:**
- SLA monitoring: 4 weeks = 10 pts
- DBT test generation: 12 weeks = 5 pts
- ML drift detection: 4 weeks = 10 pts

---

### Factor 5: Dependencies & Blocking (0-10 points)
**Question:** Does this unblock other work?

**Scoring:**
- Unblocks 3+ features: 10 pts
- Unblocks 2 features: 7 pts
- Unblocks 1 feature: 5 pts
- No blocking value: 0 pts

**Example:**
- v3.0 (streaming): Unblocks v3.1, v4.0, v5.0 = 10 pts
- v2.3 (ML): Unblocks v4.0 = 7 pts
- v5.0 (contracts): Unblocks v5.1 = 7 pts

---

## Scoring Examples

### v3.0: Streaming Validation
```
Strategic alignment: 20 pts (Pillar 3: Act)
Market demand:      20 pts (40% asking)
Competitive:        5 pts (low pressure, we lead here)
Engineering effort: 10 pts (6 weeks, complex but doable)
Blocking value:     10 pts (unblocks v3.1, v4.0, v5.0)
─────────────────────────────
TOTAL:              65 pts → P0 (CRITICAL)

Decision: START IMMEDIATELY (Sep 25)
```

### v2.3: ML Drift Detection
```
Strategic alignment: 20 pts (Pillar 2: Predict)
Market demand:      30 pts (60% asking)
Competitive:        10 pts (Monte Carlo has it)
Engineering effort: 10 pts (4 weeks)
Blocking value:     7 pts (unblocks v4.0)
─────────────────────────────
TOTAL:              77 pts → P1 (HIGH)

Decision: START IMMEDIATELY (Aug 21)
```

### v5.1: Enterprise Governance
```
Strategic alignment: 10 pts (Pillar 4: Ecosystem)
Market demand:      10 pts (15% asking, enterprise-only)
Competitive:        5 pts (Monte Carlo has basic)
Engineering effort: 5 pts (8 weeks, complex)
Blocking value:     0 pts (doesn't unblock anything)
─────────────────────────────
TOTAL:              30 pts → P2 (MEDIUM)

Decision: DEFER TO Q2 2027
```

### Advanced Analytics Dashboard
```
Strategic alignment: 5 pts (Pillar 3: Act, nice-to-have)
Market demand:      5 pts (5% asking)
Competitive:        0 pts (not differentiated)
Engineering effort: 0 pts (12+ weeks)
Blocking value:     0 pts (nobody waits on this)
─────────────────────────────
TOTAL:              10 pts → P3 (LOW)

Decision: DEFER TO Q3 2027+
```

---

## Priority Trade-offs

### When to Upgrade a Feature
Move P2 → P1 if ANY of these are true:
- Customer churn risk (top 10 customer about to leave)
- Competitive emergency (competitor ships breakthrough feature)
- Market shift (new category emerging, e.g., streaming data)
- Strategic acquisition opportunity

### When to Downgrade a Feature
Move P1 → P2 if:
- Engineering estimates increase 2x+
- Market demand drops (fewer customers asking)
- Competitive pressure fades
- Blocker feature delayed beyond 1 quarter

### When to Freeze/Kill a Feature
- Low score (<20 pts) AND no strategic reason
- Engineering effort > 12 weeks AND market demand < 25%
- Superseded by newer feature with same benefit

---

## Real Decision Examples

### Decision 1: DBT Test Generation (Dec 6 Start)
**Proposal:** Generate dbt tests automatically (12-week feature)

**Scorecard:**
- Strategic: 20 pts (Pillar 1: Understand)
- Market: 25 pts (30% asking)
- Competitive: 0 pts (we're first!)
- Effort: 5 pts (12 weeks)
- Blocking: 7 pts (unblocks v5.0)
**Total: 57 pts → P2**

**Decision:** Parallelize with v3.0 (not on critical path)
**Timing:** Start Dec 6, finish Feb 28 (8-week engineering window)
**Rationale:** Differentiator (only us), but v3.0 more urgent

---

### Decision 2: Advanced Dashboards (Rejected)
**Proposal:** Build Grafana-like dashboard for quality trends

**Scorecard:**
- Strategic: 5 pts (nice UI, but core feature is the data)
- Market: 5 pts (5% asking, mostly "nice to have")
- Competitive: 0 pts (not differentiating)
- Effort: 0 pts (12+ weeks, complex)
- Blocking: 0 pts
**Total: 10 pts → P3**

**Decision:** DEFER (push to Q3 2027)
**Rationale:** Low ROI, long tail use case, doesn't move needle on adoption
**Alternative:** Ship simple dashboards in v3.1 (basic charts only)

---

### Decision 3: Multi-Tenant Support (Rejected)
**Proposal:** Support multiple tenants in single deployment (for SaaS)

**Scorecard:**
- Strategic: 0 pts (not on vision, infrastructure-focused)
- Market: 15 pts (8% asking, mostly enterprise)
- Competitive: 0 pts
- Effort: 0 pts (12+ weeks, fundamental rearchitect)
- Blocking: 0 pts
**Total: 15 pts → P3**

**Decision:** DEFER until Enterprise Edition strategy is clear (Q2 2027)
**Rationale:** Premature. Ship open-source v1-v5 first, then plan SaaS in 2027

---

## How to Re-prioritize Mid-Stream

### Quarterly Review Process

**Every quarter (Jan, Apr, Jul, Oct):**

1. **Score all features** using scorecard (Friday)
2. **Identify movers** (score changed by 10+ pts)
3. **Discuss trade-offs** (Monday, 2-hour review)
4. **Update roadmap** (Tuesday)
5. **Communicate changes** to team (Wednesday)

### Decision Criteria
**Change priority only if:**
- Score changes by ≥10 points, OR
- Customer churn risk, OR
- Competitive emergency, OR
- Resource constraints force replan

### Example: Mid-Q4 Replan
**Scenario:** Monte Carlo announces streaming validation feature

**Action:**
- Competitive pressure: +10 pts
- v3.0 score: 65 → 75 pts (even more critical)
- **Decision:** Add 1 engineer to v3.0, accelerate from Nov 6 → Oct 20

**Communication:** "Competitive pressure increased, pulling forward v3.0 delivery"

---

## P0 vs P1 vs P2 by Timeframe

### Right Now (Next 4 Weeks)
**Only P0 (Critical):**
- v2.3: ML Detection ✅ (Aug 21 start)

**Why:** Need focused effort on this before v3.0

---

### Next Quarter (4-12 Weeks)
**P0 + P1:**
- v3.0: Streaming ✅ (P0, Sep 25 start)
- v2.3: Finishing (Aug 21-Sep 18)

**Parallel (not on critical path):**
- DBT tests research (1 engineer)

---

### Next 6 Months (12-26 Weeks)
**P0 + P1 + P2:**
- v3.1: SLA (P2)
- v4.0: Automation (P2)
- DBT tests (P2)
- v5.0: Contracts (P1, prep)

---

### Next 12 Months (26+ Weeks)
**All priorities + future work:**
- v5.0: Contracts (P1)
- v5.1: Governance (P2)
- v6.0: Autonomous (P1, future)

---

## FAQ: How We Use This

**Q: Can we prioritize a customer request higher?**
A: Yes, if scoring justifies it (customer churn = priority upgrade). But be honest about trade-offs.

**Q: What if two P1 items both need to ship?**
A: Parallelize if possible. If not, rescore and choose winner.

**Q: Can we build P3 features?**
A: Only if spare capacity exists (no impact on P0/P1). Example: Bug fixes, technical debt.

**Q: How do we know the scorecard is working?**
A: Review quarterly. If features we prioritized as P0 got abandoned, re-calibrate.

---

## Scorecard Template (For Future Use)

```
Feature Name: ___________________________
Proposer: ___________________________
Date: ___________________________

Strategic Alignment (0-30):
  ☐ Understand (20)  ☐ Predict (20)  ☐ Act (20)  ☐ Ecosystem (10)
  Score: ___

Market Demand (0-30):
  ☐ 50%+ (30)  ☐ 25-50% (20)  ☐ 10-25% (10)  ☐ <10% (0)
  Score: ___

Competitive Pressure (0-20):
  ☐ Direct competitor (20)  ☐ Competitor roadmapped (10)
  ☐ Indirect competitor (5)  ☐ None (0)
  Score: ___

Engineering Effort (0-20):
  ☐ <2 weeks (20)  ☐ 2-4 weeks (15)  ☐ 4-8 weeks (10)
  ☐ 8-12 weeks (5)  ☐ 12+ weeks (0)
  Score: ___

Blocking Value (0-10):
  ☐ Unblocks 3+ (10)  ☐ Unblocks 2 (7)  ☐ Unblocks 1 (5)  ☐ None (0)
  Score: ___

─────────────────
TOTAL: ___

PRIORITY:
  ☐ P0 (65-110)  ☐ P1 (45-64)  ☐ P2 (25-44)  ☐ P3 (<25)
```

---

## Success: How Prioritization Drives Outcomes

**If we execute this roadmap (P0 → P1 → P2):**

- ✅ v2.3: 50+ companies using ML detection (Oct 2026)
- ✅ v3.0: 30+ streaming pipelines monitored (Nov 2026)
- ✅ v4.0: 80% reduction in on-call burden (Feb 2027)
- ✅ v5.0: 60% of users with data contracts (Apr 2027)
- ✅ v5.1: Enterprise customers in production (Jun 2027)

**If we ignore priorities (random backlog):**

- ❌ Ship features nobody asked for
- ❌ Fail to unblock dependent work
- ❌ Competitors ship faster
- ❌ Roadmap becomes unmaintainable

---

## Summary

**Prioritization Framework:**
- P0: 65+ pts (foundational, blocks 3+ features)
- P1: 45-64 pts (differentiators, market demand)
- P2: 25-44 pts (important but not urgent)
- P3: <25 pts (nice-to-have, defer indefinitely)

**Current Plan:**
- P0: v2.3 (DONE), v3.0 (NEXT)
- P1: v2.3, v5.0, v6.0
- P2: v3.1, v4.0, v5.1, DBT tests

**Review:** Quarterly (Jan, Apr, Jul, Oct)

**Trade-offs:** Explicit, scored, defensible
