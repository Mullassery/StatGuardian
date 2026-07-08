/// Unit tests for statguard-validators
/// Tests schema validation with Polars DataFrames
use polars::prelude::*;
use statguardian_core::parse_and_compile;
use statguardian_engine::Engine;

#[test]
fn test_parser_and_engine_basic() {
    let dsl = r#"
        dataset test {
            schema {
                id: int,
                value: float
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "should parse basic schema");

    let pairs = result.unwrap();
    let (contract, dag) = pairs.into_iter().next().unwrap();
    let engine = Engine::new(contract, dag);

    let data = df!(
        "id" => &[1i64, 2, 3],
        "value" => &[1.0f64, 2.0, 3.0]
    )
    .unwrap();

    let report = engine.execute(&data, None);
    assert_eq!(report.row_count, 3);
}

#[test]
fn test_validation_on_valid_data() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, not_null
                status: string
            }
        }
    "#;

    let pairs = parse_and_compile(dsl).unwrap();
    let (contract, dag) = pairs.into_iter().next().unwrap();
    let engine = Engine::new(contract, dag);

    let data = df!(
        "id" => &[1i64, 2, 3],
        "status" => &["active", "inactive", "pending"]
    )
    .unwrap();

    let report = engine.execute(&data, None);
    assert!(report.health.score >= 0.0);
    assert!(report.health.score <= 1.0);
}

#[test]
fn test_constraints_recognized() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, primary_key, unique, not_null
                score: float, min=0.0, max=100.0
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "should recognize all constraint types");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    // Verify schema was parsed
    assert_eq!(contract.schema.len(), 2);
    assert_eq!(contract.schema[0].name, "id");
    assert_eq!(contract.schema[1].name, "score");
}

#[test]
fn test_large_dataset_processing() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
                value: float
            }
        }
    "#;

    let pairs = parse_and_compile(dsl).unwrap();
    let (contract, dag) = pairs.into_iter().next().unwrap();
    let engine = Engine::new(contract, dag);

    // Create reasonably large dataset
    let n = 10_000;
    let ids: Vec<i64> = (0..n as i64).collect();
    let values: Vec<f64> = (0..n).map(|i| (i as f64) / 100.0).collect();

    let data = df!(
        "id" => ids,
        "value" => values
    )
    .unwrap();

    let start = std::time::Instant::now();
    let report = engine.execute(&data, None);
    let elapsed = start.elapsed();

    assert_eq!(report.row_count, n as usize);
    assert!(
        elapsed.as_secs() < 30,
        "processing should be reasonably fast"
    );
}

#[test]
fn test_empty_dataframe_handling() {
    let dsl = r#"
        dataset test {
            schema {
                id: int
            }
        }
    "#;

    let pairs = parse_and_compile(dsl).unwrap();
    let (contract, dag) = pairs.into_iter().next().unwrap();
    let engine = Engine::new(contract, dag);

    let empty = df!(
        "id" => Vec::<Option<i64>>::new()
    )
    .unwrap();

    let report = engine.execute(&empty, None);
    assert_eq!(report.row_count, 0);
}

#[test]
fn test_multiple_schema_fields() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, primary_key
                name: string, not_null
                email: string, regex="^[^@]+@[^@]+\\.[^@]+$"
                age: int, min=0, max=150
                active: bool
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok());

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();
    assert_eq!(contract.schema.len(), 5);
}
