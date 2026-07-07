use polars::prelude::*;
use regex::Regex;
use statguardian_core::ast::{DataType as ContractType, FieldDef, Severity, Constraint};
use crate::Violation;

pub struct SchemaValidator;

impl SchemaValidator {
    pub fn validate(df: &DataFrame, fields: &[FieldDef]) -> Vec<Violation> {
        let mut violations = Vec::new();
        for field in fields {
            match df.column(&field.name) {
                Err(_) => {
                    violations.push(Violation::new(
                        &field.name, "column_exists",
                        format!("column '{}' not found in dataset", field.name),
                        Severity::Blocking,
                    ));
                }
                Ok(col) => {
                    if let Some(series) = col.as_series() {
                        violations.extend(check_type(series, field));
                        violations.extend(check_constraints(series, &field.constraints, &field.name));
                    }
                }
            }
        }
        violations
    }
}

fn check_type(series: &Series, field: &FieldDef) -> Vec<Violation> {
    let matches = match &field.data_type {
        ContractType::Int => matches!(
            series.dtype(),
            DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
            | DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64
        ),
        ContractType::Float  => matches!(series.dtype(), DataType::Float32 | DataType::Float64),
        ContractType::String => matches!(series.dtype(), DataType::String),
        ContractType::Bool   => matches!(series.dtype(), DataType::Boolean),
        ContractType::Date   => matches!(series.dtype(), DataType::Date),
        ContractType::Datetime => matches!(series.dtype(), DataType::Datetime(_, _)),
        ContractType::Bytes  => matches!(series.dtype(), DataType::Binary),
    };

    if !matches {
        let severity = if field.constraints.contains(&Constraint::Coerce) {
            Severity::Warning
        } else {
            Severity::Blocking
        };
        vec![Violation::new(
            &field.name, "type_check",
            format!("column '{}' has dtype {:?}, expected {}", field.name, series.dtype(), field.data_type),
            severity,
        )]
    } else {
        vec![]
    }
}

fn check_constraints(series: &Series, constraints: &[Constraint], col: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    for constraint in constraints {
        match constraint {
            Constraint::NotNull | Constraint::PrimaryKey => {
                let n = series.null_count();
                if n > 0 {
                    violations.push(
                        Violation::new(col, "not_null",
                            format!("{n} null value(s) in '{col}'"), Severity::Error)
                        .with_values(n as f64, 0.0));
                }
            }
            Constraint::Unique => {
                let n_unique = series.n_unique().unwrap_or(0);
                let n_total  = series.len();
                if n_unique != n_total {
                    violations.push(
                        Violation::new(col, "uniqueness",
                            format!("{} duplicate(s) in '{col}'", n_total - n_unique),
                            Severity::Error)
                        .with_values(n_unique as f64, n_total as f64));
                }
            }
            Constraint::Positive => {
                if let Ok(fs) = series.cast(&DataType::Float64) {
                    if let Ok(ca) = fs.f64() {
                        let failing: Vec<usize> = ca.iter().enumerate()
                            .filter_map(|(i, v)| v.filter(|&x| x <= 0.0).map(|_| i)).collect();
                        if !failing.is_empty() {
                            violations.push(Violation::new(col, "positive",
                                format!("{} non-positive value(s) in '{col}'", failing.len()),
                                Severity::Error).with_rows(failing));
                        }
                    }
                }
            }
            Constraint::Negative => {
                if let Ok(fs) = series.cast(&DataType::Float64) {
                    if let Ok(ca) = fs.f64() {
                        let failing: Vec<usize> = ca.iter().enumerate()
                            .filter_map(|(i, v)| v.filter(|&x| x >= 0.0).map(|_| i)).collect();
                        if !failing.is_empty() {
                            violations.push(Violation::new(col, "negative",
                                format!("{} non-negative value(s) in '{col}'", failing.len()),
                                Severity::Error).with_rows(failing));
                        }
                    }
                }
            }
            Constraint::Between { min, max } => {
                if let Ok(fs) = series.cast(&DataType::Float64) {
                    if let Ok(ca) = fs.f64() {
                        let failing: Vec<usize> = ca.iter().enumerate()
                            .filter_map(|(i, v)| {
                                v.filter(|&x| x < *min || x > *max).map(|_| i)
                            }).collect();
                        if !failing.is_empty() {
                            violations.push(Violation::new(col, "between",
                                format!("{} value(s) outside [{min},{max}] in '{col}'", failing.len()),
                                Severity::Error).with_rows(failing));
                        }
                    }
                }
            }
            Constraint::Min { value } => {
                if let Ok(fs) = series.cast(&DataType::Float64) {
                    if let Ok(ca) = fs.f64() {
                        let failing: Vec<usize> = ca.iter().enumerate()
                            .filter_map(|(i, v)| v.filter(|&x| x < *value).map(|_| i)).collect();
                        if !failing.is_empty() {
                            violations.push(Violation::new(col, "min",
                                format!("{} value(s) below min={value} in '{col}'", failing.len()),
                                Severity::Error).with_rows(failing));
                        }
                    }
                }
            }
            Constraint::Max { value } => {
                if let Ok(fs) = series.cast(&DataType::Float64) {
                    if let Ok(ca) = fs.f64() {
                        let failing: Vec<usize> = ca.iter().enumerate()
                            .filter_map(|(i, v)| v.filter(|&x| x > *value).map(|_| i)).collect();
                        if !failing.is_empty() {
                            violations.push(Violation::new(col, "max",
                                format!("{} value(s) above max={value} in '{col}'", failing.len()),
                                Severity::Error).with_rows(failing));
                        }
                    }
                }
            }
            Constraint::Regex { pattern } => {
                match Regex::new(pattern) {
                    Err(e) => violations.push(Violation::new(col, "regex",
                        format!("invalid pattern '{pattern}': {e}"), Severity::Error)),
                    Ok(re) => {
                        if let Ok(ca) = series.str() {
                            let failing: Vec<usize> = ca.iter().enumerate()
                                .filter_map(|(i, v)| v.filter(|s| !re.is_match(s)).map(|_| i))
                                .collect();
                            if !failing.is_empty() {
                                violations.push(Violation::new(col, "regex",
                                    format!("{} value(s) don't match '{pattern}' in '{col}'", failing.len()),
                                    Severity::Error).with_rows(failing));
                            }
                        }
                    }
                }
            }
            Constraint::Len { min, max } => {
                if let Ok(ca) = series.str() {
                    let failing: Vec<usize> = ca.iter().enumerate()
                        .filter_map(|(i, v)| {
                            v.filter(|s| { let l = s.len(); l < *min || l > *max }).map(|_| i)
                        }).collect();
                    if !failing.is_empty() {
                        violations.push(Violation::new(col, "len",
                            format!("{} value(s) with length outside [{min},{max}] in '{col}'", failing.len()),
                            Severity::Error).with_rows(failing));
                    }
                }
            }
            Constraint::Enum { values } => {
                if let Ok(ca) = series.str() {
                    let allowed: std::collections::HashSet<&str> =
                        values.iter().map(|s| s.as_str()).collect();
                    let failing: Vec<usize> = ca.iter().enumerate()
                        .filter_map(|(i, v)| v.filter(|s| !allowed.contains(*s)).map(|_| i))
                        .collect();
                    if !failing.is_empty() {
                        violations.push(Violation::new(col, "enum",
                            format!("{} value(s) not in allowed set in '{col}'", failing.len()),
                            Severity::Error).with_rows(failing));
                    }
                }
            }
            Constraint::Coerce | Constraint::ForeignKey { .. } => {}
        }
    }
    violations
}
