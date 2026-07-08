use crate::ast::{ComparisonOp, MetricFn, Severity, StatFn};
use serde::{Deserialize, Serialize};

/// A single node in the execution DAG.
/// Each node is a pure, side-effect-free unit of work against a column or dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DagNode {
    // ── Schema checks ────────────────────────────────────────────────────────
    TypeCheck {
        column: String,
        expected_type: String,
        coerce: bool,
        severity: Severity,
    },
    NullCheck {
        column: String,
        severity: Severity,
    },
    UniquenessCheck {
        column: String,
        severity: Severity,
    },
    RegexCheck {
        column: String,
        pattern: String,
        severity: Severity,
    },
    RangeCheck {
        column: String,
        min: Option<f64>,
        max: Option<f64>,
        severity: Severity,
    },
    LenCheck {
        column: String,
        min: usize,
        max: usize,
        severity: Severity,
    },
    EnumCheck {
        column: String,
        allowed: Vec<String>,
        severity: Severity,
    },
    PositiveCheck {
        column: String,
        severity: Severity,
    },
    NegativeCheck {
        column: String,
        severity: Severity,
    },

    // ── Quality metrics ──────────────────────────────────────────────────────
    QualityMetricCheck {
        column: String,
        metric: MetricFn,
        op: ComparisonOp,
        threshold: f64,
        severity: Severity,
    },

    // ── Drift / stats ────────────────────────────────────────────────────────
    DriftCheck {
        column: String,
        stat: StatFn,
        op: ComparisonOp,
        threshold: f64,
        severity: Severity,
    },

    // ── Anomaly detection ─────────────────────────────────────────────────────
    OutlierDetection {
        column: String,
        method: String,
        severity: Severity,
    },
    DuplicateDetection {
        column: String,
        severity: Severity,
    },
    NullAnomalyDetection {
        column: String,
        severity: Severity,
    },
    CardinalityCheck {
        column: String,
        severity: Severity,
    },

    // ── Profiling (always runs, free) ─────────────────────────────────────────
    Profile {
        column: String,
    },
}

impl DagNode {
    pub fn column(&self) -> &str {
        match self {
            DagNode::TypeCheck { column, .. }
            | DagNode::NullCheck { column, .. }
            | DagNode::UniquenessCheck { column, .. }
            | DagNode::RegexCheck { column, .. }
            | DagNode::RangeCheck { column, .. }
            | DagNode::LenCheck { column, .. }
            | DagNode::EnumCheck { column, .. }
            | DagNode::PositiveCheck { column, .. }
            | DagNode::NegativeCheck { column, .. }
            | DagNode::QualityMetricCheck { column, .. }
            | DagNode::DriftCheck { column, .. }
            | DagNode::OutlierDetection { column, .. }
            | DagNode::DuplicateDetection { column, .. }
            | DagNode::NullAnomalyDetection { column, .. }
            | DagNode::CardinalityCheck { column, .. }
            | DagNode::Profile { column } => column.as_str(),
        }
    }

    /// Cost estimate for ordering (lower = cheaper, should run first to short-circuit).
    pub fn cost(&self) -> u8 {
        match self {
            DagNode::TypeCheck { .. } => 1,
            DagNode::NullCheck { .. } => 1,
            DagNode::PositiveCheck { .. } => 2,
            DagNode::NegativeCheck { .. } => 2,
            DagNode::RangeCheck { .. } => 2,
            DagNode::LenCheck { .. } => 2,
            DagNode::EnumCheck { .. } => 3,
            DagNode::RegexCheck { .. } => 4,
            DagNode::UniquenessCheck { .. } => 5,
            DagNode::QualityMetricCheck { .. } => 5,
            DagNode::Profile { .. } => 6,
            DagNode::CardinalityCheck { .. } => 6,
            DagNode::DriftCheck { .. } => 7,
            DagNode::OutlierDetection { .. } => 8,
            DagNode::DuplicateDetection { .. } => 8,
            DagNode::NullAnomalyDetection { .. } => 3,
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            DagNode::TypeCheck { severity, .. }
            | DagNode::NullCheck { severity, .. }
            | DagNode::UniquenessCheck { severity, .. }
            | DagNode::RegexCheck { severity, .. }
            | DagNode::RangeCheck { severity, .. }
            | DagNode::LenCheck { severity, .. }
            | DagNode::EnumCheck { severity, .. }
            | DagNode::PositiveCheck { severity, .. }
            | DagNode::NegativeCheck { severity, .. }
            | DagNode::QualityMetricCheck { severity, .. }
            | DagNode::DriftCheck { severity, .. }
            | DagNode::OutlierDetection { severity, .. }
            | DagNode::DuplicateDetection { severity, .. }
            | DagNode::NullAnomalyDetection { severity, .. }
            | DagNode::CardinalityCheck { severity, .. } => severity.clone(),
            DagNode::Profile { .. } => Severity::Info,
        }
    }
}

/// The compiled execution plan: a flat, ordered list of nodes grouped by column.
/// Edges are implicit — same-column nodes share a single column scan pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDag {
    /// Ordered list of nodes (optimizer has already sorted by cost).
    pub nodes: Vec<DagNode>,
    /// Column names referenced by this DAG (for pre-fetching / schema validation).
    pub referenced_columns: Vec<String>,
}

impl ExecutionDag {
    pub fn new(nodes: Vec<DagNode>) -> Self {
        let mut referenced_columns: Vec<String> =
            nodes.iter().map(|n| n.column().to_string()).collect();
        referenced_columns.sort();
        referenced_columns.dedup();

        Self {
            nodes,
            referenced_columns,
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
