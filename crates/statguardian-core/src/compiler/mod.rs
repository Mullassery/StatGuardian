pub mod dag;
pub mod optimizer;

use crate::ast::*;
use crate::compiler::dag::{DagNode, ExecutionDag};
use crate::compiler::optimizer::Optimizer;
use crate::error::CoreResult;

/// Compiles a `DataContract` into an optimized `ExecutionDag`.
pub struct Compiler {
    optimizer: Optimizer,
}

impl Default for Compiler {
    fn default() -> Self {
        Self { optimizer: Optimizer::default() }
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compile(&self, contract: &DataContract) -> CoreResult<ExecutionDag> {
        let mut nodes = Vec::new();

        // Schema → DAG nodes
        for field in &contract.schema {
            nodes.push(DagNode::Profile { column: field.name.clone() });
            nodes.push(DagNode::TypeCheck {
                column: field.name.clone(),
                expected_type: field.data_type.to_string(),
                coerce: field.constraints.contains(&Constraint::Coerce),
                severity: Severity::Blocking,
            });

            for constraint in &field.constraints {
                let col = field.name.clone();
                let node = match constraint {
                    Constraint::NotNull | Constraint::PrimaryKey => Some(DagNode::NullCheck {
                        column: col,
                        severity: Severity::Error,
                    }),
                    Constraint::Unique => Some(DagNode::UniquenessCheck {
                        column: col,
                        severity: Severity::Error,
                    }),
                    Constraint::Positive => Some(DagNode::PositiveCheck {
                        column: col,
                        severity: Severity::Error,
                    }),
                    Constraint::Negative => Some(DagNode::NegativeCheck {
                        column: col,
                        severity: Severity::Error,
                    }),
                    Constraint::Regex { pattern } => Some(DagNode::RegexCheck {
                        column: col,
                        pattern: pattern.clone(),
                        severity: Severity::Error,
                    }),
                    Constraint::Between { min, max } => Some(DagNode::RangeCheck {
                        column: col,
                        min: Some(*min),
                        max: Some(*max),
                        severity: Severity::Error,
                    }),
                    Constraint::Min { value } => Some(DagNode::RangeCheck {
                        column: col,
                        min: Some(*value),
                        max: None,
                        severity: Severity::Error,
                    }),
                    Constraint::Max { value } => Some(DagNode::RangeCheck {
                        column: col,
                        min: None,
                        max: Some(*value),
                        severity: Severity::Error,
                    }),
                    Constraint::Len { min, max } => Some(DagNode::LenCheck {
                        column: col,
                        min: *min,
                        max: *max,
                        severity: Severity::Error,
                    }),
                    Constraint::Enum { values } => Some(DagNode::EnumCheck {
                        column: col,
                        allowed: values.clone(),
                        severity: Severity::Error,
                    }),
                    Constraint::Coerce | Constraint::ForeignKey { .. } => None,
                };
                if let Some(n) = node {
                    nodes.push(n);
                }
            }
        }

        // Quality rules → DAG nodes
        for rule in &contract.quality_rules {
            nodes.push(DagNode::QualityMetricCheck {
                column: rule.column.clone(),
                metric: rule.metric.clone(),
                op: rule.op.clone(),
                threshold: rule.threshold,
                severity: rule.severity.clone(),
            });
        }

        // Stats / drift rules → DAG nodes
        for rule in &contract.stats_rules {
            nodes.push(DagNode::DriftCheck {
                column: rule.column.clone(),
                stat: rule.stat.clone(),
                op: rule.op.clone(),
                threshold: rule.threshold,
                severity: rule.severity.clone(),
            });
        }

        // Anomaly rules → DAG nodes
        for rule in &contract.anomaly_rules {
            let node = match rule.function {
                AnomalyFn::DetectOutliers => DagNode::OutlierDetection {
                    column: rule.column.clone(),
                    method: rule.args.get("method").cloned().unwrap_or_else(|| "iqr".into()),
                    severity: rule.severity.clone(),
                },
                AnomalyFn::DetectDuplicates => DagNode::DuplicateDetection {
                    column: rule.column.clone(),
                    severity: rule.severity.clone(),
                },
                AnomalyFn::DetectNulls => DagNode::NullAnomalyDetection {
                    column: rule.column.clone(),
                    severity: rule.severity.clone(),
                },
                AnomalyFn::DetectCardinalityExplosion => DagNode::CardinalityCheck {
                    column: rule.column.clone(),
                    severity: rule.severity.clone(),
                },
                AnomalyFn::DetectPatternBreaks => DagNode::RegexCheck {
                    column: rule.column.clone(),
                    pattern: rule.args.get("pattern").cloned().unwrap_or_default(),
                    severity: rule.severity.clone(),
                },
            };
            nodes.push(node);
        }

        Ok(self.optimizer.optimize(nodes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_compile_produces_dag() {
        let dsl = r#"
dataset orders {
    schema {
        id: int, not_null, unique
        amount: float, positive
    }
    quality {
        completeness(id) > 0.99
    }
}
"#;
        let contracts = parser::parse(dsl).unwrap();
        let compiler = Compiler::new();
        let dag = compiler.compile(&contracts[0]).unwrap();
        assert!(dag.node_count() > 0);
        assert!(dag.referenced_columns.contains(&"id".to_string()));
        assert!(dag.referenced_columns.contains(&"amount".to_string()));
    }
}
