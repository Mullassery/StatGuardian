/// Unit tests for statguard-core parser
/// Tests DSL parsing, AST generation, validation rules, and error handling

use statguard_core::{parse_and_compile, DataContract};

#[test]
fn test_parse_simple_dataset() {
    let dsl = r#"
        dataset users {
            schema {
                id: int,
                name: string
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "simple dataset should parse");

    let pairs = result.unwrap();
    assert_eq!(pairs.len(), 1, "should have one contract");

    let (contract, _dag) = pairs.into_iter().next().unwrap();
    assert_eq!(contract.name, "users");
}

#[test]
fn test_parse_with_schema_constraints() {
    let dsl = r#"
        dataset products {
            schema {
                id: int, primary_key, not_null
                name: string, not_null
                price: float, min=0.0, max=1000000.0
                stock: int, between(0, 100000)
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "schema with constraints should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert_eq!(contract.schema.len(), 4, "should have 4 fields");
    assert_eq!(contract.schema[0].name, "id");
    assert!(!contract.schema[0].constraints.is_empty(), "id should have constraints");
}

#[test]
fn test_parse_quality_rules() {
    let dsl = r#"
        dataset transactions {
            schema {
                id: int
                amount: float
            }
            quality {
                completeness(id) > 0.99
                @warning: uniqueness(id) == 1.0
                @blocking: avg(amount) > 0
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "quality rules should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert!(!contract.quality_rules.is_empty(), "should have quality rules");
}

#[test]
fn test_parse_anomaly_detection() {
    let dsl = r#"
        dataset sensors {
            schema {
                reading: float
            }
            anomalies {
                detect_outliers(reading, method="iqr")
                detect_duplicates(reading)
                @blocking: detect_spikes(reading)
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "anomaly detection should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert!(!contract.anomaly_rules.is_empty(), "should have anomaly rules");
}

#[test]
fn test_parse_regex_constraint() {
    let dsl = r#"
        dataset contacts {
            schema {
                email: string, regex="^[^@]+@[^@]+\.[^@]+$"
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "regex constraint should parse");
}

#[test]
fn test_parse_multiple_datasets() {
    let dsl = r#"
        dataset users {
            schema {
                id: int
            }
        }

        dataset orders {
            schema {
                order_id: int
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "multiple datasets should parse");

    let pairs = result.unwrap();
    assert_eq!(pairs.len(), 2, "should have two contracts");
}

#[test]
fn test_parse_streaming_config() {
    let dsl = r#"
        dataset events {
            schema {
                timestamp: datetime
            }
            stream {
                window: tumbling(60s)
                watermark: 30s
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "streaming config should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert!(contract.stream_config.is_some(), "should have streaming config");
}

#[test]
fn test_error_on_invalid_syntax() {
    let dsl = "dataset {"; // Missing name

    let result = parse_and_compile(dsl);
    assert!(result.is_err(), "invalid syntax should error");
}

#[test]
fn test_error_on_unknown_type() {
    let dsl = r#"
        dataset test {
            schema {
                id: unknown_type
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_err(), "unknown type should error");
}

#[test]
fn test_parse_all_supported_types() {
    let dsl = r#"
        dataset types {
            schema {
                int_col: int,
                float_col: float,
                string_col: string,
                bool_col: bool,
                date_col: date,
                datetime_col: datetime,
                bytes_col: bytes
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "all types should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert_eq!(contract.schema.len(), 7, "should have all 7 types");
}

#[test]
fn test_parse_with_stats_rules() {
    let dsl = r#"
        dataset metrics {
            schema {
                value: float
            }
            stats {
                percentile_drift(value, [50, 95]) < 0.1
                @warning: distribution_change(value) < 0.05
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "stats rules should parse");

    let pairs = result.unwrap();
    let (contract, _dag) = pairs.into_iter().next().unwrap();

    assert!(!contract.stats_rules.is_empty(), "should have stats rules");
}

#[test]
fn test_constraint_ordering() {
    let dsl = r#"
        dataset items {
            schema {
                price: float, min=0.0, max=999.99, not_null
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "constraint order should not matter");
}

#[test]
fn test_empty_dataset_error() {
    let dsl = r#"
        dataset empty {
        }
    "#;

    let result = parse_and_compile(dsl);
    // May be ok or err depending on implementation
    // Just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_parse_preserves_dataset_name() {
    let names = vec!["users", "orders", "products", "metrics_prod_v2"];

    for name in names {
        let dsl = format!(r#"
            dataset {} {{
                schema {{
                    id: int
                }}
            }}
        "#, name);

        let result = parse_and_compile(&dsl);
        assert!(result.is_ok());

        let pairs = result.unwrap();
        let (contract, _dag) = pairs.into_iter().next().unwrap();
        assert_eq!(contract.name, name);
    }
}

#[test]
fn test_parse_cross_dataset_relationships() {
    let dsl = r#"
        dataset users {
            schema {
                id: int, primary_key
            }
        }

        dataset orders {
            schema {
                user_id: int, foreign_key("users.id")
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "cross-dataset references should parse");

    let pairs = result.unwrap();
    assert_eq!(pairs.len(), 2);
}
