/// Unit tests for statguard-validators
/// Tests schema validation, quality rules, and anomaly detection

use polars::prelude::*;
use statguard_core::parse_and_compile;
use statguard_engine::Engine;

fn make_df(data: Vec<(&str, Vec<Option<i32>>)>) -> DataFrame {
    let cols: Vec<_> = data
        .into_iter()
        .map(|(name, vals)| Series::new(name, vals.iter().map(|&v| v).collect::<Vec<_>>()))
        .collect();
    DataFrame::new(cols).unwrap()
}

#[test]
fn test_not_null_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, not_null
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Valid data (no nulls)
    let valid = df!(
        "id" => &[1i64, 2, 3]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.8, "non-null data should pass");

    // Invalid data (has nulls)
    let invalid = df!(
        "id" => &[Some(1i64), None, Some(3)]
    ).unwrap();
    let report = engine.execute(&invalid, None);
    assert!(!report.violations.is_empty(), "null data should produce violations");
}

#[test]
fn test_unique_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                email: string, unique
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Unique values
    let valid = df!(
        "email" => &["a@b.com", "c@d.com", "e@f.com"]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.8, "unique data should pass");

    // Duplicate values
    let invalid = df!(
        "email" => &["a@b.com", "a@b.com", "e@f.com"]
    ).unwrap();
    let report = engine.execute(&invalid, None);
    assert!(!report.violations.is_empty(), "duplicates should produce violations");
}

#[test]
fn test_min_max_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                age: int, min=0, max=150
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Valid range
    let valid = df!(
        "age" => &[18i64, 25, 65, 42]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.8, "in-range data should pass");

    // Out of range
    let invalid = df!(
        "age" => &[18i64, -5, 200, 42]
    ).unwrap();
    let report = engine.execute(&invalid, None);
    assert!(!report.violations.is_empty(), "out-of-range data should produce violations");
}

#[test]
fn test_between_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                score: float, between(0.0, 1.0)
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Valid range
    let valid = df!(
        "score" => &[0.1f64, 0.5, 0.9, 0.0]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.8);

    // Out of range
    let invalid = df!(
        "score" => &[-0.1f64, 0.5, 1.5, 0.0]
    ).unwrap();
    let report = engine.execute(&invalid, None);
    assert!(!report.violations.is_empty());
}

#[test]
fn test_regex_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                email: string, regex="^[^@]+@[^@]+\.[^@]+$"
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Valid emails
    let valid = df!(
        "email" => &["user@example.com", "admin@company.co.uk"]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.8);

    // Invalid emails
    let invalid = df!(
        "email" => &["notanemail", "user@domain", "@example.com"]
    ).unwrap();
    let report = engine.execute(&invalid, None);
    assert!(!report.violations.is_empty());
}

#[test]
fn test_completeness_rule() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
            }
            quality {
                completeness(id) > 0.8
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // 80% complete (threshold is > 0.8, so this should fail)
    let borderline = df!(
        "id" => &[Some(1i64), Some(2), None, None, Some(5)]
    ).unwrap();
    let report = engine.execute(&borderline, None);
    // Completeness is 3/5 = 0.6, which is < 0.8, so should fail
    assert!(!report.violations.is_empty());

    // 100% complete
    let perfect = df!(
        "id" => &[1i64, 2, 3, 4, 5]
    ).unwrap();
    let report = engine.execute(&perfect, None);
    assert!(report.health.score > 0.9);
}

#[test]
fn test_multiple_constraints_on_field() {
    let dsl = r#"
        dataset test {
            schema {
                product_id: int, primary_key, not_null, unique
                price: float, min=0.0, max=1000000.0, not_null
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Valid data
    let valid = df!(
        "product_id" => &[1i64, 2, 3],
        "price" => &[9.99f64, 19.99, 99.99]
    ).unwrap();
    let report = engine.execute(&valid, None);
    assert!(report.health.score > 0.9);
}

#[test]
fn test_violation_severity_levels() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
            }
            quality {
                @warning: completeness(id) > 0.8
                @blocking: uniqueness(id) == 1.0
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    let data = df!(
        "id" => &[Some(1i64), Some(1), None]
    ).unwrap();
    let report = engine.execute(&data, None);

    // Should have violations with different severities
    assert!(!report.violations.is_empty());
    // Check that at least one violation exists
    let has_warning = report.violations.iter().any(|v| v.severity == "warning");
    let has_blocking = report.violations.iter().any(|v| v.severity == "blocking");
    assert!(has_warning || has_blocking, "should have severity levels");
}

#[test]
fn test_empty_dataframe() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    let empty = df!(
        "id" => Vec::<Option<i64>>::new()
    ).unwrap();
    let report = engine.execute(&empty, None);

    assert_eq!(report.row_count, 0);
}

#[test]
fn test_large_dataframe_performance() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, unique
                value: float, between(0.0, 1.0)
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    // Create large dataframe
    let n = 100_000;
    let ids: Vec<i64> = (1..=n as i64).collect();
    let values: Vec<f64> = (0..n).map(|i| (i as f64 % 1000.0) / 1000.0).collect();

    let large = df!(
        "id" => ids,
        "value" => values
    ).unwrap();

    let start = std::time::Instant::now();
    let report = engine.execute(&large, None);
    let elapsed = start.elapsed();

    assert_eq!(report.row_count, n as usize);
    assert!(elapsed.as_secs() < 10, "should complete in under 10 seconds");
}

#[test]
fn test_all_null_column() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
            }
            quality {
                completeness(id) > 0.5
            }
        }
    "#;

    let engine = {
        let pairs = parse_and_compile(dsl).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        Engine::new(contract, dag)
    };

    let all_null = df!(
        "id" => &[Option::<i64>::None; 5]
    ).unwrap();
    let report = engine.execute(&all_null, None);

    // Completeness = 0%, which is < 50%, should fail
    assert!(!report.violations.is_empty());
}
