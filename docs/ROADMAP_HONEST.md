# statguardian — Honest Roadmap

**Last audited:** 2026-09-21
**Current version:** 2.6.0 (`Cargo.toml`, `pyproject.toml`)

This document was previously stale (last substantively updated at v1.0.0 /
July 2026) and, in several places, described features as "not implemented"
that have since shipped. It has been rewritten against the current codebase.
For a canonical view of security status specifically, see
[`SECURITY_AUDIT.md`](SECURITY_AUDIT.md) — that document is kept current
release-to-release; this one is a point-in-time snapshot and can drift again.

`docs/ROADMAP.md` and `docs/ROADMAP_INTEGRATED.md` also exist in this
directory. Both contain aspirational and, in places, outright fabricated
"shipped" claims (a REST API, n8n/Power Automate/Temporal/Airflow
integrations, Slack/PagerDuty alerting, JSON-lines audit logging — none of
which exist anywhere in this codebase). Both now carry a disclaimer banner
pointing back here. This file is the one to trust for "what's actually
built."

---

## What's actually shipped and tested

Verified by reading the source, not by trusting prior roadmap claims.

- **`.sg` contract DSL** — pest grammar (`crates/statguardian-core/src/parser/grammar.pest`)
  supports schema types, `not_null`/`unique`/`primary_key`, `enum=[...]`,
  `regex=`, `between(min,max)`, `foreign_key(table.column)`, quality
  thresholds (`completeness(...)`), and anomaly blocks
  (`detect_outliers`/`detect_duplicates`, `@blocking`). Compiled via
  `DataContract.from_dsl()`/`from_file()`. Exercised by
  `crates/statguardian-core`'s own parser tests and `examples/*.sg`.
- **Schema validation, drift detection (PSI/KS), anomaly detection
  (isolation forest, z-score, seasonal)** — real Rust implementations in
  `statguardian-validators` / `statguardian-stats`, not stubs.
- **Delta Lake and Apache Iceberg table validation** — real readers
  (`crates/statguardian-io/src/iceberg.rs` and the Delta path), not "planned."
  The previous version of this document said Iceberg was "listed in README
  but not implemented" — that was wrong even at the time this file claims
  (v1.0.0), and is definitely wrong now.
- **SQL, Spark, and cloud-object-storage sources** (`execute_sql`,
  `execute_spark`, `execute_cloud`) — real, not stubs; see
  `statguardian-io/src/sql.rs`. `execute_sql()` covers Postgres, MySQL,
  SQLite, BigQuery, Snowflake, Redshift, Databricks, ClickHouse, Trino, and
  DuckDB (`pip install "statguardian[sql-duckdb]"`) via `connectorx`.
- **Data lineage tracking** (`get_lineage_graph`, `save_lineage_version`,
  `get_lineage_version`, `get_lineage_history`) — real, was previously fully
  stubbed and fixed (see git history, commit `fbb41f3`).
- **Custom Python validators + report merging** (`@statguardian.validator`,
  `run_custom_validators`, `merge_violations`) — real.
- **dbt integration** (`integrations/dbt-statguardian`, `statguardian dbt
  validate`) — real, ships its own integration tests.
- **PII detection** (`python/statguardian/_pii.py`) and **HTML report
  generation** (`python/statguardian/_html.py`) — real; not mentioned
  accurately in any of the other roadmap docs.
- **CLI**: `statguardian validate`, `statguardian check`, `statguardian dbt
  validate` (`python/statguardian/_cli.py`). There is **no** `detect-drift`,
  `detect-anomalies`, or `check-schema` subcommand — `docs/ROADMAP.md`
  claims these exist; they do not as of this audit. `statguardian --version`
  did not exist before this audit (the CLI had no version flag at all,
  despite `docs/CLI.md`'s Installation section instructing users to run it)
  — added during this pass.

## What does not exist, despite prior docs claiming otherwise

- **REST API** — no HTTP server, no web framework dependency, no port-8008
  anything. `docs/ROADMAP.md` marked this shipped; it never was.
- **Workflow tool integrations** (n8n, Power Automate, Temporal, Airflow,
  UiPath) — zero code, zero dependencies, zero docs referencing any of these
  beyond the roadmap bullet points themselves.
- **Slack/PagerDuty alerting** and **JSON-lines audit logging** —
  `docs/ROADMAP_INTEGRATED.md` marked both shipped in "v2.1.0 CURRENT"; no
  such code exists.
- **LLM/RAG quality gates, "openanchor" integration** — entirely
  speculative content in `docs/ROADMAP_INTEGRATED.md`'s "v2.2.0" section;
  no code, no dependency.

## Known regressions and open engineering work

- **HIGH — `regex=`, `enum=[...]`, and anomaly `method=` DSL constraints are
  silently broken: they never match anything, for every user, always.**
  Found by actually running `statguardian validate` against a real `.sg`
  contract and a real Parquet file during this audit (2026-09-21) — not
  previously documented anywhere. Root cause: pest's `string_literal` rule
  (`crates/statguardian-core/src/parser/grammar.pest:10`,
  `@{ "\"" ~ inner_str ~ "\"" }`) is atomic, so `.as_str()` on a matched
  token returns the literal text **including the surrounding double quotes**.
  One call site already knows and handles this correctly —
  `parse_literal_value()` in `crates/statguardian-core/src/parser/mod.rs`
  (~line 253) explicitly strips exactly one leading/trailing `"` with a
  comment explaining why. Three other call sites in the same file do not:
  - `parse_constraint()`'s `Rule::regex_constraint` arm (~line 121-126):
    builds `Constraint::Regex { pattern }` from the raw quoted string. Every
    compiled regex therefore requires a literal `"` character at both ends
    of the value, which no real input has — **100% of non-null values in any
    `regex=`-constrained column are always reported as violations**, even
    values that plainly match the pattern. Verified: contract
    `email: string, regex="^[^@]+@[^@]+\.[^@]+$"` against
    `a@x.com`/`b@y.com`/`c@z.com`/`d@w.com` reported all 4 as
    `"4 value(s) don't match '"^[^@]+@[^@]+\.[^@]+$"' in 'email'"` — note the
    embedded literal quotes in the error message itself, the tell-tale sign.
  - `parse_constraint()`'s `Rule::enum_constraint` arm (~line 159-162): same
    bug for enum values. **Every value in an `enum=[...]`-constrained column
    is always reported as "not in allowed set,"** including values that are
    legitimately in the enum. Verified: contract
    `country: string, enum=["US","UK","DE","FR","CA","AU","JP"]` against
    values `US`/`UK`/`DE`/`FR`/`ZZ` reported **all 5** rows as violations
    (only `ZZ` should have failed).
  - `parse_anomaly_rule()`'s `named_arg` loop (~line 386-389): same bug for
    named-argument string values, e.g. `method="iqr"` on `detect_outliers`,
    `pattern="..."` on `detect_pattern_breaks`. The quoted value flows
    through `crates/statguardian-core/src/compiler/mod.rs:123-126`
    (`rule.args.get("method")`) into
    `crates/statguardian-engine/src/batch.rs`'s
    `match method { "iqr" => ..., "zscore" => ..., _ => vec![] }` (~line
    446-478) — since `method` is actually the 5-character string `"iqr"`
    (quotes included), it never matches either arm and silently falls
    through to `_ => vec![]`. **`detect_outliers(col, method="iqr"|"zscore")`
    always reports zero outliers, with no error, regardless of the actual
    data.** Verified: a column with an obvious IQR outlier (130 among
    22/25/31/40) and a column with an obvious z-score outlier (1.5 among
    0.1/0.3/0.5/0.9) both produced zero `outlier_detection` violations.
    `detect_pattern_breaks`'s `pattern=` argument shares the exact same code
    path (`compiler/mod.rs:144`) and is almost certainly broken the same
    way, though not separately verified live.

  **Impact:** this breaks three separately-advertised, headline DSL
  features — `enum=`, `regex=`, and anomaly-detection `method=` — including
  the exact examples used in this project's own README ("30-Second Start"
  uses `enum=["pending","paid","cancelled"]`; "Schema validation" uses
  `regex="^[^@]+@[^@]+\.[^@]+$"`). Anyone following the README quickstart
  literally gets either guaranteed false-positive violations on every row
  (`enum=`/`regex=`) or anomaly detection that silently never fires
  (`method=`), with no error or warning either way. None of the 118 passing
  Rust tests or the Python test suite caught this — there is a real
  test-coverage gap: nothing exercises `enum=`/`regex=`/`method=` end-to-end
  against data expected to both pass and fail one of these checks. Not
  fixed here — the correct fix (mirror `parse_literal_value`'s quote-strip
  at the three broken call sites) is small, but doing it properly needs
  regression tests added for all three constructs plus a full
  clippy/fmt/test cycle, which deserves a dedicated, focused session rather
  than a fix folded into a docs pass.

- **`StreamingBatcher` bounded-memory regression** (introduced 2026-09-14 by
  the required `polars` 0.44→0.55 security upgrade) — Polars removed its
  public incremental/batched CSV and Parquet reader API, so
  `StreamingBatcher` (`crates/statguardian-io/src/lib.rs`) now materializes
  the whole file once instead of reading it in bounded-memory chunks. Still
  correct (one read total, same results), no longer bounded-memory for very
  large files. Full detail and remediation options in
  [`SECURITY_AUDIT.md` §2b](SECURITY_AUDIT.md). Not fixed; needs a dedicated
  pass (either a hand-rolled chunked CSV reader or `polars-parquet`'s
  lower-level row-group API).
- **`cargo audit`: 2 of an original 10 findings remain** (`quick-xml` HIGH
  severity, `RUSTSEC-2026-0194`/`-0195`), confirmed blocked on an upstream
  `polars`/`object_store` release, not fixable from this repo today. Full
  trace in `SECURITY_AUDIT.md` §2. CI explicitly ignores just these two IDs
  with a comment pointing back to that doc.
- **Benchmark numbers are not reproducible** — `docs/bench/benchmark.py`
  exists but its output has never been committed. Treat all performance
  claims (including this project's own) as unverified until you run it
  yourself.
- **Three overlapping roadmap docs** (`ROADMAP.md`, `ROADMAP_HONEST.md`,
  `ROADMAP_INTEGRATED.md`) are not kept in sync and were not written with a
  consistent honesty bar — `ROADMAP.md` and `ROADMAP_INTEGRATED.md` now
  carry disclaimers, but a full consolidation into one document has not been
  done. This is itself a piece of documentation debt worth a dedicated
  cleanup pass.
- **`pandas` is not auto-converted** — `execute()` requires a Polars
  DataFrame; passing pandas raises an unhelpful `AttributeError` rather than
  converting. Documented in the main `README.md` Known Issues section.

## Technical debt summary

Concrete, file-specific findings from this audit (2026-09-21), each flagged
by whether it needs its own dedicated follow-up session.

**Needs a dedicated session:**

1. **DSL parser quote-stripping bug** (`enum=`, `regex=`, anomaly `method=`/
   `pattern=` all silently non-functional) — see the top item under "Known
   regressions" above. HIGH priority: breaks advertised, headline features
   silently, with no error surfaced to users. Fix locations:
   `crates/statguardian-core/src/parser/mod.rs` lines ~121-126 (regex),
   ~159-162 (enum), ~386-389 (named args) — mirror the correct pattern
   already used in `parse_literal_value()` (~line 253). Needs new regression
   tests for all three constructs, not just the parser fix itself.
2. **`StreamingBatcher` bounded-memory regression**
   (`crates/statguardian-io/src/lib.rs`) — see "Known regressions" above and
   `SECURITY_AUDIT.md` §2b. Needs either a hand-rolled chunked CSV reader or
   `polars-parquet`'s row-group API.
3. **`cargo audit`: 2 findings remain** (`quick-xml` HIGH, transitive via
   `object_store`/`polars`) — blocked on an upstream release, re-check after
   any future `polars` bump. See `SECURITY_AUDIT.md` §2.
4. **Three overlapping, inconsistently-honest roadmap docs** — not
   consolidated (see above).
5. **`docs/bench/benchmark.py` output never committed** — no reproducible
   performance numbers exist for this project despite marketing-style
   performance claims having appeared in project metadata in the past (see
   item 8 below).

**Safe cleanup, not urgent, no dedicated session needed:**

6. **A compiled binary is committed directly to git**:
   `python/statguardian/_statguardian.abi3.so` (macOS arm64 Mach-O shared
   library, ~25MB, confirmed via `file` and `git ls-files`) is tracked in
   version control, not gitignored. It was last updated in commit `fa78c1c`
   (2026-08-23) — **before** the 2026-08-30 sliding-window fix, the
   2026-09-14 pyo3/polars migration, and the 2026-09-17 rustls bump — so the
   binary checked into git is now stale relative to source — confirmed
  directly: reinstalling the committed binary and calling the new
  `statguardian --version` CLI flag (added in this audit, see below) prints
  `statguardian 2.4.0`, two releases behind the actual `2.6.0` in
  `Cargo.toml`/`pyproject.toml`, because `__version__` is baked in at Rust
  compile time via `env!("CARGO_PKG_VERSION")`
  (`crates/statguardian-py/src/lib.rs:558`). It is also
   single-platform (macOS arm64 only), useless for Linux CI or other
   architectures, and bloats repository size/history. `pip install` from
   PyPI or a fresh `maturin build` both rebuild from source and are
   unaffected, but anyone relying on the checked-in binary directly would
   get stale, pre-migration behavior. Recommend: remove from tracking, add
   `*.abi3.so` / `*.so` to `.gitignore`, and — in a separate, deliberate
   step (not done here, out of scope for a docs pass) — consider whether it
   should also be purged from git history given its size.
7. Two minor, already-documented `#[allow(dead_code)]` uses
   (`crates/statguardian-io/src/iceberg.rs:28`,
   `crates/statguardian-io/src/delta.rs:22`) for JSON-deserialized struct
   fields that are parsed for format-completeness but not yet consumed.
   Each has an explanatory comment; low-risk, not urgent.
8. `pyproject.toml`'s `description` field previously read "13x faster than
   pandera. Supports Pandas, Polars, DuckDB" — an unverified specific
   performance multiplier (contradicting this project's own admission that
   no reproducible benchmark has ever been published) that also implied
   DuckDB was a first-class DataFrame framework rather than one of several
   SQL backends behind `execute_sql()`. Corrected during this audit to match
   the honest description used in `README.md`.
9. **Fixed 2026-09-21 (quick-fix pass following this audit):** a handful of
   small, independently-found issues, none requiring a dedicated session —
   `docs/LICENSES.md:7`'s `[LICENSE](LICENSE)` link resolved to a
   nonexistent `docs/LICENSE` (now `../LICENSE`); `docs/CLI.md:352` and
   `docs/DBT_INTEGRATION.md:63` both linked to `../README.md#dsl-reference`,
   an anchor that has never existed in `README.md` (repointed to
   `examples/*.sg`/the README itself); `.github/CI_ERRORS.md` was an
   orphaned template with unresolved `$REPO_NAME`/`#$repo` placeholders and
   links to two files that don't exist anywhere in this repo, referenced by
   nothing else, so it was deleted; and one `clippy::single_match` warning
   in a `statguardian-io` test (`crates/statguardian-io/src/cloud.rs:163`)
   was fixed. See `CHANGELOG.md` `[Unreleased]` → `Fixed` for the full list.
   Full test suite (Rust + `pytest`) re-verified green after these changes.

## Not planned

- GUI for DSL authoring (CLI/text-editor only)
- Direct Snowflake/BigQuery native validation without going through
  `execute_sql`/`execute_cloud` first
