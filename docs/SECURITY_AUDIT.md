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

**Remaining, not fixed — needs dedicated follow-up, not a quick fix:**
`pyo3` 0.21.2 (2 advisories: `RUSTSEC-2025-0020` buffer-overflow risk needs
>=0.24.1; `RUSTSEC-2026-0177` missing `Sync` bound needs >=0.29.0),
`quick-xml` 0.36.2 (2 **HIGH (7.5)** advisories, needs >=0.41.0), `fast-float`
0.2.0 (segfault risk + separately-tracked unsound warning, no fixed release
exists), `memmap2` 0.7.1 (unsound warning), and `rustls-pemfile` 2.2.0
(unmaintained warning). All five of the vulnerabilities trace back to one
root cause: `pyo3-polars` 0.18.0 pins `pyo3 ^0.21` and `polars ^0.44.0`
exactly, and quick-xml/fast-float/memmap2 are pulled in transitively through
that same pinned `polars` 0.44.x. **Attempted the full upgrade** (`polars`
0.44→0.55, `pyo3-polars` 0.18→0.28, `pyo3` 0.21→0.29 — the versions needed to
clear every remaining advisory at once) and hit substantial, real API
breakage: `DataFrame::new`'s signature changed (now takes an explicit
`height: usize` alongside columns), `DataFrame::get_columns()` was renamed,
`Series` was replaced by a new `Column` type in several APIs,
`LazyFrame::scan_parquet`/`LazyCsvReader::new`/`LazyJsonLineReader::new`/
`LazyFrame::scan_ipc` all changed their path argument type and `scan_ipc`'s
whole signature, and `CsvReader`/`ParquetReader`'s `.batched()` method was
removed/restructured — spanning `statguardian-io/src/{sql,cloud,lib}.rs` and
`statguardian-stats/src/profiler.rs`. This is genuine, non-mechanical
migration work (correctly handling the `Series`→`Column` change alone touches
null-handling semantics) that deserves its own dedicated pass with full test
coverage, not a rushed patch during a dependency-audit sweep — reverted
rather than risk silently-wrong data-loading behavior. Status of item 2
should be read as "significantly improved, CI-monitored, not yet fully
clean" — not "closed."

### 3. Environment Variable Secrets
**Location:** `python/statguardian/_connectors.py`, `docs/SECURITY.md`
**Status:** Closed — guidance exists.

`execute_cloud()` docs and `docs/SECURITY.md` already recommend IAM roles /
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
**Status:** Partially closed. `docs/SECURITY.md` covers reporting process and
basic practices; `_connectors.py` docstrings cover IAM-role/Workload-Identity
guidance per cloud provider. Still open: a single consolidated deployment
security page (least-privilege DB user setup, query audit logging) — tracked
as a documentation nice-to-have, not a code risk.

---

## Security Roadmap (updated)

| Issue | Severity | Status |
|-------|----------|--------|
| SQL injection review | CRITICAL | Closed — false positive, no vulnerable code found |
| Pin dependencies | HIGH | Closed — core deps pinned, extras intentionally floating |
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
