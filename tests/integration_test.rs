/// Integration tests — exercise the full pipeline from DSL string to report.
/// These tests run against the statguard-engine crate.
use polars::prelude::*;
use statguardian_core::parse_and_compile;
use statguardian_engine::Engine;

fn make_clean_df() -> DataFrame {
    df!(
        "id"      => &[1i64, 2, 3, 4, 5],
        "email"   => &["a@b.com", "c@d.com", "e@f.com", "g@h.com", "i@j.com"],
        "age"     => &[25i64, 30, 45, 22, 55],
        "country" => &["US", "UK", "DE", "FR", "CA"],
        "score"   => &[0.9f64, 0.8, 0.7, 0.6, 0.5]
    )
    .unwrap()
}

fn make_dirty_df() -> DataFrame {
    df!(
        "id"      => &[Some(1i64), Some(2), Some(2), None, Some(5)],   // null + dupe
        "email"   => &["a@b.com", "not_an_email", "c@d.com", "e@f.com", "g@h.com"],
        "age"     => &[25i64, 300, 45, 22, -5],                         // out of range
        "country" => &[Some("US"), Some("UK"), None, Some("FR"), Some("CA")],
        "score"   => &[0.9f64, 1.5, 0.7, -0.1, 0.5]                    // out of range
    )
    .unwrap()
}

const FULL_DSL: &str = r#"
dataset orders {
    schema {
        id:      int,    not_null, unique, primary_key
        email:   string, regex="^[^@]+@[^@]+\.[^@]+$"
        age:     int,    between(0, 120)
        country: string, not_null
        score:   float,  min=0.0, max=1.0
    }
    quality {
        completeness(id)    > 0.99
        @warning: uniqueness(email) == 1.0
    }
    anomalies {
        detect_outliers(age, method="iqr")
        @blocking: detect_duplicates(id)
    }
}
"#;

fn engine_from_dsl(dsl: &str) -> Engine {
    let pairs = parse_and_compile(dsl).expect("DSL should parse");
    let (contract, dag) = pairs.into_iter().next().unwrap();
    Engine::new(contract, dag)
}

#[test]
fn test_clean_data_passes_with_high_score() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_clean_df(), None);

    assert!(report.passed, "clean data should pass");
    assert!(
        report.health.score > 0.9,
        "score should be high: {}",
        report.health.score
    );
    assert_eq!(report.row_count, 5);
    assert!(!report.column_profiles.is_empty());
}

#[test]
fn test_dirty_data_produces_violations() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_dirty_df(), None);

    assert!(
        !report.violations.is_empty(),
        "dirty data must produce violations"
    );
    // duplicate id with @blocking severity should mark as failed
    assert!(!report.passed, "duplicate primary key should fail");
}

#[test]
fn test_violation_categories() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_dirty_df(), None);

    let checks: Vec<&str> = report.violations.iter().map(|v| v.check.as_str()).collect();
    // We expect at least one of these categories to appear:
    let has_null = checks.iter().any(|c| c.contains("null"));
    let has_range = checks.iter().any(|c| {
        c.contains("between") || c.contains("range") || c.contains("min") || c.contains("max")
    });
    let has_unique = checks
        .iter()
        .any(|c| c.contains("unique") || c.contains("duplicate"));
    let has_regex = checks.iter().any(|c| c.contains("regex"));

    assert!(
        has_null || has_range || has_unique || has_regex,
        "expected typed violations, got: {checks:?}"
    );
}

#[test]
fn test_drift_detection() {
    const DRIFT_DSL: &str = r#"
dataset metrics {
    schema {
        value: float, positive
    }
    stats {
        value.mean drift < 0.5
    }
}
"#;

    let engine = engine_from_dsl(DRIFT_DSL);

    let reference = df!("value" => &[1.0f64, 2.0, 3.0, 4.0, 5.0]).unwrap();
    let current = df!("value" => &[1.0f64, 2.0, 3.0, 4.0, 5.0]).unwrap(); // identical

    let report = engine.execute(&current, Some(&reference));
    assert!(
        report.drift_results.iter().all(|r| r.passed),
        "identical distributions should not drift"
    );
}

#[test]
fn test_drift_detection_catches_large_shift() {
    const DRIFT_DSL: &str = r#"
dataset metrics {
    schema { value: float }
    stats { value.mean drift < 0.1 }
}
"#;

    let engine = engine_from_dsl(DRIFT_DSL);
    let reference = df!("value" => &[1.0f64, 2.0, 3.0]).unwrap();
    let current = df!("value" => &[100.0f64, 200.0, 300.0]).unwrap(); // massive shift

    let report = engine.execute(&current, Some(&reference));
    assert!(
        report.drift_results.iter().any(|r| !r.passed),
        "large distribution shift should be detected as drift"
    );
}

#[test]
fn test_report_prometheus_output() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_clean_df(), None);
    let prom = report.to_prometheus();

    assert!(prom.contains("statguard_health_score"));
    assert!(prom.contains("statguard_violations_total"));
    assert!(prom.contains("statguard_row_count"));
}

#[test]
fn test_report_json_is_valid() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_clean_df(), None);
    let json: serde_json::Value =
        serde_json::from_str(&report.to_json()).expect("report JSON should be valid");

    assert!(json["id"].is_string());
    assert!(json["passed"].is_boolean());
    assert!(json["health"]["score"].is_number());
}

#[test]
fn test_column_profiling() {
    let engine = engine_from_dsl(FULL_DSL);
    let report = engine.execute(&make_clean_df(), None);

    let age_profile = report.column_profiles.iter().find(|p| p.name == "age");
    assert!(age_profile.is_some(), "age column should be profiled");
    let age = age_profile.unwrap();
    assert!(age.mean.is_some());
    assert!(age.min.is_some());
    assert!(age.max.is_some());
    assert_eq!(age.null_count, 0);
}

#[test]
fn test_health_score_degrades_with_violations() {
    let engine = engine_from_dsl(FULL_DSL);
    let clean_report = engine.execute(&make_clean_df(), None);
    let dirty_report = engine.execute(&make_dirty_df(), None);

    assert!(
        clean_report.health.score >= dirty_report.health.score,
        "clean data score ({}) should be >= dirty data score ({})",
        clean_report.health.score,
        dirty_report.health.score
    );
}

#[test]
fn test_streaming_execution() {
    let engine = engine_from_dsl(FULL_DSL);
    // Streaming is file-based; test via the file API if a temp file is available.
    // Here we just verify the batch executor produces consistent results
    // by splitting the clean df in half and executing each part.

    let df = make_clean_df();
    let batch1 = df.slice(0, 3);
    let batch2 = df.slice(3, 2);

    let r1 = engine.execute(&batch1, None);
    let r2 = engine.execute(&batch2, None);

    assert_eq!(r1.row_count + r2.row_count, 5);
    // Both clean batches should pass
    let has_blocking1 = r1.violations.iter().any(|v| v.is_blocking());
    let has_blocking2 = r2.violations.iter().any(|v| v.is_blocking());
    assert!(!has_blocking1 && !has_blocking2);
}

#[test]
fn test_multiple_contracts_in_dsl() {
    let dsl = r#"
dataset a { schema { x: int } }
dataset b { schema { y: string } }
"#;
    let pairs = parse_and_compile(dsl).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].0.name, "a");
    assert_eq!(pairs[1].0.name, "b");
}
