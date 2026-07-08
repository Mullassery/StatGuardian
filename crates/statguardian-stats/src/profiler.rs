use crate::hll::HyperLogLog;
use polars::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub dtype: String,
    pub row_count: usize,
    pub null_count: usize,
    pub null_rate: f64,
    pub distinct_count: u64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub mean: Option<f64>,
    pub std: Option<f64>,
    pub median: Option<f64>,
    pub p05: Option<f64>,
    pub p25: Option<f64>,
    pub p75: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
    pub histogram: Option<Vec<(f64, f64, u32)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetProfile {
    pub row_count: usize,
    pub column_count: usize,
    pub columns: Vec<ColumnProfile>,
}

pub struct Profiler;

impl Profiler {
    pub fn profile(df: &DataFrame) -> DatasetProfile {
        use rayon::prelude::*;

        let row_count = df.height();
        let column_count = df.width();

        let columns: Vec<ColumnProfile> = df.get_columns().par_iter().map(profile_column).collect();

        DatasetProfile {
            row_count,
            column_count,
            columns,
        }
    }
}

fn profile_column(col: &Column) -> ColumnProfile {
    let name = col.name().to_string();
    let dtype = format!("{:?}", col.dtype());
    let row_count = col.len();
    let null_count = col.null_count();
    let null_rate = if row_count > 0 {
        null_count as f64 / row_count as f64
    } else {
        0.0
    };

    // HLL distinct count — cast to String for universal hashing
    let distinct_count = {
        let mut hll = HyperLogLog::new();
        if let Ok(as_str) = col.cast(&DataType::String) {
            if let Some(s) = as_str.as_series() {
                if let Ok(ca) = s.str() {
                    for v in ca.iter().flatten() {
                        hll.add_str(v);
                    }
                }
            }
        }
        hll.cardinality()
    };

    let (min, max, mean, std, median, p05, p25, p75, p95, p99, histogram) = match col.as_series() {
        Some(series) if is_numeric(series) => numeric_stats(series),
        _ => (
            None, None, None, None, None, None, None, None, None, None, None,
        ),
    };

    ColumnProfile {
        name,
        dtype,
        row_count,
        null_count,
        null_rate,
        distinct_count,
        min,
        max,
        mean,
        std,
        median,
        p05,
        p25,
        p75,
        p95,
        p99,
        histogram,
    }
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

/// Linear-interpolation quantile without relying on polars Scalar extraction.
fn quantile_manual(ca: &Float64Chunked, q: f64) -> Option<f64> {
    let mut vals: Vec<f64> = ca.iter().flatten().collect();
    if vals.is_empty() {
        return None;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (q * (vals.len() as f64 - 1.0)).round() as usize;
    Some(vals[idx.min(vals.len() - 1)])
}

#[allow(clippy::type_complexity)]
fn numeric_stats(
    series: &Series,
) -> (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<Vec<(f64, f64, u32)>>,
) {
    let float_s = match series.cast(&DataType::Float64) {
        Ok(s) => s,
        Err(_) => {
            return (
                None, None, None, None, None, None, None, None, None, None, None,
            )
        }
    };
    let ca = match float_s.f64() {
        Ok(c) => c,
        Err(_) => {
            return (
                None, None, None, None, None, None, None, None, None, None, None,
            )
        }
    };

    let min = ca.min();
    let max = ca.max();
    let mean = ca.mean();
    let std = ca.std(1);
    let median = quantile_manual(ca, 0.5);
    let p05 = quantile_manual(ca, 0.05);
    let p25 = quantile_manual(ca, 0.25);
    let p75 = quantile_manual(ca, 0.75);
    let p95 = quantile_manual(ca, 0.95);
    let p99 = quantile_manual(ca, 0.99);
    let histogram = Some(build_histogram(ca, min, max, 10));

    (
        min, max, mean, std, median, p05, p25, p75, p95, p99, histogram,
    )
}

fn build_histogram(
    ca: &Float64Chunked,
    min: Option<f64>,
    max: Option<f64>,
    bins: usize,
) -> Vec<(f64, f64, u32)> {
    let (lo, hi) = match (min, max) {
        (Some(a), Some(b)) if a < b => (a, b),
        _ => return vec![],
    };
    let width = (hi - lo) / bins as f64;
    let mut counts = vec![0u32; bins];
    for v in ca.iter().flatten() {
        let idx = ((v - lo) / width) as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    (0..bins)
        .map(|i| {
            let lower = lo + i as f64 * width;
            (lower, lower + width, counts[i])
        })
        .collect()
}
