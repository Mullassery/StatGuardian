use std::time::Instant;
use polars::prelude::*;
use rayon::prelude::*;

use statguardian_core::ast::Severity;
use statguardian_core::compiler::dag::{DagNode, ExecutionDag};
use statguardian_core::compiler::optimizer::Optimizer;
use statguardian_core::DataContract;
use statguardian_metrics::report::ValidationReport;
use statguardian_stats::{DriftEngine, Profiler};
use statguardian_validators::{SchemaValidator, RuleEngine, Violation};

pub struct BatchExecutor;

impl BatchExecutor {
    pub fn execute(
        contract: &DataContract,
        dag: &ExecutionDag,
        df: &DataFrame,
        reference: Option<&DataFrame>,
    ) -> ValidationReport {
        let t0 = Instant::now();

        let schema_violations = SchemaValidator::validate(df, &contract.schema);
        let rule_violations       = RuleEngine::evaluate(df, &contract.quality_rules);
        let cross_col_violations = RuleEngine::evaluate_cross_column(df, &contract.cross_column_rules);

        let dag_violations: Vec<Violation> = {
            let groups = Optimizer::group_by_column(dag);
            groups
                .into_par_iter()
                .flat_map(|(_col, nodes)| {
                    let mut col_violations = Vec::new();
                    let mut abort = false;
                    for node in nodes {
                        if abort { break; }
                        let vs = execute_node(node, df);
                        if vs.iter().any(|v| v.severity == Severity::Blocking) {
                            abort = true;
                        }
                        col_violations.extend(vs);
                    }
                    col_violations
                })
                .collect()
        };

        let mut all_violations: Vec<Violation> = schema_violations
            .into_iter()
            .chain(rule_violations)
            .chain(cross_col_violations)
            .chain(dag_violations)
            .collect();
        all_violations.sort_by(|a, b| a.column.cmp(&b.column).then(a.check.cmp(&b.check)));
        all_violations.dedup_by(|a, b| a.column == b.column && a.check == b.check);

        let drift_results = match reference {
            Some(ref_df) => DriftEngine::evaluate(df, ref_df, &contract.stats_rules),
            None => vec![],
        };

        let profile       = Profiler::profile(df);
        let duration_ms   = t0.elapsed().as_millis() as u64;
        let total_checks  = dag.node_count() + contract.quality_rules.len();

        ValidationReport::new(
            &contract.name,
            df.height(),
            all_violations,
            drift_results,
            profile.columns,
            total_checks,
            duration_ms,
        )
    }
}

// ── Helper: get Series from a DataFrame column by name ────────────────────────

fn get_series(df: &DataFrame, name: &str) -> Option<Series> {
    df.column(name).ok().and_then(|c| c.as_series().cloned())
}

// ── DAG node execution ────────────────────────────────────────────────────────

fn execute_node(node: &DagNode, df: &DataFrame) -> Vec<Violation> {
    match node {
        DagNode::Profile { .. } => vec![],

        DagNode::TypeCheck { .. } => vec![], // handled by SchemaValidator

        DagNode::NullCheck { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let n = s.null_count();
            if n > 0 {
                vec![Violation::new(column, "null_check",
                    format!("{n} null(s) in '{column}'"), severity.clone())
                    .with_values(n as f64, 0.0)]
            } else { vec![] }
        }

        DagNode::UniquenessCheck { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let n_total  = s.len();
            let n_unique = s.n_unique().unwrap_or(n_total);
            if n_unique != n_total {
                vec![Violation::new(column, "uniqueness",
                    format!("{} duplicate(s) in '{column}'", n_total - n_unique),
                    severity.clone())
                    .with_values(n_unique as f64, n_total as f64)]
            } else { vec![] }
        }

        DagNode::RegexCheck { column, pattern, severity } => {
            use regex::Regex;
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => return vec![Violation::new(column, "regex",
                    format!("invalid regex '{pattern}': {e}"), severity.clone())],
            };
            if let Ok(ca) = s.str() {
                let failing: Vec<usize> = ca.iter().enumerate()
                    .filter_map(|(i, v)| v.filter(|s| !re.is_match(s)).map(|_| i))
                    .collect();
                if !failing.is_empty() {
                    return vec![Violation::new(column, "regex",
                        format!("{} value(s) don't match '{pattern}'", failing.len()),
                        severity.clone()).with_rows(failing)];
                }
            }
            vec![]
        }

        DagNode::RangeCheck { column, min, max, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            if let Ok(fs) = s.cast(&DataType::Float64) {
                if let Ok(ca) = fs.f64() {
                    let failing: Vec<usize> = ca.iter().enumerate()
                        .filter_map(|(i, v)| {
                            v.filter(|&x| min.map(|m| x < m).unwrap_or(false)
                                || max.map(|m| x > m).unwrap_or(false))
                            .map(|_| i)
                        }).collect();
                    if !failing.is_empty() {
                        return vec![Violation::new(column, "range_check",
                            format!("{} value(s) out of range in '{column}'", failing.len()),
                            severity.clone()).with_rows(failing)];
                    }
                }
            }
            vec![]
        }

        DagNode::LenCheck { column, min, max, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            if let Ok(ca) = s.str() {
                let failing: Vec<usize> = ca.iter().enumerate()
                    .filter_map(|(i, v)| {
                        v.filter(|s| { let l = s.len(); l < *min || l > *max }).map(|_| i)
                    }).collect();
                if !failing.is_empty() {
                    return vec![Violation::new(column, "len_check",
                        format!("{} value(s) with invalid length in '{column}'", failing.len()),
                        severity.clone()).with_rows(failing)];
                }
            }
            vec![]
        }

        DagNode::EnumCheck { column, allowed, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            if let Ok(ca) = s.str() {
                let set: std::collections::HashSet<&str> =
                    allowed.iter().map(|s| s.as_str()).collect();
                let failing: Vec<usize> = ca.iter().enumerate()
                    .filter_map(|(i, v)| v.filter(|s| !set.contains(*s)).map(|_| i))
                    .collect();
                if !failing.is_empty() {
                    return vec![Violation::new(column, "enum_check",
                        format!("{} value(s) not in allowed set in '{column}'", failing.len()),
                        severity.clone()).with_rows(failing)];
                }
            }
            vec![]
        }

        DagNode::PositiveCheck { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            if let Ok(fs) = s.cast(&DataType::Float64) {
                if let Ok(ca) = fs.f64() {
                    let failing: Vec<usize> = ca.iter().enumerate()
                        .filter_map(|(i, v)| v.filter(|&x| x <= 0.0).map(|_| i)).collect();
                    if !failing.is_empty() {
                        return vec![Violation::new(column, "positive_check",
                            format!("{} non-positive value(s) in '{column}'", failing.len()),
                            severity.clone()).with_rows(failing)];
                    }
                }
            }
            vec![]
        }

        DagNode::NegativeCheck { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            if let Ok(fs) = s.cast(&DataType::Float64) {
                if let Ok(ca) = fs.f64() {
                    let failing: Vec<usize> = ca.iter().enumerate()
                        .filter_map(|(i, v)| v.filter(|&x| x >= 0.0).map(|_| i)).collect();
                    if !failing.is_empty() {
                        return vec![Violation::new(column, "negative_check",
                            format!("{} non-negative value(s) in '{column}'", failing.len()),
                            severity.clone()).with_rows(failing)];
                    }
                }
            }
            vec![]
        }

        DagNode::QualityMetricCheck { column, metric, op, threshold, severity } => {
            RuleEngine::evaluate(df, &[statguard_core::ast::QualityRule {
                metric: metric.clone(),
                column: column.clone(),
                op: op.clone(),
                threshold: *threshold,
                severity: severity.clone(),
            }])
        }

        DagNode::DriftCheck { .. } => vec![], // handled by DriftEngine

        DagNode::OutlierDetection { column, method, severity } => {
            detect_outliers(column, method, severity, df)
        }

        DagNode::DuplicateDetection { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let n_total  = s.len();
            let n_unique = s.n_unique().unwrap_or(n_total);
            if n_unique < n_total {
                vec![Violation::new(column, "duplicate_detection",
                    format!("{} duplicate(s) in '{column}'", n_total - n_unique),
                    severity.clone())]
            } else { vec![] }
        }

        DagNode::NullAnomalyDetection { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let null_rate = s.null_count() as f64 / s.len().max(1) as f64;
            if null_rate > 0.0 {
                vec![Violation::new(column, "null_anomaly",
                    format!("null rate {:.2}% in '{column}'", null_rate * 100.0),
                    severity.clone()).with_values(null_rate, 0.0)]
            } else { vec![] }
        }

        DagNode::CardinalityCheck { column, severity } => {
            let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
            let n_unique = s.n_unique().unwrap_or(0);
            let n_total  = s.len();
            if n_total > 100 && n_unique as f64 / n_total as f64 > 0.95 {
                vec![Violation::new(column, "cardinality_check",
                    format!("high cardinality in '{column}': {n_unique}/{n_total} unique"),
                    severity.clone())]
            } else { vec![] }
        }
    }
}

fn detect_outliers(column: &str, method: &str, severity: &Severity, df: &DataFrame) -> Vec<Violation> {
    let s = match get_series(df, column) { Some(s) => s, None => return vec![] };
    let float_s = match s.cast(&DataType::Float64) { Ok(f) => f, Err(_) => return vec![] };
    let ca      = match float_s.f64() { Ok(c) => c, Err(_) => return vec![] };

    let mut vals: Vec<(usize, f64)> = ca.iter().enumerate()
        .filter_map(|(i, v)| v.map(|x| (i, x)))
        .collect();

    if vals.is_empty() { return vec![]; }

    let outlier_indices: Vec<usize> = match method {
        "iqr" => {
            let mut sorted: Vec<f64> = vals.iter().map(|(_, x)| *x).collect();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = sorted.len();
            let q1 = sorted[n / 4];
            let q3 = sorted[3 * n / 4];
            let iqr = q3 - q1;
            let lo  = q1 - 1.5 * iqr;
            let hi  = q3 + 1.5 * iqr;
            vals.iter().filter_map(|(i, x)| if *x < lo || *x > hi { Some(*i) } else { None }).collect()
        }
        "zscore" => {
            let n    = vals.len() as f64;
            let mean = vals.iter().map(|(_, x)| x).sum::<f64>() / n;
            let std  = (vals.iter().map(|(_, x)| (x - mean).powi(2)).sum::<f64>() / (n - 1.0))
                .sqrt()
                .max(f64::EPSILON);
            vals.iter().filter_map(|(i, x)| {
                if ((x - mean) / std).abs() > 3.0 { Some(*i) } else { None }
            }).collect()
        }
        _ => vec![],
    };

    if outlier_indices.is_empty() {
        vec![]
    } else {
        vec![Violation::new(column, "outlier_detection",
            format!("{} outlier(s) in '{column}' (method={method})", outlier_indices.len()),
            severity.clone()).with_rows(outlier_indices)]
    }
}

