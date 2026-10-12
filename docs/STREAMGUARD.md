# StreamGuard & AgentGuard — Extension Design + Capability-Gap Matrix

**Status:** design + first implementation milestone
**Base version:** 2.6.3
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
- **Tests:** **111 Rust tests, 0 failing** (baseline captured 2026-10-12 via
  `cargo test --workspace --exclude statguardian --release`). The PyO3 crate
  is excluded from plain `cargo test` (link-only failure on macOS, by design).
- **Git:** working tree clean at commit `4e2a8d5` (2.6.3), `origin/main`.
  No uncommitted changes; nothing to preserve.

### Capability-gap matrix

| Capability | State | Evidence / note |
|---|---|---|
| Schema / quality / anomaly validation on full DataFrames | Implemented | `validators`, `engine`, 111 tests |
| File → per-batch reports (`execute_streaming`) | Incomplete | one whole-file read then slice; not bounded memory |
| Incremental NDJSON record reading | Missing | no reader over `BufRead` |
| Record count / byte / line-size limits | Missing | no limit type anywhere |
| Malformed-record policy (fail/skip/collect) | Missing | format errors abort or are not modeled |
| Field redaction | Missing | `_pii.py` detects PII; nothing redacts |
| Field normalization policy | Missing | none |
| Stable per-record findings + global indices | Missing | batch-local `row_indices` only |
| Streaming exit codes (0/1/2) | Missing | only whole-file `validate` has codes |
| stdout/stderr separation + broken-pipe handling | Partial | `_cli.py` uses `print`; no `BrokenPipe` guard |
| Agent event model (tool calls, tokens, cost, retries) | Missing | no code |
| Agent telemetry measured-vs-estimated distinction | Missing | none |
| Loop detection (heuristic) | Missing | none |
| Agent policy enforcement | Missing | none |
| DSL `stream{}` consumed by engine | Missing (TD-0003) | parsed only |

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
5. **Stable findings.** Row-scoped violations are expanded to global record
   indices (`batch_base + local_row`) and sorted deterministically.
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
- **M6** CLI wiring, Python binding, docs, CI, performance notes. ⚠️ Python
  binding + CLI wired but **not verified on this machine** (no `polars` in the
  local Python and the extension was not rebuilt); Rust path fully verified.

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

let mut v = StreamValidator::new(contract, dag, opts);
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

**Wired but not verified here:** the PyO3 binding
(`execute_stream_ndjson`) and the `statguardian stream` CLI subcommand. The
extension must be rebuilt with `maturin develop` and the Python suite run in
CI to count as verified.

**Deferred (explicitly not attempted):** consuming the DSL `stream{}`
window/watermark config (TD-0003); infra-based backpressure control against a
live producer (the reader honors `BufRead` natural backpressure, but there is
no Kafka/Flink consumer); purging repo history for the committed `.so`
(TD-0008); upstream `quick-xml` advisories (TD-0004/TD-0005).
