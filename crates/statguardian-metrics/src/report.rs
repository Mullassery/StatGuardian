use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use statguardian_core::ast::Severity;
use statguardian_stats::{ColumnProfile, DriftResult};
use statguardian_validators::Violation;

/// Composite health score for a dataset (0.0 = complete failure, 1.0 = perfect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetHealthScore {
    /// Overall score in [0, 1].
    pub score: f64,
    /// Weighted sub-scores.
    pub schema_score: f64,
    pub quality_score: f64,
    pub drift_score: f64,
    /// Human-readable grade: A/B/C/D/F.
    pub grade: String,
}

impl DatasetHealthScore {
    pub fn compute(
        violations: &[Violation],
        drift_results: &[DriftResult],
        total_checks: usize,
    ) -> Self {
        let blocking = violations
            .iter()
            .filter(|v| v.severity == Severity::Blocking)
            .count();
        let errors = violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let warnings = violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        // Penalty weights: blocking=1.0, error=0.5, warning=0.1
        let penalty = (blocking as f64 * 1.0 + errors as f64 * 0.5 + warnings as f64 * 0.1)
            / total_checks.max(1) as f64;

        let schema_score = 1.0 - penalty.min(1.0);

        let drift_failing = drift_results.iter().filter(|r| !r.passed).count();
        let drift_score = if drift_results.is_empty() {
            1.0
        } else {
            1.0 - (drift_failing as f64 / drift_results.len() as f64)
        };

        let score = (schema_score * 0.7 + drift_score * 0.3).clamp(0.0, 1.0);

        let grade = match score {
            s if s >= 0.95 => "A",
            s if s >= 0.85 => "B",
            s if s >= 0.70 => "C",
            s if s >= 0.50 => "D",
            _ => "F",
        }
        .to_string();

        Self {
            score,
            schema_score,
            quality_score: schema_score, // same weight in default model
            drift_score,
            grade,
        }
    }
}

/// Per-check execution result for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check: String,
    pub column: String,
    pub passed: bool,
    pub severity: Severity,
    pub message: Option<String>,
}

/// Complete structured report produced by a single contract execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub id: String,
    pub dataset: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: u64,

    /// Total rows processed.
    pub row_count: usize,

    /// Schema and quality violations.
    pub violations: Vec<Violation>,

    /// Drift analysis results (empty when no reference is provided).
    pub drift_results: Vec<DriftResult>,

    /// Per-column statistical profiles.
    pub column_profiles: Vec<ColumnProfile>,

    /// Aggregate health score.
    pub health: DatasetHealthScore,

    /// Whether all blocking checks passed.
    pub passed: bool,
}

impl ValidationReport {
    pub fn new(
        dataset: impl Into<String>,
        row_count: usize,
        violations: Vec<Violation>,
        drift_results: Vec<DriftResult>,
        column_profiles: Vec<ColumnProfile>,
        total_checks: usize,
        duration_ms: u64,
    ) -> Self {
        let health = DatasetHealthScore::compute(&violations, &drift_results, total_checks);
        let passed = !violations.iter().any(|v| v.severity == Severity::Blocking);

        Self {
            id: Uuid::new_v4().to_string(),
            dataset: dataset.into(),
            executed_at: Utc::now(),
            duration_ms,
            row_count,
            violations,
            drift_results,
            column_profiles,
            health,
            passed,
        }
    }

    /// Render as compact JSON (machine-readable).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Render as pretty-printed JSON.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Prometheus-compatible text exposition format.
    pub fn to_prometheus(&self) -> String {
        let mut out = String::new();
        let labels = format!("dataset=\"{}\"", self.dataset);

        out.push_str(&format!(
            "# HELP statguard_health_score Dataset health score (0–1)\n\
             # TYPE statguard_health_score gauge\n\
             statguard_health_score{{{labels}}} {:.4}\n",
            self.health.score
        ));
        out.push_str(&format!(
            "# HELP statguard_violations_total Total violations\n\
             # TYPE statguard_violations_total counter\n\
             statguard_violations_total{{{labels}}} {}\n",
            self.violations.len()
        ));
        out.push_str(&format!(
            "# HELP statguard_row_count Rows processed\n\
             # TYPE statguard_row_count gauge\n\
             statguard_row_count{{{labels}}} {}\n",
            self.row_count
        ));
        out.push_str(&format!(
            "# HELP statguard_execution_ms Execution time in milliseconds\n\
             # TYPE statguard_execution_ms gauge\n\
             statguard_execution_ms{{{labels}}} {}\n",
            self.duration_ms
        ));

        for drift in &self.drift_results {
            let dlabels = format!("dataset=\"{}\",column=\"{}\"", self.dataset, drift.column);
            out.push_str(&format!(
                "statguard_drift{{{dlabels},stat=\"{}\"}} {:.6}\n",
                drift.stat, drift.drift
            ));
            if let Some(psi) = drift.psi {
                out.push_str(&format!("statguard_psi{{{dlabels}}} {psi:.6}\n"));
            }
        }

        out
    }

    /// Summary for human display.
    pub fn summary(&self) -> ExecutionSummary {
        let blocking = self
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Blocking)
            .count();
        let errors = self
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let warnings = self
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        ExecutionSummary {
            dataset: self.dataset.clone(),
            passed: self.passed,
            health_score: self.health.score,
            grade: self.health.grade.clone(),
            row_count: self.row_count,
            violation_count: self.violations.len(),
            blocking_count: blocking,
            error_count: errors,
            warning_count: warnings,
            drift_failing: self.drift_results.iter().filter(|r| !r.passed).count(),
            duration_ms: self.duration_ms,
        }
    }
}

/// Lightweight summary (suitable for CI output or logging).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub dataset: String,
    pub passed: bool,
    pub health_score: f64,
    pub grade: String,
    pub row_count: usize,
    pub violation_count: usize,
    pub blocking_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub drift_failing: usize,
    pub duration_ms: u64,
}

impl std::fmt::Display for ExecutionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.passed { "PASS ✓" } else { "FAIL ✗" };
        write!(
            f,
            "[StatGuard] {status} | dataset={} | score={:.2} ({}) | rows={} | \
             violations={} (blocking={}, errors={}, warnings={}) | drift_failing={} | {}ms",
            self.dataset,
            self.health_score,
            self.grade,
            self.row_count,
            self.violation_count,
            self.blocking_count,
            self.error_count,
            self.warning_count,
            self.drift_failing,
            self.duration_ms,
        )
    }
}
