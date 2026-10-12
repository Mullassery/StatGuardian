use serde::{Deserialize, Serialize};

use statguardian_core::ast::Severity;

use crate::sanitize::SanitizationSummary;

/// A single validation finding attributed to a global record index (0-based).
/// `record` is `None` for aggregate findings that apply to a whole batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamFinding {
    pub record: Option<u64>,
    pub batch: usize,
    pub column: String,
    pub check: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MalformedKind {
    Json,
    Utf8,
    Oversized,
}

/// A record that could not be parsed or was rejected by a size limit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MalformedRecord {
    pub record: u64,
    pub kind: MalformedKind,
    pub reason: String,
    /// Raw line preview. Only populated when the caller explicitly opted in
    /// via `StreamOptions::allow_raw_diagnostics`; otherwise always `None`.
    pub preview: Option<String>,
}

/// The first hard limit that stopped the stream early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamLimitHit {
    pub limit: String,
    pub value: u64,
}

/// The complete result of validating an NDJSON stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamReport {
    pub dataset: String,
    pub records_read: u64,
    pub records_validated: u64,
    pub malformed_count: u64,
    pub oversized_count: u64,
    pub batches: usize,
    pub bytes_consumed: u64,
    /// True when a resource limit stopped the run before end-of-input.
    pub truncated: bool,
    /// True when there are no blocking findings.
    pub passed: bool,
    pub duration_ms: u64,
    pub findings: Vec<StreamFinding>,
    pub malformed: Vec<MalformedRecord>,
    pub sanitization: SanitizationSummary,
    pub limit_hit: Option<StreamLimitHit>,
}

impl StreamReport {
    pub fn findings_with_severity(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    pub fn blocking_count(&self) -> usize {
        self.findings_with_severity(Severity::Blocking)
    }

    pub fn warning_count(&self) -> usize {
        self.findings_with_severity(Severity::Warning)
    }

    /// Exit code for CLI/library callers.
    ///
    /// `0` passed, `1` validation failed. Hard execution errors are returned
    /// as `Err` from the validator and map to `2` at the CLI layer.
    pub fn exit_code(&self, fail_on_warning: bool) -> i32 {
        let fail = !self.passed || (fail_on_warning && !self.findings.is_empty());
        if fail {
            1
        } else {
            0
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Prometheus text exposition format.
    pub fn to_prometheus(&self) -> String {
        let labels = format!("dataset=\"{}\"", self.dataset);
        let mut out = String::new();
        out.push_str(&format!(
            "# HELP statguard_stream_passed Stream validation passed (0/1)\n\
             # TYPE statguard_stream_passed gauge\n\
             statguard_stream_passed{{{labels}}} {}\n",
            if self.passed { 1 } else { 0 }
        ));
        out.push_str(&format!(
            "# HELP statguard_stream_records_read Records read\n\
             # TYPE statguard_stream_records_read counter\n\
             statguard_stream_records_read{{{labels}}} {}\n",
            self.records_read
        ));
        out.push_str(&format!(
            "# HELP statguard_stream_records_validated Records validated\n\
             # TYPE statguard_stream_records_validated counter\n\
             statguard_stream_records_validated{{{labels}}} {}\n",
            self.records_validated
        ));
        out.push_str(&format!(
            "# HELP statguard_stream_findings_total Findings\n\
             # TYPE statguard_stream_findings_total counter\n\
             statguard_stream_findings_total{{{labels}}} {}\n",
            self.findings.len()
        ));
        out.push_str(&format!(
            "# HELP statguard_stream_malformed_total Malformed records\n\
             # TYPE statguard_stream_malformed_total counter\n\
             statguard_stream_malformed_total{{{labels}}} {}\n",
            self.malformed_count
        ));
        out.push_str(&format!(
            "# HELP statguard_stream_sanitized_total Sanitization events\n\
             # TYPE statguard_stream_sanitized_total counter\n\
             statguard_stream_sanitized_total{{{labels}}} {}\n",
            self.sanitization.total_events
        ));
        out
    }

    /// One-line human summary suitable for a terminal or CI log.
    pub fn summary(&self) -> String {
        let status = if self.passed { "PASS" } else { "FAIL" };
        let trunc = if self.truncated { " | truncated" } else { "" };
        format!(
            "[StreamGuard] {status} | dataset={} | records={}/{} | findings={} \
             (blocking={}, warnings={}) | malformed={} | sanitized={} | {}ms{trunc}",
            self.dataset,
            self.records_validated,
            self.records_read,
            self.findings.len(),
            self.blocking_count(),
            self.warning_count(),
            self.malformed_count,
            self.sanitization.total_events,
            self.duration_ms,
        )
    }
}
