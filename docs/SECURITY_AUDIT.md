# Statguardian Security Audit

**Last Audited:** July 2026
**Remediation Pass:** August 2026
**Status:** All CRITICAL/HIGH items closed. Remaining items are documentation/hardening follow-ups.

---

## CRITICAL — CLOSED

### 1. SQL Injection Patterns
**Location:** `python/statguardian/_connectors.py`
**Original finding:** Dynamic SQL construction patterns in database connectors.

**Reassessment:** No string-interpolated or concatenated SQL was found anywhere in
the codebase (Python or Rust). `execute_sql()` forwards the caller-supplied
`connection_string` and `query` verbatim to `polars.read_database_uri` /
`sqlalchemy.create_engine` / `pandas.read_sql` — the same trust model as using
those libraries directly. There is no code path that builds a query string from
untrusted fragments (contract DSL, dataset paths, etc.). The original finding
was a false positive; there is nothing to patch here.

**Status:** Closed — no code change required.

---

## HIGH Priority — CLOSED

### 2. No Dependency Version Pinning
**Location:** `pyproject.toml`
**Status:** Closed for core/security-sensitive dependencies.

Core dependencies are pinned to exact versions: `polars==0.19.12`,
`pandas==2.1.0`, `pyarrow==14.0.1`. Optional extras (`connectorx`,
`psycopg2-binary`, `pymysql`, `google-cloud-bigquery`, etc.) intentionally use
floating minimums (`>=`) — these are third-party DB drivers pulled in only when
a user opts into that extra, and over-pinning them would force users into
version conflicts with their own environments. `sqlalchemy==2.0.23` is pinned
since it's shared across all SQL extras.

**Remaining follow-up (LOW):** periodically bump floating extras and re-check
for known CVEs — now covered by `cargo audit` in CI (Rust side) and should be
paired with a `pip-audit` run before each release (manual, not yet automated).

**Update, 2026-09-11 — `cargo audit` in CI was failing, unaddressed.**
The safety net above was running but not being kept green: `cargo audit`
against `Cargo.lock` at the time reported 10 real advisories (plus 6
warnings) in transitive dependencies, including two **HIGH severity (7.5)**
findings in `quick-xml` 0.36.2, plus findings in `pyo3` 0.21.2, `rsa` 0.9.10,
`rustls-webpki` 0.101.7, `sqlx` 0.8.0 itself, `memmap2` 0.7.1, `fast-float`
0.2.0, a yanked `chacha20` 0.10.1, and unmaintained `paste`/`rustls-pemfile`.

**Update, 2026-09-13 — 5 of 10 vulnerabilities + 3 of 6 warnings fixed.**
Upgraded `sqlx` 0.8→0.9 (fixes its own advisory outright, and — more
importantly — made the `mysql-rsa` feature opt-in separately from the base
`mysql` feature; since this crate never needed the RSA-based legacy MySQL
auth plugin, upgrading eliminated the vulnerable `rsa` crate — Marvin Attack
timing side-channel, `RUSTSEC-2023-0071`, no fixed upgrade ever existed —
from the dependency graph entirely, not just patched around it). That pulled
a patched `rustls-webpki` 0.103.15 (fixing all 3 of its advisories) and
dropped the unmaintained `paste` crate. Separately bumped `rusqlite` 0.31→
0.37 (required to resolve a native-library-linkage conflict between
`sqlx-sqlite` 0.9's and `rusqlite`'s `libsqlite3-sys` version ranges — both
link the system `sqlite3` library, so Cargo requires exactly one version)
and `chacha20` 0.10.1→0.10.2 (0.10.1 was yanked). `sqlx` 0.9's breaking API
change to `Database::Arguments` (dropped its lifetime parameter) and its new
`AssertSqlSafe` opt-in for non-`'static` query strings both required small,
verified source changes in `statguardian-io/src/sql.rs` — the latter is a
correct use, not a safety regression: these functions' whole contract is
"run the caller-supplied SQL," the same trust boundary that existed before.

**Update, 2026-09-14 — pyo3/polars migration completed. 3 of the remaining 5
vulnerabilities fixed; 2 are now blocked on an upstream release, not on
anything in this codebase.**

Completed the full upgrade this entry's previous version described as
reverted: `polars` 0.44→0.55, `pyo3-polars` 0.18→0.28, `pyo3` 0.21→0.29.
This closes both `pyo3` advisories (`RUSTSEC-2025-0020`, `RUSTSEC-2026-0177`)
outright, and — as a side effect of the `polars` bump — also cleared
`fast-float` 0.2.0 (segfault risk) and the `memmap2` 0.7.1 / `rustls-pemfile`
2.2.0 unmaintained warnings; none of those three appear in the dependency
tree at the new version anymore.

The real migration work, for the record: `DataFrame::new` now takes an
explicit `height: usize` alongside its columns; `DataFrame::get_columns()`
was renamed to `.columns()`; `LazyFrame::scan_parquet` /
`LazyCsvReader::new` / `LazyJsonLineReader::new` / `LazyFrame::scan_ipc` all
changed their path argument type to `PlRefPath` (fixed with `.into()`) and
`scan_ipc` gained a third `UnifiedScanArgs` argument; and — the one without a
mechanical fix — **Polars removed its public pull-based batched-reader API
entirely** (`OwnedBatchedCsvReader` / `BatchedParquetReader`, and
`CsvReader`/`ParquetReader::batched()`). There is no longer any public,
stable way in this dependency stack to read a CSV or Parquet file
incrementally, row-group-by-row-group, from a single held-open handle.
`StreamingBatcher` (`statguardian-io/src/lib.rs`) now falls back to the same
materialize-once-then-slice strategy previously used only for formats
without a native incremental reader (plain JSON, etc.): the source file is
still read exactly once for the whole batching session (never once per
batch — the bug this type exists to prevent), but peak memory now scales
with total file size rather than `batch_size`. `StreamingBatcher::
is_bounded_memory()` now always returns `false`, honestly, for every format.
**This is a real regression in this codebase, not just closed-out
paperwork** — tracked as its own open item below (§2b), separate from the
CVE remediation.

Separately, building `statguardian-py` (the crate that actually links
`pyo3-polars`) surfaced a second issue that isn't in RustSec at all: with
`pyo3-polars`'s `derive` feature enabled, `polars-plan`'s `python` feature
transitively enables `polars-core`'s `object` dtype — and `polars-ops`
0.55.2's `unique_counts` has a non-exhaustive `match` over `DataType` that
doesn't handle `DataType::Object(_)`, so the crate fails to compile whenever
`object` is on. This is a bug in `polars-ops` 0.55.2 itself
(`polars-ops-0.55.2/src/series/ops/unique.rs:45`), not something fixable
from this repo. Worked around it, not patched around it: `statguardian-py`
only ever used `pyo3_polars::PyDataFrame`, which lives in `pyo3-polars`'s
`types` module and was never gated on the `derive` feature in the first
place — dropping `derive` from this crate's `pyo3-polars` feature list
(see `Cargo.toml`) removes the `object` dependency chain entirely and avoids
the bug without giving up anything statguardian-py actually uses.

**Remaining, not fixed — genuinely blocked on an upstream release, not a
gap in this codebase:** `quick-xml` 0.39.4, 2 **HIGH (7.5)** advisories
(`RUSTSEC-2026-0194`, `RUSTSEC-2026-0195`), needs >=0.41.0. `quick-xml` is a
transitive dependency of `object_store` (via `polars-io`/`polars-error`),
which is itself a transitive dependency of `polars` — nothing in this
workspace depends on either directly. `polars` 0.55.2 pins
`object_store = "^0.13.1"`, and `object_store` 0.13.2 in turn pins
`quick-xml = "^0.39.0"` — both are exact-enough Cargo caret requirements
that neither can be bumped independently via `cargo update -p <crate>
--precise <version>` (confirmed: `cargo update -p quick-xml --precise
0.41.0` fails to resolve, citing this exact chain). Closing this requires
either a `polars` release that bumps its `object_store` requirement to
>=0.14, or an `object_store` 0.13.x patch that widens its own `quick-xml`
requirement past `^0.39.0` — neither is available as of 2026-09-14. Tracked
here; re-run `cargo audit` after any future `polars` bump to check whether
this has cleared upstream.

A new low-severity warning appeared as a side effect of the `polars` bump:
`bincode` 2.0.1 (`RUSTSEC-2025-0141`, unmaintained) is now a transitive
dependency via `polars-utils`. Warning only, not a vulnerability; no action
needed unless it graduates to an advisory.

Status of item 2: **2 of the original 10 vulnerabilities remain, both
confirmed blocked on an upstream release** — this is the closest to "closed"
this item can get without a `polars`/`object_store` release doing the rest.

### 2b. `StreamingBatcher` bounded-memory regression (NEW — introduced by the polars 0.55 upgrade above)
**Location:** `crates/statguardian-io/src/lib.rs`
**Severity:** Low (correctness/performance, not a security issue) — noted
here because it's a direct consequence of the security-driven dependency
upgrade above and belongs next to that history, not because it's itself a
vulnerability.

**Status:** Open — needs a dedicated pass, not a quick fix.

Before the polars 0.55 upgrade, `StreamingBatcher` read CSV and Parquet
files genuinely incrementally (via Polars' now-removed batched-reader API),
bounding peak memory to roughly `batch_size` regardless of total file size.
That primitive no longer has a public replacement in Polars' stable API (see
above). `StreamingBatcher` now reads the whole file once and slices it in
memory — correct (same row counts, same single-open-per-session guarantee,
covered by `crates/statguardian-io/src/lib.rs`'s own test module and
`tests/test_streaming.rs`) but no longer bounded-memory: a very large
CSV/Parquet file streamed in small batches will now hold the entire
decoded `DataFrame` in memory for the duration of the batching session.

**Options for restoring genuine incremental reads**, not yet attempted:
1. Read raw Parquet row groups directly via `polars-parquet`'s lower-level
   API (below the removed `polars-io` convenience layer) and convert each
   row group to a `DataFrame` independently.
2. Hand-roll a chunked CSV reader (read N lines at a time from a
   `BufReader`, parse each chunk with `CsvReadOptions` against an in-memory
   buffer) — loses some of Polars' CSV-parsing edge-case handling unless
   done carefully.
3. Wait for Polars to reintroduce a public streaming-read primitive designed
   around its new streaming execution engine, and adopt that.

Any of these is real, non-trivial engineering — deliberately not rushed
alongside the CVE remediation above.

### 3. Environment Variable Secrets
**Location:** `python/statguardian/_connectors.py`, `SECURITY.md`
**Status:** Closed — guidance exists.

`execute_cloud()` docs and `SECURITY.md` already recommend IAM roles /
Workload Identity / Managed Identity over long-lived credentials, and
`.env.example` + `.gitignore` prevent accidental secret commits. Secrets are
never logged (verified — no logging calls include connection strings or
credential values).

---

## MEDIUM Priority

### 4. Rust Unsafe Blocks
**Location:** Rust codebase (`crates/`)
**Status:** Closed.

`grep -rn "unsafe" crates/ --include=*.rs` (excluding `target/`) returns **zero
matches** — there are no `unsafe` blocks anywhere in the workspace. `cargo
audit` is now run automatically in `.github/workflows/ci.yml` (`rust-build`
job) to catch known-vulnerable dependency advisories on every push/PR.

### 5. No Input Validation on DSL
**Status:** Closed.

Two layers now enforce this:
- **Rust parser** (`crates/statguardian-core/src/parser/mod.rs`): hard
  `MAX_INPUT_SIZE` of 10MB, returns a structured `Result`/`CoreError` on
  malformed input instead of panicking (covered by `tests/test_security.rs`
  and `tests/test_parser.rs`).
- **Python CLI** (`python/statguardian/_dsl_validator.py`): previously written
  but never called anywhere in the codebase — dead code. Now wired into both
  `statguardian check` and `statguardian validate` (`_cli.py`) to reject
  oversized (>1MB), overly-nested (>50), or malformed contracts before they
  reach the parser, with a clean error message instead of a stack trace.

### 6. Broad Exception Handling
**Status:** Closed for the silent-failure cases; broad `except Exception` that
already surfaces the error (message, re-raise, or captured in a result object)
was left as-is by design.

Fixed two categories of `except: pass` that discarded errors with no trace:
- `_connectors.py` (`_read_sql_to_polars`): the connectorx→SQLAlchemy fallback
  chain silently dropped the reason each earlier strategy failed, so if all
  three failed the final error gave no diagnostic info. Now logs each
  intermediate failure at `DEBUG`.
- `okf_contracts.py` (`get_rule_success_rate`, anomaly pattern scan): corrupted
  or unreadable frontmatter files were skipped with zero indication anything
  was wrong. Now logs a `WARNING` naming the file and the error.

---

## LOW Priority — CLOSED

### 7. No Secrets Scanning in CI
**Status:** Closed. Added a `secrets-scan` job running
`gitleaks/gitleaks-action@v2` to `.github/workflows/ci.yml`, on every push and
PR to `main`.

### 8. Documentation: No Security Deployment Guide
**Status:** Partially closed. `SECURITY.md` covers reporting process and
basic practices; `_connectors.py` docstrings cover IAM-role/Workload-Identity
guidance per cloud provider. Still open: a single consolidated deployment
security page (least-privilege DB user setup, query audit logging) — tracked
as a documentation nice-to-have, not a code risk.

---

## Security Roadmap (updated)

| Issue | Severity | Status |
|-------|----------|--------|
| SQL injection review | CRITICAL | Closed — false positive, no vulnerable code found |
| Pin dependencies | HIGH | 2 of 10 `cargo audit` findings remain, blocked upstream (§2) |
| Rust unsafe block audit | MEDIUM | Closed — zero unsafe blocks; cargo audit now in CI |
| Secrets handling guide | HIGH | Closed — IAM/Workload Identity guidance in place |
| DSL input validation | MEDIUM | Closed — Rust size limit + Python validator now wired in |
| Exception handling review | MEDIUM | Closed — silent swallows now logged |
| CI secrets scanning | LOW | Closed — gitleaks added to CI |
| Consolidated deployment guide | LOW | Open — documentation only, no code risk |

---

## Testing Recommendations (still applicable)

1. **Dependency Audit** (automated for Rust, manual for Python):
   ```bash
   cargo audit      # now runs in CI on every push/PR
   pip-audit        # run manually before release
   ```

2. **SAST for Rust:**
   ```bash
   cargo clippy -- -W clippy::all
   cargo miri test
   ```

3. **DSL fuzzing:** `tests/test_security.rs` covers oversized input, deep
   nesting, malformed regex, and unbalanced braces. Extending with a proper
   fuzz target (`cargo fuzz`) is a reasonable next step if the parser grows
   more complex.

---

## Deployment Recommendations

- Use IAM roles instead of long-term AWS credentials
- Run database user with minimal permissions (no DROP, no CREATE)
- Enable query logging for audit trail
- Never commit `.env` files (use `.env.example`)
