# Complete Technical Debt Register

This register consolidates findings from this audit pass (2026-10-05) with
the prior, already-thorough audits captured in `docs/SECURITY_AUDIT.md` and
`docs/ROADMAP_HONEST.md`. Those two documents remain the canonical
deep-dives for security and roadmap-honesty respectively; this file is the
master cross-referenced backlog with stable IDs, per-item status, and
priority, and is the one to update on future audit passes.

## Executive Summary

| | Count |
|---|---|
| Total items | 12 |
| Open | 8 |
| Resolved (this pass) | 2 |
| Resolved (prior passes, kept for history) | 2 |
| Critical (P0) | 0 |
| High (P1) | 2 |
| Medium (P2) | 5 |
| Low (P3) | 5 |

## P0 — Critical

None open.

## P1 — High

| ID | Category | Description | Source | Status | Fix |
|---|---|---|---|---|---|
| TD-0001 | BUG | DSL parser quote-stripping bug: `regex=`, `enum=[...]`, and anomaly named args (`method=`, `pattern=`) read `string_literal`'s raw `as_str()` text directly, which includes the surrounding `"` characters (the grammar rule is atomic). Every regex/enum constraint always failed every value; every `method=`/`pattern=` named arg silently matched nothing. Headline, README-advertised DSL features were completely non-functional for every user, always, with no error surfaced. | `docs/ROADMAP_HONEST.md` (discovered 2026-09-21, left unfixed pending a "dedicated session") | **RESOLVED** (this pass) | Added a shared `unquote()` helper in `crates/statguardian-core/src/parser/mod.rs` and applied it at all affected call sites: `regex_constraint`, `enum_constraint`, `named_arg` (anomaly args). Added 3 regression tests (`test_regex_constraint_pattern_is_unquoted`, `test_enum_constraint_values_are_unquoted`, `test_anomaly_named_arg_is_unquoted`) asserting the *actual unquoted value*, not just that a constraint of the right variant exists (the latter is why the original bug had zero test coverage). |
| TD-0002 | BUG | **Newly found during this audit** (not in prior docs): `stream { window=... watermark=... emit=... }` block parsing is a complete no-op regardless of quoting. `parse_stream()` matched `opt.as_rule()` against `Rule::stream_window`/`stream_watermark`/`stream_emit` directly, but the grammar's `stream_option = { stream_window \| stream_watermark \| stream_emit }` wraps each alternative in an intermediate `stream_option` pair — so the match always fell through to `_ => {}` and `StreamConfig.window/watermark/emit` were always `None`, for any input. | SOURCE_CODE (found while verifying TD-0001's fix) | **RESOLVED** (this pass) | Unwrap one more level (`wrapper.into_inner().next().unwrap()`) before matching on the rule. Added `test_stream_config_values_are_unquoted`, which also exercises this nesting fix (it failed with `None` before the nesting fix was added, confirming the bug independently of the quoting fix). Note: even after this fix, `StreamConfig.window/watermark/emit` are parsed but still not consumed anywhere downstream — see TD-0003. |

## P2 — Medium

| ID | Category | Description | Source | Status | Fix |
|---|---|---|---|---|---|
| TD-0003 | ARCHITECTURE / STUB | `StreamConfig.window`/`.watermark`/`.emit` (see TD-0002) are parsed from the DSL into the AST but never read by `statguardian-engine`, `statguardian-io`, or the Python bindings — confirmed via repo-wide grep. The `stream { ... }` block is accepted syntactically but has zero runtime effect. | INFERRED (this pass) | OPEN | Either wire these into actual windowed-execution behavior (a real feature, belongs in a dedicated session design-first) or document the block as parsed-but-not-yet-enforced in `docs/ROADMAP_HONEST.md` / `docs/CLI.md` so users don't believe `watermark=`/`emit=` do anything today. |
| TD-0004 | SECURITY / DEPENDENCY | `cargo audit`: 2 HIGH-severity findings remain open (`quick-xml` 0.39.4, `RUSTSEC-2026-0194`/`-0195`, memory-exhaustion DoS + quadratic runtime). Pulled transitively via `object_store` → `polars` 0.55.2; no compatible `quick-xml` version is reachable from this repo (`cargo update -p quick-xml` resolves 0 packages). Confirmed blocked on an upstream `polars`/`object_store` release during this pass. | CI_FAILURE / `docs/SECURITY_AUDIT.md` §2 | OPEN — blocked on upstream | Re-check after every future `polars` bump. CI already allow-lists just these two advisory IDs with a comment pointing back to the security doc. |
| TD-0005 | DEPENDENCY | `cargo audit` warning: `bincode` 2.0.1 unmaintained (`RUSTSEC-2025-0141`), pulled transitively via `polars-utils` → `polars`. Not our direct choice. | CI (cargo-audit warnings) | OPEN — blocked on upstream | Same remediation path as TD-0004: re-check after `polars` bumps. |
| TD-0006 | BUILD / DEVEX | Plain `cargo build`/`cargo test` on macOS fails to link the `statguardian` (PyO3 extension-module) crate — `ld: symbol(s) not found for architecture arm64` for `_PyTuple_*`/`_PyType_*`/etc. This is expected (the crate must be built via `maturin develop`, documented in `CONTRIBUTING.md`), and CI already works around it (`ci: exclude statguardian py-bindings crate from plain cargo build/test`, commit `8819852`). However, CI only runs on `ubuntu-latest` — there is **no CI coverage at all for `maturin develop`/the actual Python-extension build on macOS**, which is a common contributor platform (this audit was run on a macOS laptop). A regression in the PyO3 binding that only manifests on macOS linking would not be caught by CI today. | SOURCE_CODE / CI_CD (this pass) | OPEN | Add a `macos-latest` leg to the CI matrix that runs `pip install maturin && maturin develop --release` + the Python test suite, in addition to the existing Linux `rust-build`/`python-tests` jobs. |
| TD-0007 | TEST_DEBT | `statguardian-metrics` and `statguardian-validators` crates have **zero** in-crate unit tests (`cargo test` reports `0 passed` for both). They are exercised indirectly through `tests/test_validators.rs` (integration tests, outside the crate boundary) and the Python test suite, so this is not a coverage hole in practice, but there's no fast, crate-local regression signal for changes to e.g. `statguardian-validators::schema` (the exact file where TD-0001's bug lived) or `statguardian-metrics::report`. | SOURCE_CODE (this pass) | OPEN | Add focused unit tests inside `crates/statguardian-validators/src/schema.rs` and `crates/statguardian-metrics/src/report.rs` for their core logic, independent of the integration-test crate. |

## P3 — Low

| ID | Category | Description | Source | Status | Fix |
|---|---|---|---|---|---|
| TD-0008 | DOCUMENTATION | A compiled binary, `python/statguardian/_statguardian.abi3.so` (~25MB, macOS arm64), is committed directly to git and not gitignored. Confirmed stale: it reports `statguardian 2.4.0` via the (newly added) `--version` flag, two releases behind the actual `2.6.0`. | `docs/ROADMAP_HONEST.md` item 6 (found 2026-09-21) | OPEN | Remove from tracking, add `*.abi3.so`/`*.so` to `.gitignore`. Purging it from git history (repo-size concern) is a separate, more deliberate step. |
| TD-0009 | DOCUMENTATION | Three overlapping roadmap docs (`docs/ROADMAP.md`, `docs/ROADMAP_HONEST.md`, `docs/ROADMAP_INTEGRATED.md`) are not consolidated. The first two now carry disclaimer banners pointing at `ROADMAP_HONEST.md` as canonical, but no single-document cleanup has been done. | `docs/ROADMAP_HONEST.md` item 4 | OPEN | Dedicated docs-consolidation pass; not urgent since the disclaimers already prevent readers from trusting the stale claims. |
| TD-0010 | DOCUMENTATION / PERFORMANCE | `docs/bench/benchmark.py` exists but its output has never been committed — there are no reproducible numbers backing this project's own performance claims (including the historical "13x faster than pandera" framing, already corrected out of `pyproject.toml`'s description per `docs/ROADMAP_HONEST.md` item 8). `docs/bench/nyc311_vs_pandera.py` (added 2026-09-26) is a newer, real-data comparison script — also unclear if its output has been committed anywhere citable. | `docs/ROADMAP_HONEST.md` item 5 | OPEN | Run the benchmark scripts, commit the output (or a summary of it) somewhere linked from the README, and treat uncommitted/unreproduced numbers as marketing claims until then. |
| TD-0011 | ARCHITECTURE | Two documented `#[allow(dead_code)]` uses (`crates/statguardian-io/src/iceberg.rs:28`, `crates/statguardian-io/src/delta.rs:22`) for JSON-deserialized struct fields that are parsed for format-completeness but not yet consumed. Each has an explanatory comment. | `docs/ROADMAP_HONEST.md` item 7 | OPEN — low risk | Wire the fields in when a consumer needs them, or remove if genuinely never going to be used. |
| TD-0012 | DOCUMENTATION | `execute()` requires a Polars DataFrame; passing a pandas DataFrame raises an unhelpful `AttributeError` instead of converting or erroring clearly. Already documented in `README.md`'s Known Issues section. | `README.md` | OPEN | Either auto-convert via `pl.from_pandas()` at the API boundary, or raise a clear `TypeError` naming the expected type instead of letting `AttributeError` leak from internals. |

## Explicit TODOs

None found in Rust (`crates/`, `tests/`, `integrations/`) or Python (`python/`, `src/`) source during this pass — grep for `TODO|FIXME|XXX|HACK|NOT IMPLEMENTED|placeholder|stub|dummy` returned zero matches outside test fixtures. `docs/ROADMAP_HONEST.md` itself states "TODOs in Code: None found in Python; Rust safety needs audit" (that Rust-safety audit has since been done — see `docs/SECURITY_AUDIT.md`, zero `unsafe` blocks confirmed).

## Stubs

- `python/statguardian/_html.py:15` — `pass  # ValidationReport is a Rust PyObject; type hint only`. Legitimate type-stub, not debt.
- See TD-0003 (`stream` block parsed but not behaviorally wired up — arguably a "stub feature" even though every line of parsing code is real).

## Partial Implementations

- TD-0003 (stream config parsed, not consumed).
- TD-0011 (two dead-code-allowed struct fields, parsed not consumed).

## Planned Features

See `docs/ROADMAP_HONEST.md` for the authoritative "what's shipped vs. not" breakdown (this repo does **not** have a REST API, workflow-tool integrations (n8n/Power Automate/Temporal/Airflow/UiPath), Slack/PagerDuty alerting, JSON-lines audit logging, or LLM/RAG quality gates, despite other roadmap docs having claimed otherwise at various points). No new planned-feature work was identified in this pass beyond what's already tracked there.

## CI/CD Debt

- TD-0006 (no macOS leg in CI for the PyO3 extension build).
- `cargo audit` in `.github/workflows/ci.yml` allow-lists exactly TD-0004/TD-0005's advisory IDs — confirmed still accurate and necessary as of this pass (removing the allow-list would currently fail CI on 2 errors + 1 warning).
- `actionlint` run against all workflow YAML during this pass: clean, no findings.

## Test Debt

- TD-0007 (no in-crate unit tests for `statguardian-metrics`/`statguardian-validators`).
- TD-0001/TD-0002's root cause — tests asserted a constraint's *variant* existed but never its *value* — is itself a test-debt pattern worth watching for elsewhere in the parser test suite.

## Dependency Debt

- TD-0004, TD-0005 (both blocked on upstream `polars`/`object_store`).

## Security Debt

- TD-0004 (2 HIGH-severity `quick-xml` advisories, open, blocked upstream). Everything else in `docs/SECURITY_AUDIT.md` is closed as of that document's last update; re-verified current during this pass (zero `unsafe` blocks, DSL input validation wired, exception-swallowing fixed, gitleaks in CI).

## Architecture Debt

- TD-0003, TD-0011.

## Performance Debt

- `StreamingBatcher` bounded-memory regression (introduced by the required `polars` 0.44→0.55 security upgrade — Polars removed its public incremental/batched reader API) — tracked in `docs/ROADMAP_HONEST.md` item 2 / `docs/SECURITY_AUDIT.md` §2b. Still open as of this pass; not re-derived here in full, see those docs.
- TD-0010 (no reproducible benchmark numbers to evaluate actual performance debt against).

## Documentation Debt

- TD-0008, TD-0009, TD-0010, TD-0012.

## Resolved Historical Issues

Kept for history, not re-derived in this pass — see `docs/SECURITY_AUDIT.md` for full detail:

- SQL injection review (CRITICAL) — closed, false positive, no vulnerable code found.
- Dependency pinning (HIGH) — closed, core deps pinned.
- Rust unsafe-block audit (MEDIUM) — closed, zero `unsafe` blocks, `cargo audit` wired into CI.
- Secrets-handling guide (HIGH) — closed.
- DSL input validation / DoS limits (MEDIUM) — closed (10MB parser limit, Python CLI validator wired in).
- Broad exception handling / silent swallows (MEDIUM) — closed.
- CI secrets scanning (LOW) — closed, gitleaks added.
- 5 of original 10 `cargo audit` findings fixed 2026-09-13/14 (`sqlx` 0.8→0.9, `rusqlite` 0.31→0.37, `chacha20` 0.10.1→0.10.2, `pyo3` 0.21→0.29 + `polars` 0.44→0.55 migration, `rustls` 0.23.43→0.23.45) — see commits `5a9b728`, `723aad5`, `99d8d13`, `8819852`, `ff6910f`.
- Stale `docs/ROADMAP_HONEST.md` (previously described shipped features — Iceberg, lineage tracking — as "not implemented") — rewritten against the real codebase 2026-09-21.
- Fully-stubbed data lineage tracking — implemented for real, commit `fbb41f3`.
- `sliding-window` state never evicted (unbounded memory) — fixed, release 2.6.0.
- Broken doc links, orphaned CI error template, one `clippy::single_match` warning — fixed 2026-09-21 quick-fix pass.
- One `clippy::useless_vec` warning in `statguardian-lineage`'s test module (`storage.rs:623`, found and fixed independently during this pass before discovering it was unrelated to the already-landed `cloud.rs:163` fix from the prior pass).
