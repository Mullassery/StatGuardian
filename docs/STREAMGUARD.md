# StreamGuard & AgentGuard — Extension Design + Capability-Gap Matrix

**Status:** implemented + Rust-verified + Python/CLI runtime-verified (M1–M6)
**Version:** 2.7.0 (added on top of 2.6.3)
**Last updated:** 2026-10-12

This document is the source-grounded plan for extending StatGuardian with
optional streaming validation, sanitization, and agent-runtime observability.
It reuses the existing validation engine, contracts, findings, and reporting;
it does **not** replace them and does not move core dependencies toward any
LLM SDK. Everything below is verified against the actual source in this
repository unless explicitly marked otherwise.

---

## 1. Audit summary (what actually exists today)

Verified by reading source, running the test suite, and inspecting Git state.

- **Workspace:** 9 crates under `crates/` (`core`, `engine`, `validators`,
  `stats`, `io`, `metrics`, `lineage`, `py`, plus the `tests` integration
  package). Rust 1.97, edition 2021, `polars` 0.55.2, `pyo3` 0.29.
- **Core validation:** `statguardian-core` (pest DSL parser, AST, compiler,
  execution DAG), `statguardian-validators` (`SchemaValidator`, `RuleEngine`,
  `Violation`), `statguardian-engine` (`BatchExecutor::execute`), and
  `statguardian-metrics` (`ValidationReport`, `DatasetHealthScore`).
- **Existing "streaming":** `Engine::execute_streaming(path, batch_size)`
  reads a whole file once via `StreamingBatcher` and slices it into
  per-batch `ValidationReport`s. The batcher is **materialize-once-then-slice**
  — honest `is_bounded_memory() == false` (Polars 0.55 removed the public
  incremental CSV/Parquet reader). There is **no** record-by-record NDJSON
  path, no malformed-record policy, no per-record limit, no sanitization.
- **Existing stream DSL block:** `stream { window= watermark= emit= }` parses
  into `StreamConfig` but is **not consumed** downstream (TD-0003). Window
  executors exist in `statguardian-core::streaming` but are not wired into
  the engine.
- **Python surface:** `crates/statguardian-py` exposes `DataContract`,
  `ValidationReport`, `execute`, `execute_file`, `execute_streaming`,
  Delta/Iceberg, `validate_dsl`, and lineage functions. The CLI
  (`python/statguardian/_cli.py`) has `validate`, `check`, and `dbt validate`.
- **Tests:** baseline **111 Rust tests, 0 failing** (captured 2026-10-12 via
  `cargo test --workspace --exclude statguardian --release`). After the work in
  this document: **145 passed / 0 failed** for that command, and **146 passed /
  0 failed** under the exact CI command
  `cargo test --release --all-features --workspace --exclude statguardian`
  (all-features enables one extra feature-gated test). 20 StreamGuard unit + 13
  StreamGuard integration + 1 doc-test were added; no pre-existing test changed.
  The PyO3 crate is excluded from plain `cargo test` (link-only failure on
  macOS, by design).
- **Git:** clean at the audit baseline commit `4e2a8d5` (2.6.3); this extension
  lands in 2.7.0 on `main`.

### Capability-gap matrix

| Capability | State | Evidence / note |
|---|---|---|
| Schema / quality / anomaly validation on full DataFrames | Implemented | `validators`, `engine`, 111 baseline tests |
| File → per-batch reports (`execute_streaming`) | Incomplete (unchanged) | one whole-file read then slice; not bounded memory |
| Incremental NDJSON record reading | Implemented | `statguardian-stream::ndjson` (`BufRead`, capped lines) |
| Record count / byte / line-size limits | Implemented | `StreamLimits` |
| Malformed-record policy (fail/skip/collect) | Implemented | `MalformedPolicy` |
| Field redaction | Implemented | `RedactionPolicy` (mask/mask-with/hash/null/drop) |
| Field normalization policy | Implemented | `NormalizationPolicy` (trim/lower/upper) |
| Stable per-record findings + global indices | Implemented | `StreamFinding` via real record-id mapping |
| Streaming exit codes (0/1/2) | Implemented | `StreamReport::exit_code` + CLI mapping |
| stdout/stderr separation + broken-pipe handling | Implemented | `_cmd_stream` (report→stdout, diagnostics→stderr) |
| Agent event model (tool calls, tokens, cost, retries) | Implemented | `statguardian-stream::agent` |
| Agent telemetry measured-vs-estimated distinction | Implemented | `Provenance` / `Measured<T>` |
| Loop detection (heuristic) | Implemented | `detect_loops`, labeled `heuristic: true` |
| Agent policy enforcement | Implemented (advisory) | `Enforcement::{Continue,Warn,Stop}` |
| DSL `stream{}` consumed by engine | Missing (TD-0003, deferred) | parsed only |

---

## 2. Architecture

New crate **`crates/statguardian-stream`** ("StreamGuard" + "AgentGuard"),
optional and additive. It depends on existing crates and Polars only — no new
runtime, no LLM SDK, no network:

```
statguardian-stream
├── ndjson.rs    incremental BufRead reader (capped line reads)
├── limits.rs    StreamLimits (records / bytes / record size / batch size)
├── sanitize.rs  redaction + normalization policies (explicit, non-silent)
├── report.rs    StreamReport, StreamFinding, MalformedRecord, exit codes
├── validator.rs StreamValidator → reuses Engine::execute per micro-batch
├── agent.rs     AgentEvent model + deterministic AgentGuard policies
└── error.rs     StreamError
```

Design rules honored:

1. **Reuse, don't fork.** Records are buffered into micro-batches, materialized
   to a Polars `DataFrame` via `JsonReader::with_json_format(JsonLines)`, and
   run through the existing `statguardian_engine::Engine::execute`. Findings
   come from the existing `Violation` type.
2. **Bounded memory.** Only `batch_size` records are ever resident; each line
   is read with a hard byte cap and the remainder of an oversized line is
   discarded without allocation. Source bytes are streamed from any `BufRead`
   (file, stdin, socket).
3. **No silent mutation.** Sanitization produces a *new* value and returns a
   `SanitizationEvent` per change. The source/reader is never mutated and the
   raw record is never logged unless the caller explicitly opts in.
4. **Diagnostics never leak payloads.** Malformed records report an index,
   a kind, and the JSON parser's position message — never the raw line unless
   `allow_raw_diagnostics: true` (off by default).
5. **Stable findings.** Row-scoped violations are expanded to *true global*
   record indices by mapping each batch row back through its source record id
   (`record_ids[local_row]`), so a skipped malformed line never shifts the
   reported index. Findings are sorted deterministically by
   `(record, column, check, batch)`.
6. **Telemetry honesty.** AgentGuard tags every value as `Measured`,
   `Estimated`, or `Unknown`; loop detection is labeled heuristic; limits are
   only *reported* (the tool has no authority to cancel a foreign agent).

### CLI / exit-code contract

`statguardian stream` (new subcommand) writes the machine report to **stdout**
and diagnostics/progress to **stderr**:

- `0` — passed
- `1` — validation failed (blocking finding, or any finding with
  `--fail-on-warning`)
- `2` — execution error (I/O, contract parse, malformed record under
  `--malformed=fail`, hard limit under `--on-limit=fail`)

---

## 3. Implementation milestones

- **M1** Audit + baseline. ✅ done (this document).
- **M2** Capability matrix + architecture. ✅ done (this document).
- **M3** Minimal end-to-end NDJSON stream validation with limits, malformed
  policy, stable findings, exit codes. ✅ implemented + tested in Rust.
- **M4** Redaction + normalization policies, non-silent, leak-safe. ✅
  implemented + tested in Rust.
- **M5** Agent event model + deterministic policies (measured vs estimated,
  heuristic loop detection). ✅ implemented + tested in Rust.
- **M6** CLI wiring, Python binding, docs, CI, performance notes. ✅
  `execute_stream_ndjson` + `StreamReport` added to `statguardian-py` and the
  `statguardian stream` subcommand added to `_cli.py`. Both are now
  **runtime-verified** end-to-end (see §5): the extension was built with
  `maturin develop` and the CLI/binding exercised for exit codes, stdout/stderr
  separation, malformed policy, limits, and blocking findings. Covered by
  `tests/test_python_bindings.py` in the CI `python-tests` job.

### Correctness fix surfaced by streaming

A pre-existing silent-skip bug was found and fixed: in Polars 0.55 a
single-row column is stored scalar-backed, and `Column::as_series()` returns
`None` for it. `SchemaValidator`, `RuleEngine`, and the engine's DAG `get_series`
all used `as_series()`/`if let Some(..)`, so a 1-row DataFrame validated
**nothing**. Streaming exposes this directly, because the final leftover
micro-batch is frequently exactly one record — meaning the last record could
silently pass. All three call sites now use `as_materialized_series()`, and
`findings_are_stable_and_sorted_across_batches` is a regression test asserting
the final 1-record batch is validated.

---

## 4. Rust API reference (implemented)

```rust
use statguardian_stream::{
    StreamValidator, StreamOptions, StreamLimits, MalformedPolicy,
    Sanitizer, RedactionPolicy, RedactionRule, RedactionStrategy,
    NormalizationPolicy,
};

let opts = StreamOptions {
    limits: StreamLimits { max_records: Some(1_000_000), ..Default::default() },
    malformed: MalformedPolicy::Collect { max: 100 },
    sanitizer: Sanitizer::new(
        RedactionPolicy {
            rules: vec![RedactionRule { field: "ssn".into(),
                                       strategy: RedactionStrategy::Hash }],
            recursive: true,
        },
        NormalizationPolicy::default(),
    ),
    ..Default::default()
};

let v = StreamValidator::new(contract, dag, opts);
let report = v.validate_reader(file)?;   // any std::io::BufRead
println!("{}", report.summary());
std::process::exit(report.exit_code(false));
```

`AgentGuard`:

```rust
use statguardian_stream::agent::{AgentGuard, AgentPolicy, Enforcement};
let guard = AgentGuard::new(AgentPolicy::default());
let verdict = guard.evaluate(&events);   // events: Vec<AgentEvent>
```

---

## 5. Verified vs. deferred

**Verified (Rust, this pass):** incremental NDJSON validation; bounded
per-batch memory; record/byte/size limits; fail/skip/collect malformed policy;
redaction (mask/hash/null/drop) + normalization (trim/case) with disclosure;
stable global findings; exit-code computation; AgentGuard telemetry
(measured/estimated/unknown), retry and loop heuristics, budget verdicts.
Commands: `cargo test --release --all-features --workspace --exclude statguardian`
→ **146 passed / 0 failed** (plain `--release` without `--all-features`: 145 /
0); `cargo clippy --release --all-features --workspace --exclude statguardian
--all-targets -- -D warnings` → clean; `cargo check -p statguardian` (PyO3) →
clean.

**Verified (Python + CLI, this pass):** the PyO3 binding
(`execute_stream_ndjson` + `StreamReport`) and the `statguardian stream` CLI
subcommand, built with `maturin develop --release` and exercised on
Python 3.9.6 / polars 1.32.2 / pyarrow 21.0.0 / statguardian 2.7.0:

- clean NDJSON → `passed`, exit `0`; report on stdout, empty stderr
- `between`/`positive` violations are `Severity::Error` → `passed`/exit `0`
  by default; `--fail-on-warning` / `exit_code(true)` → `1`
- a column absent from the data → `Severity::Blocking` → exit `1`
- malformed line under `collect` → diagnostic on **stderr only**, and global
  record ids do **not** shift (skipped line at index 2 leaves later findings at
  their true index); `preview` is `null` unless `allow_raw_diagnostics`
- `malformed="fail_fast"` → raises; CLI maps it to exit `2`
- `max_records` truncates (`truncated == true`); `--format json` is valid JSON
  on stdout; broken pipe (`| head`) raises no traceback

Locked into `tests/test_python_bindings.py` (6 new tests, 9 total in that file;
full `pytest tests/` → 35 passed), which runs in the CI `python-tests` job.

**Distribution (this pass):** `maturin build --release` emits one abi3 wheel,
`statguardian-2.7.0-cp38-abi3-macosx_11_0_arm64.whl` (~17 MB); installing it into
a fresh venv exposes `execute_stream_ndjson` / `StreamReport` and the
`statguardian stream` CLI (exit codes 0/1/2 verified from the installed wheel).

**Deferred (explicitly not attempted):** consuming the DSL `stream{}`
window/watermark config (TD-0003); infra-based backpressure control against a
live producer (the reader honors `BufRead` natural backpressure, but there is
no Kafka/Flink consumer); purging repo history for the committed `.so`
(TD-0008); upstream `quick-xml` advisories (TD-0004/TD-0005).
