# Statguardian Security Audit

**Last Audited:** July 2026
**Remediation Pass:** August 2026
**Status:** All CRITICAL/HIGH items closed. Remaining items are documentation/hardening follow-ups.

---

## 🔴 CRITICAL — CLOSED

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

## 🟡 HIGH Priority — CLOSED

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

### 3. Environment Variable Secrets
**Location:** `python/statguardian/_connectors.py`, `docs/SECURITY.md`
**Status:** Closed — guidance exists.

`execute_cloud()` docs and `docs/SECURITY.md` already recommend IAM roles /
Workload Identity / Managed Identity over long-lived credentials, and
`.env.example` + `.gitignore` prevent accidental secret commits. Secrets are
never logged (verified — no logging calls include connection strings or
credential values).

---

## 🔵 MEDIUM Priority

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

## 🔵 LOW Priority — CLOSED

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
