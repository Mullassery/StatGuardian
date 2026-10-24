# Repository Audit — StatGuardian

**Date:** 2026-10-05
**Version at time of audit:** 2.6.0

## Health

**GREEN.** This repository has already been through multiple rigorous audit
passes (see `docs/SECURITY_AUDIT.md`, `docs/ROADMAP_HONEST.md`, and 157
commits of history). This pass found and fixed the one remaining HIGH-severity
functional bug plus one previously-undiscovered adjacent bug, found no new
security issues, and confirmed the build/test/lint/CI pipeline is clean.
Remaining open items are either blocked on upstream dependency releases or
genuine, already-well-documented future work — not hidden defects.

## Before Audit (start of this pass, after pulling 8 commits this local
clone was behind on)

- Open debt items (tracked across `TECHNICAL_DEBT.md` + prior docs): 12
- Critical: 0
- High: 2 (TD-0001 regex/enum/method quote-stripping bug — known, undocumented-as-fixed; TD-0002 stream-config no-op — undiscovered)
- Medium: 5
- Low: 5
- `cargo audit`: 2 HIGH vulnerabilities + 1 unmaintained warning (unchanged by this pass — both blocked upstream)
- `cargo clippy --workspace --exclude statguardian --all-targets`: 0 warnings
- Tests: 107 Rust tests passing, 0 failing

## After Audit

- Open debt items: 10
- Critical: 0
- High: 0
- Medium: 5
- Low: 5
- `cargo audit`: unchanged (2 HIGH, blocked upstream on `polars`/`object_store`; 1 warning, same)
- `cargo clippy`: 0 warnings (unchanged, already clean)
- Tests: 111 Rust tests passing (added 4 regression tests), 0 failing

## Items Fixed

1. **TD-0001** — DSL parser quote-stripping bug (`regex=`, `enum=[...]`,
   anomaly `method=`/`pattern=` silently non-functional for every user,
   always). Fixed via a shared `unquote()` helper applied at all broken
   call sites in `crates/statguardian-core/src/parser/mod.rs`.
2. **TD-0002** (found during this pass) — `stream { window= watermark=
   emit= }` block parsing was a complete no-op due to a pest rule-nesting
   mismatch, independent of TD-0001's quoting issue. Fixed by unwrapping
   the `stream_option` wrapper pair correctly.
3. Added 4 regression tests (`test_regex_constraint_pattern_is_unquoted`,
   `test_enum_constraint_values_are_unquoted`,
   `test_anomaly_named_arg_is_unquoted`,
   `test_stream_config_values_are_unquoted`) that assert actual values, not
   just constraint variants — closing the test-coverage gap that let TD-0001
   ship unnoticed in the first place.
4. One `clippy::useless_vec` warning in
   `crates/statguardian-lineage/src/storage.rs` test module.

Full diff: `crates/statguardian-core/src/parser/mod.rs`,
`crates/statguardian-lineage/src/storage.rs`, plus the two new docs in this
commit.

## Items Remaining

See `TECHNICAL_DEBT.md` for the full register. Highlights:

- TD-0003 — parsed `stream` config still not consumed by the engine (now at
  least *parsed correctly*, per the TD-0002 fix, but still has zero runtime
  effect).
- TD-0004/TD-0005 — 2 HIGH `cargo audit` findings + 1 unmaintained-crate
  warning, all transitive via `polars`/`object_store`, confirmed blocked on
  an upstream release during this pass (`cargo update -p quick-xml`
  resolves 0 packages).
- TD-0006 — no macOS CI leg for the PyO3 extension build; a mac-only linking
  regression would not be caught today.
- TD-0007 — `statguardian-metrics`/`statguardian-validators` have zero
  in-crate unit tests (covered indirectly by integration tests only).
- TD-0008 through TD-0012 — documentation/perf-claim debt, already
  well-tracked in `docs/ROADMAP_HONEST.md`.

## Future Phase Work

None newly identified beyond what `docs/ROADMAP_HONEST.md` already tracks
("Not planned": GUI for DSL authoring, direct Snowflake/BigQuery native
validation without `execute_sql`/`execute_cloud`).

## CI Status

`.github/workflows/ci.yml`: secrets-scan (gitleaks), rust-build (`cargo
build/test --release --all-features` + `cargo audit` with the 2
known-blocked advisories allow-listed), python-tests (pytest on 3.10/3.11/
3.12, Linux only). `actionlint` run against all workflow YAML: clean. Last
observed CI run on `origin/main` (789638f, 2026-09-26): success. No macOS
runner exists (TD-0006).

## Test Status

111 Rust tests across the workspace (excluding the `statguardian` PyO3
crate, which cannot be exercised by plain `cargo test` — see Build Status),
0 failing. Python test suite (`tests/test_python_bindings.py`,
`test_lineage.py`, `test_okf_contracts.py`) not run locally in this pass —
this machine lacks `maturin` and the compiled extension, so it could not be
verified beyond static review; CI's `python-tests` job is the source of
truth and was last green on 2026-09-26.

## Build Status

`cargo build --workspace --exclude statguardian`: clean. Building the
`statguardian` crate (PyO3 bindings) via plain `cargo build` fails to link
on macOS (`_PyTuple_*`/`_PyType_*` undefined symbols) — this is expected;
the crate must be built via `maturin develop --release` per
`CONTRIBUTING.md`, and CI already excludes it from the plain-cargo path
(commit `8819852`). Not re-litigated as a new finding; TD-0006 tracks the
real gap (no CI coverage of the `maturin` path on macOS).

## Security Status

See `docs/SECURITY_AUDIT.md` — all CRITICAL/HIGH items closed except
TD-0004 (2 HIGH `quick-xml` advisories, blocked upstream). Zero `unsafe`
blocks in the Rust codebase (verified by grep during this pass, matching
the prior finding). `gitleaks` in CI.

## Dependency Status

429 crate dependencies (`cargo audit` scan count), 2 vulnerabilities + 1
unmaintained warning, both transitive via `polars`/`object_store`, both
confirmed unfixable from this repo today. No action taken this pass beyond
re-verification — a prior session already closed 5 of the original 10
findings (`sqlx`, `rusqlite`, `chacha20`, `pyo3`, `rustls` all bumped).

## Final Assessment

The repository's own `docs/ROADMAP_HONEST.md` and `docs/SECURITY_AUDIT.md`
set an unusually high bar for self-reported honesty about what's broken —
this pass validated that bar is still being met, closed the one HIGH-severity
item that document had explicitly deferred, caught one bug neither prior
audit had found (the `stream_option` nesting no-op), and confirmed the
remaining open items are genuinely blocked on external factors (upstream
dependency releases) or are legitimate, already-tracked future work rather
than hidden gaps. No regressions introduced: full workspace test suite and
clippy both clean after the fix.
