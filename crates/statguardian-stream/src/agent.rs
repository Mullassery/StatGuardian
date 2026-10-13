//! Optional, normalized agent-runtime observability.
//!
//! This module is deliberately independent of any LLM SDK: an agent runtime
//! (or an adapter in `integrations/`) is expected to emit a normalized NDJSON
//! event stream, which `AgentGuard` parses, aggregates, and checks against
//! deterministic policies.
//!
//! Honesty guarantees:
//!
//! - Every numeric metric carries a [`Provenance`] tag. Values reported by the
//!   runtime as real are `Measured`; values the runtime computed locally (for
//!   example token counts from a tokenizer) are `Estimated`; absent values are
//!   `Unknown`.
//! - Loop detection is a **heuristic** over recent calls and is labeled as
//!   such; it is not proof of a loop.
//! - Budget enforcement is advisory. StatGuardian observes an event stream; it
//!   cannot cancel a foreign agent. `evaluate` therefore returns a
//!   recommendation ([`Enforcement`]) and flags whether the decision rests on
//!   measured or estimated data, leaving the actual stop to the caller.
use std::collections::BTreeMap;
use std::io::BufRead;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use statguardian_core::ast::Severity;

use crate::error::{StreamError, StreamResult};
use crate::ndjson::NdjsonSource;

/// Whether a telemetry value was actually observed or only computed locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    Measured,
    Estimated,
    Unknown,
}

/// A single numeric metric with its provenance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Measured<T> {
    pub value: T,
    pub provenance: Provenance,
}

#[allow(clippy::self_named_constructors)]
impl<T> Measured<T> {
    pub fn measured(value: T) -> Self {
        Self {
            value,
            provenance: Provenance::Measured,
        }
    }

    pub fn estimated(value: T) -> Self {
        Self {
            value,
            provenance: Provenance::Estimated,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKind {
    ToolCall,
    ToolResult,
    LlmCall,
    Retry,
    Error,
    PolicyViolation,
    Other,
}

impl AgentEventKind {
    fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "tool_call" | "tool_use" | "call" => AgentEventKind::ToolCall,
            "tool_result" | "tool_output" | "result" => AgentEventKind::ToolResult,
            "llm_call" | "llm" | "completion" | "model_call" => AgentEventKind::LlmCall,
            "retry" => AgentEventKind::Retry,
            "error" | "failure" => AgentEventKind::Error,
            "policy_violation" => AgentEventKind::PolicyViolation,
            _ => AgentEventKind::Other,
        }
    }
}

/// A normalized agent event. Unknown input fields are preserved in
/// `metadata` so nothing is silently discarded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
    pub execution_id: String,
    pub seq: u64,
    pub timestamp_ms: Option<i64>,
    pub kind: AgentEventKind,
    pub tool: Option<String>,
    pub call_signature: Option<String>,
    pub tokens_in: Option<Measured<u64>>,
    pub tokens_out: Option<Measured<u64>>,
    pub cost_usd: Option<Measured<f64>>,
    pub duration_ms: Option<Measured<u64>>,
    pub retry_of: Option<u64>,
    pub error: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AgentEvent {
    /// Parse one event from a JSON value, accepting common field aliases.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or_else(|| "agent event must be a JSON object".to_string())?;

        let get_str = |keys: &[&str]| -> Option<String> {
            keys.iter()
                .find_map(|k| obj.get(*k))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        let execution_id = get_str(&["execution_id", "run_id", "trace_id", "session_id"])
            .unwrap_or_else(|| "unknown".to_string());
        let seq = obj
            .get("seq")
            .or_else(|| obj.get("step"))
            .or_else(|| obj.get("index"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let timestamp_ms = obj
            .get("timestamp_ms")
            .or_else(|| obj.get("ts_ms"))
            .and_then(|v| v.as_i64());
        let kind = AgentEventKind::parse(
            &get_str(&["kind", "type", "event"]).unwrap_or_else(|| "other".to_string()),
        );

        let default_provenance = match get_str(&["provenance"]).as_deref() {
            Some("measured") => Provenance::Measured,
            Some("estimated") => Provenance::Estimated,
            _ => Provenance::Measured,
        };

        let tokens_in = parse_measured_u64(
            obj,
            &["tokens_in", "input_tokens", "prompt_tokens"],
            default_provenance,
        );
        let tokens_out = parse_measured_u64(
            obj,
            &["tokens_out", "output_tokens", "completion_tokens"],
            default_provenance,
        );
        let cost_usd = parse_measured_f64(obj, &["cost_usd", "cost"], default_provenance);
        let duration_ms = parse_measured_u64(
            obj,
            &["duration_ms", "latency_ms", "elapsed_ms"],
            default_provenance,
        );

        let known: &[&str] = &[
            "execution_id",
            "run_id",
            "trace_id",
            "session_id",
            "seq",
            "step",
            "index",
            "timestamp_ms",
            "ts_ms",
            "kind",
            "type",
            "event",
            "tool",
            "tool_name",
            "name",
            "call_signature",
            "args_hash",
            "signature",
            "tokens_in",
            "input_tokens",
            "prompt_tokens",
            "tokens_out",
            "output_tokens",
            "completion_tokens",
            "cost_usd",
            "cost",
            "duration_ms",
            "latency_ms",
            "elapsed_ms",
            "retry_of",
            "retry_index",
            "error",
            "provenance",
        ];
        let mut metadata = BTreeMap::new();
        for (k, v) in obj {
            if !known.contains(&k.as_str()) {
                metadata.insert(k.clone(), v.to_string());
            }
        }

        Ok(Self {
            execution_id,
            seq,
            timestamp_ms,
            kind,
            tool: get_str(&["tool", "tool_name", "name"]),
            call_signature: get_str(&["call_signature", "args_hash", "signature"]),
            tokens_in,
            tokens_out,
            cost_usd,
            duration_ms,
            retry_of: obj
                .get("retry_of")
                .or_else(|| obj.get("retry_index"))
                .and_then(|v| v.as_u64()),
            error: get_str(&["error"]),
            metadata,
        })
    }
}

fn parse_measured_u64(
    obj: &serde_json::Map<String, Value>,
    keys: &[&str],
    default: Provenance,
) -> Option<Measured<u64>> {
    let v = keys.iter().find_map(|k| obj.get(*k))?;
    match v {
        Value::Number(n) => n.as_u64().map(|u| Measured {
            value: u,
            provenance: default,
        }),
        Value::Object(o) => {
            let value = o.get("value").and_then(|x| x.as_u64())?;
            let provenance = parse_provenance(o.get("provenance"), default);
            Some(Measured { value, provenance })
        }
        _ => None,
    }
}

fn parse_measured_f64(
    obj: &serde_json::Map<String, Value>,
    keys: &[&str],
    default: Provenance,
) -> Option<Measured<f64>> {
    let v = keys.iter().find_map(|k| obj.get(*k))?;
    match v {
        Value::Number(n) => n.as_f64().map(|x| Measured {
            value: x,
            provenance: default,
        }),
        Value::Object(o) => {
            let value = o.get("value").and_then(|x| x.as_f64())?;
            let provenance = parse_provenance(o.get("provenance"), default);
            Some(Measured { value, provenance })
        }
        _ => None,
    }
}

fn parse_provenance(v: Option<&Value>, default: Provenance) -> Provenance {
    match v.and_then(|x| x.as_str()) {
        Some("measured") => Provenance::Measured,
        Some("estimated") => Provenance::Estimated,
        Some("unknown") => Provenance::Unknown,
        _ => default,
    }
}

/// Deterministic policy limits for an agent run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub max_tool_calls: Option<u64>,
    pub max_total_tokens: Option<u64>,
    pub max_cost_usd: Option<f64>,
    pub max_duration_ms: Option<u64>,
    /// Number of recent calls considered for loop detection.
    pub loop_window: usize,
    /// How many times a signature must recur in the window to be flagged.
    pub loop_repeat_threshold: usize,
    /// When true, budget breaches yield a `Stop` recommendation (advisory).
    pub enforce: bool,
}

impl Default for AgentPolicy {
    fn default() -> Self {
        Self {
            max_tool_calls: None,
            max_total_tokens: None,
            max_cost_usd: None,
            max_duration_ms: None,
            loop_window: 8,
            loop_repeat_threshold: 3,
            enforce: false,
        }
    }
}

/// Aggregate telemetry over a run, split by provenance.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentTelemetry {
    pub events: u64,
    pub tool_calls: u64,
    pub retries: u64,
    pub errors: u64,
    pub tokens_in_measured: u64,
    pub tokens_in_estimated: u64,
    pub tokens_out_measured: u64,
    pub tokens_out_estimated: u64,
    pub cost_measured_usd: f64,
    pub cost_estimated_usd: f64,
    pub duration_measured_ms: u64,
    pub duration_estimated_ms: u64,
    pub unknown_metrics: u64,
    pub executions: Vec<String>,
}

impl AgentTelemetry {
    pub fn total_tokens_measured(&self) -> u64 {
        self.tokens_in_measured + self.tokens_out_measured
    }

    pub fn total_tokens_estimated(&self) -> u64 {
        self.tokens_in_estimated + self.tokens_out_estimated
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPolicyViolation {
    pub policy: String,
    pub message: String,
    pub severity: Severity,
    /// True when the breach is evidenced by measured data; false when it only
    /// holds under estimated data.
    pub measured: bool,
    /// True for heuristic detections (currently only loop detection).
    pub heuristic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopFinding {
    pub signature: String,
    pub occurrences: usize,
    pub first_seq: u64,
    pub last_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    Continue,
    Warn,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPolicyReport {
    pub telemetry: AgentTelemetry,
    pub violations: Vec<AgentPolicyViolation>,
    pub loops: Vec<LoopFinding>,
}

impl AgentPolicyReport {
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }

    /// Advisory enforcement recommendation. StatGuardian cannot stop a foreign
    /// agent; the caller must act on this.
    pub fn enforcement(&self) -> Enforcement {
        if self.violations.is_empty() {
            Enforcement::Continue
        } else if self
            .violations
            .iter()
            .any(|v| v.severity == Severity::Blocking)
        {
            Enforcement::Stop
        } else {
            Enforcement::Warn
        }
    }
}

pub struct AgentGuard {
    policy: AgentPolicy,
}

impl AgentGuard {
    pub fn new(policy: AgentPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &AgentPolicy {
        &self.policy
    }

    /// Severity for a breach evidenced by *measured* data. When the caller
    /// opts into enforcement the breach becomes `Blocking` (advisory `Stop`);
    /// otherwise it is a non-blocking `Error` (advisory `Warn`).
    fn measured_breach_severity(&self) -> Severity {
        if self.policy.enforce {
            Severity::Blocking
        } else {
            Severity::Error
        }
    }

    /// Parse a normalized NDJSON agent-event stream. Malformed lines are a hard
    /// error (no silent skipping).
    pub fn parse_ndjson<R: BufRead>(
        reader: R,
        max_record_bytes: usize,
    ) -> StreamResult<Vec<AgentEvent>> {
        let mut src = NdjsonSource::new(reader, max_record_bytes);
        let mut events = Vec::new();
        while let Some(line) = src.next_line()? {
            if line.exceeded {
                return Err(StreamError::OversizedRecord {
                    record: line.record,
                    limit: max_record_bytes,
                    actual: line.bytes,
                });
            }
            if !line.utf8 {
                return Err(StreamError::InvalidUtf8 {
                    record: line.record,
                });
            }
            let value: Value =
                serde_json::from_str(&line.content).map_err(|e| StreamError::Malformed {
                    record: line.record,
                    reason: e.to_string(),
                })?;
            let event = AgentEvent::from_json(&value).map_err(|reason| StreamError::Malformed {
                record: line.record,
                reason,
            })?;
            events.push(event);
        }
        Ok(events)
    }

    /// Aggregate telemetry and evaluate deterministic policies.
    pub fn evaluate(&self, events: &[AgentEvent]) -> AgentPolicyReport {
        let telemetry = aggregate(events);
        let mut violations = Vec::new();

        if let Some(max) = self.policy.max_tool_calls {
            if telemetry.tool_calls > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_tool_calls".into(),
                    message: format!("{} tool calls exceed budget of {max}", telemetry.tool_calls),
                    severity: self.measured_breach_severity(),
                    measured: true,
                    heuristic: false,
                });
            }
        }

        if let Some(max) = self.policy.max_total_tokens {
            let measured = telemetry.total_tokens_measured();
            let estimated = telemetry.total_tokens_estimated();
            if measured > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_total_tokens".into(),
                    message: format!("{measured} measured tokens exceed budget of {max}"),
                    severity: self.measured_breach_severity(),
                    measured: true,
                    heuristic: false,
                });
            } else if measured + estimated > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_total_tokens".into(),
                    message: format!(
                        "{} measured + {estimated} estimated tokens may exceed budget of {max}",
                        measured
                    ),
                    severity: Severity::Warning,
                    measured: false,
                    heuristic: false,
                });
            }
        }

        if let Some(max) = self.policy.max_cost_usd {
            if telemetry.cost_measured_usd > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_cost_usd".into(),
                    message: format!(
                        "${:.4} measured cost exceeds budget of ${max:.4}",
                        telemetry.cost_measured_usd
                    ),
                    severity: self.measured_breach_severity(),
                    measured: true,
                    heuristic: false,
                });
            } else if telemetry.cost_measured_usd + telemetry.cost_estimated_usd > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_cost_usd".into(),
                    message: format!(
                        "${:.4} measured + ${:.4} estimated cost may exceed budget of ${max:.4}",
                        telemetry.cost_measured_usd, telemetry.cost_estimated_usd
                    ),
                    severity: Severity::Warning,
                    measured: false,
                    heuristic: false,
                });
            }
        }

        if let Some(max) = self.policy.max_duration_ms {
            if telemetry.duration_measured_ms > max {
                violations.push(AgentPolicyViolation {
                    policy: "max_duration_ms".into(),
                    message: format!(
                        "{}ms measured duration exceeds budget of {max}ms",
                        telemetry.duration_measured_ms
                    ),
                    severity: self.measured_breach_severity(),
                    measured: true,
                    heuristic: false,
                });
            }
        }

        let loops = detect_loops(
            events,
            self.policy.loop_window,
            self.policy.loop_repeat_threshold,
        );
        for loop_finding in &loops {
            violations.push(AgentPolicyViolation {
                policy: "loop_detection".into(),
                message: format!(
                    "heuristic: signature '{}' repeated {} times (seq {}→{})",
                    loop_finding.signature,
                    loop_finding.occurrences,
                    loop_finding.first_seq,
                    loop_finding.last_seq
                ),
                severity: Severity::Warning,
                measured: false,
                heuristic: true,
            });
        }

        AgentPolicyReport {
            telemetry,
            violations,
            loops,
        }
    }
}

fn aggregate(events: &[AgentEvent]) -> AgentTelemetry {
    let mut t = AgentTelemetry::default();
    let mut executions: Vec<String> = Vec::new();

    for e in events {
        t.events += 1;
        if e.kind == AgentEventKind::ToolCall {
            t.tool_calls += 1;
        }
        if e.kind == AgentEventKind::Retry || e.retry_of.is_some() {
            t.retries += 1;
        }
        if e.kind == AgentEventKind::Error {
            t.errors += 1;
        }
        if !executions.contains(&e.execution_id) {
            executions.push(e.execution_id.clone());
        }

        accumulate_metric_u64(
            &mut t.tokens_in_measured,
            &mut t.tokens_in_estimated,
            &mut t.unknown_metrics,
            e.tokens_in.as_ref(),
        );
        accumulate_metric_u64(
            &mut t.tokens_out_measured,
            &mut t.tokens_out_estimated,
            &mut t.unknown_metrics,
            e.tokens_out.as_ref(),
        );
        accumulate_f64(&mut t, e.cost_usd.as_ref());
        accumulate_duration(&mut t, e.duration_ms.as_ref());
    }

    executions.sort();
    t.executions = executions;
    t
}

fn accumulate_metric_u64(
    measured: &mut u64,
    estimated: &mut u64,
    unknown: &mut u64,
    m: Option<&Measured<u64>>,
) {
    if let Some(m) = m {
        match m.provenance {
            Provenance::Measured => *measured += m.value,
            Provenance::Estimated => *estimated += m.value,
            Provenance::Unknown => *unknown += 1,
        }
    }
}

fn accumulate_f64(t: &mut AgentTelemetry, m: Option<&Measured<f64>>) {
    if let Some(m) = m {
        match m.provenance {
            Provenance::Measured => t.cost_measured_usd += m.value,
            Provenance::Estimated => t.cost_estimated_usd += m.value,
            Provenance::Unknown => t.unknown_metrics += 1,
        }
    }
}

fn accumulate_duration(t: &mut AgentTelemetry, m: Option<&Measured<u64>>) {
    if let Some(m) = m {
        match m.provenance {
            Provenance::Measured => t.duration_measured_ms += m.value,
            Provenance::Estimated => t.duration_estimated_ms += m.value,
            Provenance::Unknown => t.unknown_metrics += 1,
        }
    }
}

fn detect_loops(events: &[AgentEvent], window: usize, threshold: usize) -> Vec<LoopFinding> {
    if window == 0 || threshold == 0 {
        return Vec::new();
    }
    let mut loops: Vec<LoopFinding> = Vec::new();
    let mut recent: Vec<(String, u64)> = Vec::new();

    for e in events {
        let Some(sig) = &e.call_signature else {
            continue;
        };
        recent.push((sig.clone(), e.seq));
        if recent.len() > window {
            recent.remove(0);
        }
        let occurrences: Vec<&(String, u64)> = recent.iter().filter(|(s, _)| s == sig).collect();
        if occurrences.len() >= threshold {
            let first_seq = occurrences.first().map(|(_, s)| *s).unwrap_or(e.seq);
            let last_seq = occurrences.last().map(|(_, s)| *s).unwrap_or(e.seq);
            if let Some(existing) = loops.iter_mut().find(|l| &l.signature == sig) {
                // Keep the reported count in step with the moving window.
                existing.occurrences = occurrences.len();
                existing.last_seq = last_seq;
            } else {
                loops.push(LoopFinding {
                    signature: sig.clone(),
                    occurrences: occurrences.len(),
                    first_seq,
                    last_seq,
                });
            }
        }
    }
    loops
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(v: Value) -> AgentEvent {
        AgentEvent::from_json(&v).unwrap()
    }

    #[test]
    fn parses_aliases_and_provenance() {
        let e = ev(json!({
            "run_id": "r1", "type": "tool_call", "tool_name": "search",
            "input_tokens": 10, "completion_tokens": {"value": 20, "provenance": "estimated"},
            "cost": {"value": 0.02, "provenance": "measured"}, "latency_ms": 5,
            "args_hash": "search:foo"
        }));
        assert_eq!(e.execution_id, "r1");
        assert_eq!(e.kind, AgentEventKind::ToolCall);
        assert_eq!(e.tool.as_deref(), Some("search"));
        assert_eq!(e.tokens_in.unwrap().provenance, Provenance::Measured);
        assert_eq!(e.tokens_out.unwrap().provenance, Provenance::Estimated);
        assert_eq!(e.cost_usd.unwrap().value, 0.02);
        assert_eq!(e.call_signature.as_deref(), Some("search:foo"));
    }

    #[test]
    fn telemetry_separates_measured_and_estimated() {
        let events = vec![
            ev(
                json!({"seq":0,"kind":"llm_call","tokens_in":100,"tokens_out":50,"cost_usd":0.01,"duration_ms":10}),
            ),
            ev(
                json!({"seq":1,"kind":"llm_call","tokens_in":{"value":10,"provenance":"estimated"},"cost_usd":{"value":0.001,"provenance":"estimated"},"duration_ms":10}),
            ),
        ];
        let guard = AgentGuard::new(AgentPolicy::default());
        let report = guard.evaluate(&events);
        assert_eq!(report.telemetry.total_tokens_measured(), 150);
        assert_eq!(report.telemetry.total_tokens_estimated(), 10);
        assert_eq!(report.telemetry.cost_measured_usd, 0.01);
        assert!((report.telemetry.cost_estimated_usd - 0.001).abs() < 1e-12);
    }

    #[test]
    fn measured_budget_breach_is_measured() {
        let events = vec![ev(json!({"seq":0,"kind":"tool_call","tool":"x"}))];
        let policy = AgentPolicy {
            max_tool_calls: Some(0),
            enforce: true,
            ..Default::default()
        };
        let report = AgentGuard::new(policy).evaluate(&events);
        assert!(!report.passed());
        let v = &report.violations[0];
        assert_eq!(v.policy, "max_tool_calls");
        assert!(v.measured);
        assert!(!v.heuristic);
        assert_eq!(report.enforcement(), Enforcement::Stop);
    }

    #[test]
    fn estimated_only_breach_is_warning_not_measured() {
        let events = vec![ev(json!({
            "seq":0,"kind":"llm_call",
            "tokens_in":{"value":1000,"provenance":"estimated"}
        }))];
        let policy = AgentPolicy {
            max_total_tokens: Some(10),
            ..Default::default()
        };
        let report = AgentGuard::new(policy).evaluate(&events);
        assert_eq!(report.violations.len(), 1);
        assert!(!report.violations[0].measured);
        assert_eq!(report.violations[0].severity, Severity::Warning);
        assert_eq!(report.enforcement(), Enforcement::Warn);
    }

    #[test]
    fn loop_detection_is_labeled_heuristic() {
        let events: Vec<AgentEvent> = (0..4)
            .map(|i| ev(json!({"seq":i,"kind":"tool_call","tool":"lookup","args_hash":"lookup:x"})))
            .collect();
        let guard = AgentGuard::new(AgentPolicy::default());
        let report = guard.evaluate(&events);
        assert_eq!(report.loops.len(), 1);
        assert_eq!(report.loops[0].signature, "lookup:x");
        assert_eq!(report.loops[0].occurrences, 4);
        assert!(report.violations.iter().any(|v| v.heuristic));
    }

    #[test]
    fn parse_ndjson_rejects_malformed() {
        let input = b"{\"seq\":0,\"kind\":\"tool_call\"}\nnot json\n".to_vec();
        let err = AgentGuard::parse_ndjson(Cursor::new(input), 1024).unwrap_err();
        assert!(matches!(err, StreamError::Malformed { .. }));
    }

    use std::io::Cursor;
}
