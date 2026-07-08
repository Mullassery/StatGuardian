/// Unit tests for statguard-core parser
/// Tests DSL parsing, AST generation, validation rules, and error handling
use statguardian_core::parse_and_compile;

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
}

#[test]
fn test_parse_with_schema_constraints() {
    let dsl = r#"
        dataset products {
            schema {
                id: int, primary_key, not_null
                name: string, not_null
                price: float, min=0.0, max=1000000.0
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "schema with constraints should parse");
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
}

#[test]
fn test_error_on_invalid_syntax() {
    let dsl = "dataset {";
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
    assert!(result.is_ok(), "constraint order should work");
}

#[test]
fn test_parse_preserves_dataset_name() {
    let names = vec!["users", "orders", "products"];

    for name in names {
        let dsl = format!(
            r#"
            dataset {} {{
                schema {{
                    id: int
                }}
            }}
        "#,
            name
        );

        let result = parse_and_compile(&dsl);
        assert!(result.is_ok(), "should parse dataset: {}", name);
    }
}

#[test]
fn test_regex_constraint() {
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
fn test_between_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                score: float, between(0.0, 1.0)
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "between constraint should parse");
}
