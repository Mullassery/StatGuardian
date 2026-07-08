use polars::prelude::*;
use serde::{Deserialize, Serialize};
use statguardian_core::ast::{Severity, StatFn, StatsRule};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    pub column: String,
    pub stat: String,
    pub reference_value: f64,
    pub current_value: f64,
    pub drift: f64,
    pub threshold: f64,
    pub passed: bool,
    pub severity: Severity,
    pub psi: Option<f64>,
    pub ks_stat: Option<f64>,
}

pub struct DriftEngine;

impl DriftEngine {
    pub fn evaluate(
        current: &DataFrame,
        reference: &DataFrame,
        rules: &[StatsRule],
    ) -> Vec<DriftResult> {
        rules
            .iter()
            .filter_map(|r| evaluate_rule(current, reference, r))
            .collect()
    }

    pub fn psi(reference: &Series, current: &Series, bins: usize) -> f64 {
        let ref_vals = to_f64_vec(reference);
        let cur_vals = to_f64_vec(current);
        if ref_vals.is_empty() || cur_vals.is_empty() {
            return 0.0;
        }

        let lo = ref_vals
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
            .min(cur_vals.iter().cloned().fold(f64::INFINITY, f64::min));
        let hi = ref_vals
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
            .max(cur_vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max));

        if (hi - lo).abs() < f64::EPSILON {
            return 0.0;
        }

        let width = (hi - lo) / bins as f64;
        let ref_buckets = bucket(&ref_vals, lo, width, bins);
        let cur_buckets = bucket(&cur_vals, lo, width, bins);
        let ref_n = ref_vals.len() as f64;
        let cur_n = cur_vals.len() as f64;

        ref_buckets
            .iter()
            .zip(cur_buckets.iter())
            .map(|(&r, &c)| {
                let p_ref = (r as f64 / ref_n).max(1e-9);
                let p_cur = (c as f64 / cur_n).max(1e-9);
                (p_cur - p_ref) * (p_cur / p_ref).ln()
            })
            .sum()
    }

    pub fn ks_statistic(reference: &Series, current: &Series) -> f64 {
        let mut ref_vals = to_f64_vec(reference);
        let mut cur_vals = to_f64_vec(current);
        if ref_vals.is_empty() || cur_vals.is_empty() {
            return 0.0;
        }
        ref_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        cur_vals.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n_ref = ref_vals.len() as f64;
        let n_cur = cur_vals.len() as f64;
        let mut i = 0usize;
        let mut j = 0usize;
        let mut max_diff: f64 = 0.0;

        while i < ref_vals.len() || j < cur_vals.len() {
            let rv = ref_vals.get(i).cloned().unwrap_or(f64::INFINITY);
            let cv = cur_vals.get(j).cloned().unwrap_or(f64::INFINITY);
            // Advance whichever side(s) hold the smaller current value
            if rv <= cv {
                while i < ref_vals.len() && ref_vals[i] == rv {
                    i += 1;
                }
            }
            if cv <= rv {
                while j < cur_vals.len() && cur_vals[j] == cv {
                    j += 1;
                }
            }
            max_diff = max_diff.max((i as f64 / n_ref - j as f64 / n_cur).abs());
        }
        max_diff
    }
}

fn evaluate_rule(
    current: &DataFrame,
    reference: &DataFrame,
    rule: &StatsRule,
) -> Option<DriftResult> {
    let cur_col = current.column(&rule.column).ok()?;
    let ref_col = reference.column(&rule.column).ok()?;
    let cur_series: &Series = cur_col.as_series()?;
    let ref_series: &Series = ref_col.as_series()?;

    let cur_val = compute_stat(cur_series, &rule.stat)?;
    let ref_val = compute_stat(ref_series, &rule.stat)?;

    let drift = if ref_val.abs() < 1e-9 {
        (cur_val - ref_val).abs()
    } else {
        (cur_val - ref_val).abs() / ref_val.abs()
    };

    let passed = rule.op.evaluate(drift, rule.threshold);

    let psi = if is_numeric(cur_series) {
        Some(DriftEngine::psi(ref_series, cur_series, 10))
    } else {
        None
    };
    let ks_stat = if is_numeric(cur_series) {
        Some(DriftEngine::ks_statistic(ref_series, cur_series))
    } else {
        None
    };

    Some(DriftResult {
        column: rule.column.clone(),
        stat: rule.stat.to_string(),
        reference_value: ref_val,
        current_value: cur_val,
        drift,
        threshold: rule.threshold,
        passed,
        severity: rule.severity.clone(),
        psi,
        ks_stat,
    })
}

fn compute_stat(series: &Series, stat: &StatFn) -> Option<f64> {
    let float_s = series.cast(&DataType::Float64).ok()?;
    let ca = float_s.f64().ok()?;
    match stat {
        StatFn::Mean => ca.mean(),
        StatFn::Std => ca.std(1),
        StatFn::Min => ca.min(),
        StatFn::Max => ca.max(),
        StatFn::Median => quantile_manual(ca, 0.5),
        StatFn::P05 => quantile_manual(ca, 0.05),
        StatFn::P95 => quantile_manual(ca, 0.95),
        StatFn::P99 => quantile_manual(ca, 0.99),
        StatFn::P999 => quantile_manual(ca, 0.999),
    }
}

fn quantile_manual(ca: &Float64Chunked, q: f64) -> Option<f64> {
    let mut vals: Vec<f64> = ca.iter().flatten().collect();
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (q * (vals.len() as f64 - 1.0)).round() as usize;
    Some(vals[idx.min(vals.len() - 1)])
}

fn is_numeric(s: &Series) -> bool {
    matches!(
        s.dtype(),
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
    )
}

fn to_f64_vec(series: &Series) -> Vec<f64> {
    series
        .cast(&DataType::Float64)
        .ok()
        .and_then(|s| s.f64().ok().map(|ca| ca.iter().flatten().collect()))
        .unwrap_or_default()
}

fn bucket(vals: &[f64], lo: f64, width: f64, bins: usize) -> Vec<u32> {
    let mut counts = vec![0u32; bins];
    for &v in vals {
        let idx = ((v - lo) / width) as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_series(vals: &[f64]) -> Series {
        Series::new("x".into(), vals)
    }

    #[test]
    fn test_psi_identical_distributions() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let psi = DriftEngine::psi(&s, &s.clone(), 5);
        assert!(psi < 0.01);
    }

    #[test]
    fn test_ks_identical_distributions() {
        let s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let ks = DriftEngine::ks_statistic(&s, &s.clone());
        assert!(ks < 0.01);
    }

    #[test]
    fn test_ks_very_different_distributions() {
        let ref_s = make_series(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let cur_s = make_series(&[10.0, 20.0, 30.0, 40.0, 50.0]);
        let ks = DriftEngine::ks_statistic(&ref_s, &cur_s);
        assert!(ks > 0.5);
    }
}
