# Competitive intelligence in roadmap and upgrade work

**Policy date:** 2026-10-19

Competitive findings are an input to product decisions, not a one-off report.
Every pending roadmap item, new feature, and routine dependency upgrade should
check the repository's [`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md) before work
starts, and record the decision in the PR or changelog.

## Required check before building

1. **Read `AUDIT_FINDINGS.md`.** Find the relevant competitor row and the
   ranked next actions. If the finding is stale, update it with a dated source.
2. **Name the competitor and the user problem.** A feature is not justified by
   "competitor has it". State the workflow it unlocks, who needs it, and how
   often. Mark unverified demand as a hypothesis.
3. **Prefer reuse over new code.** Check whether an existing Mullassery repo
   already provides the capability (see `BUNDLES.md` in the profile repo and
   the bundle role file in this repo, when present). Typical reuse:
   - crawling/fetching/robots/SSRF → **MudFish**
   - architecture maps / PR diffs → **CodeAtlas**
   - data-quality contracts / validation → **StatGuardian**
   - token counting / cost tables → **PyTokenCalc**
   - dependency graph / advisories / SBOM → **PyDeepMesh**
   - segmentation / CLV / churn → **ClusterAudienceKit**
   - activation / lineage / PII → **PyReverseETL**
   - fleet state sync → **SyncFleet** (only after conflict resolution)
4. **Choose one disposition:** build, integrate, defer, or reject. Write it
   down. A rejection with a reason is a successful outcome.
5. **For routine upgrades**, check whether the upgrade closes a known
   competitor gap or breaks a differentiated capability. Run the tests that
   cover the affected path; do not merge a green build that silently changes a
   user-visible contract.

## PR / changelog checklist

- [ ] Competitor and source URL/date recorded (or "no direct competitor").
- [ ] User impact and frequency stated; unverified demand labelled.
- [ ] Reuse check done against sibling repos/bundles.
- [ ] Failure mode and rollback considered (especially auth, PII, cost).
- [ ] Docs/roadmap status updated honestly.
- [ ] Tests or a manual verification command included.

## Review cadence

- Refresh the competitor row when a release changes the capability.
- Re-check the full `AUDIT_FINDINGS.md` at least quarterly and before a major
  release.
- If a finding is wrong or stale, correct it in the same PR that relies on it.

## Non-goals

- Copying features for parity.
- Adding a dependency just because a competitor uses it.
- Treating star counts or download counts as proof of user need.

## OSS gap analysis — 2026-10-24

**What matters most here:** failed-row samples and warehouse pushdown. Counts without examples make debugging slow, and materializing results locally defeats warehouse-scale checks; add row samples first, then pushdown if users need it.

Scope: open-source data-quality and validation tools that overlap StatGuardian's
schema/quality/drift/anomaly checks. Dates and licenses checked against primary
repositories on 2026-10-24; star counts are intentionally excluded.

### Comparable OSS projects

- **Great Expectations** — `https://github.com/fivetran/great_expectations`;
  Apache-2.0; v1.24.1 (2026-10-22). Expectation suites, SQL/Spark datasources,
  Data Docs, and the broadest orchestrator integration set.
- **Soda Core** — `https://github.com/sodadata/soda-core`; Elastic License 2.0;
  v4.28.0 (2026-10-20). SodaCL checks run in the warehouse. The Elastic license
  is not OSI-approved and restricts offering it as a managed service.
- **Pandera** — `https://github.com/unionai-oss/pandera`; MIT; v0.34.1
  (2026-10-06). Dataframe schema validation with pandas/Polars support; no
  warehouse runner or ops UI.
- **Deequ** — `https://github.com/awslabs/deequ`; Apache-2.0; v2.1.0
  (2026-09-16). Spark-native constraint verification and profiling.

### Capability comparison

| Capability | StatGuardian | Great Expectations | Soda Core | Pandera | Deequ |
| --- | --- | --- | --- | --- | --- |
| Declarative rule DSL | Yes | Yes | Yes | Yes | Yes |
| Schema and dtype validation | Yes | Yes | Yes | Yes | Yes |
| Statistical drift detection (PSI/KS) | Yes | Partial | Partial | No | Partial |
| Anomaly rules or scoring | Yes | Partial | Partial | No | Partial |
| Pandas/Polars dataframes | Yes | Partial | No | Yes | No |
| Native Delta/Iceberg without Spark | Yes | Partial | No | No | No |
| SQL/warehouse pushdown | No | Yes | Yes | No | No |
| Spark-native execution | Partial | Yes | No | No | Yes |
| Failed-row diagnostics | No | Yes | Yes | Partial | Partial |
| Orchestrator integrations | Partial | Yes | Yes | Partial | Partial |
| Hosted/Data Docs UI | No | Yes | Partial | No | No |

### Gaps that matter (ranked)

1. **No warehouse-side execution.** `execute_sql` in
   `python/statguardian/_connectors.py` materializes query results locally, so
   checks run in Python rather than next to warehouse data. GX and Soda push
   checks down. Effort: L. Build only if warehouse-scale users are the target.
2. **No failed-row samples.** Reports expose counts and statistics, not the
   rows that violated a rule (`crates/statguardian-metrics/src/report.rs`).
   GX, Soda, and Pandera all make failing examples visible. Effort: M. Build;
   this is the highest-value debugging gap.
3. **Narrower check library and no suite importer.** There is no importer for
   GX/Soda/Pandera suites, so migration requires rewriting expectations.
   Effort: M. Build a small compatibility layer, not full parity.
4. **Packaging and release hygiene.** PyPI has only a macOS arm64 wheel and the
   license field is `None`; GitHub releases stop before the current version and
   no publish workflow is committed. Effort: S/M. Build; adoption is blocked
   before capability comparisons matter.
5. **No Data Docs/docs portal.** GX's generated documentation is a review and
   stakeholder workflow we do not serve. Effort: L. Defer until users ask for
   hosted reports rather than JSON/Prometheus.
6. **Spark-native execution.** Deequ is Spark-native; our Spark path is
   connector-based. Effort: L. Defer unless Spark users report data-movement
   pain.
7. **Orchestrator breadth.** Only dbt and Airflow integration directories are
   present; GX supports more schedulers. Effort: M. Integrate on demand.

### Where we are differentiated

- A Rust engine embedded in the process, with no JVM, server, or cloud account.
- One `.sg` DSL covers schema, quality, statistics, drift, anomalies, and
  streaming rules instead of separate APIs.
- Polars no-copy execution plus Narwhals, with native Delta/Iceberg reads without Spark.
- PSI/KS drift, anomaly rules, and Prometheus output in the same binary.
- Apache-2.0 licensing versus Soda Core's Elastic License 2.0.

### Disposition

**Build.** Prioritize failed-row diagnostics, suite import for the most common
GX/Soda checks, and packaging/release automation. Defer Data Docs and a
Spark-native rewrite until demand is evidenced. Reject claims of full GX/Soda
parity: the differentiated value is a fast embedded engine, not the largest
expectation catalog.

### Sources

- https://github.com/fivetran/great_expectations/releases/tag/1.24.1
- https://raw.githubusercontent.com/fivetran/great_expectations/develop/LICENSE
- https://github.com/sodadata/soda-core/releases/tag/v4.28.0
- https://raw.githubusercontent.com/sodadata/soda-core/main/LICENSE
- https://github.com/unionai-oss/pandera/releases/tag/v0.34.1
- https://github.com/awslabs/deequ
- Repository evidence: `README.md`, `AUDIT_FINDINGS.md`, `docs/STATUS.md`, `docs/COMPARISON.md`, `python/statguardian/_connectors.py`.
