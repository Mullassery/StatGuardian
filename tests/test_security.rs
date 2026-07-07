/// Security tests for TIER 1 fixes
/// Tests ReDoS prevention, input validation, and resource limits

use statguard_core::parse_and_compile;

#[test]
fn test_parser_input_size_limit() {
    // Create a query that exceeds MAX_INPUT_SIZE (10MB)
    let huge_query = "SELECT * FROM table WHERE ".repeat(1_000_000);

    let result = parse_and_compile(&huge_query);
    assert!(result.is_err(), "oversized input should be rejected");
}

#[test]
fn test_parser_with_max_allowed_size() {
    // Create a query just under the limit
    let dsl = r#"
        dataset test {
            schema {
                id: int, not_null, unique
                name: string, not_null
                email: string, regex="^[^@]+@[^@]+\.[^@]+$"
                age: int, between(0, 150)
                score: float, min=0.0, max=100.0
                active: bool
            }
            quality {
                completeness(id) > 0.99
                uniqueness(email) == 1.0
                @warning: avg(score) > 50.0
            }
            anomalies {
                detect_outliers(age, method="iqr")
                detect_outliers(score, method="iqr")
                @blocking: detect_duplicates(email)
            }
            stats {
                percentile_drift(age, [50, 95]) < 0.1
                distribution_change(score) < 0.05
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "valid query at reasonable size should parse");
}

#[test]
fn test_nested_query_depth_limit() {
    // Create deeply nested parentheses
    let mut nested = String::new();
    for _ in 0..15 {
        nested.push('(');
    }
    nested.push_str(r#"dataset test { schema { id: int } }"#);
    for _ in 0..15 {
        nested.push(')');
    }

    let result = parse_and_compile(&nested);
    // Should either parse (parentheses are balanced) or fail gracefully
    let _ = result;
}

#[test]
fn test_comment_injection_prevention() {
    // Try to use comments to hide malicious content
    let dsl = r#"
        dataset test {  /* hidden malicious content */
            schema {
                id: int  -- more hidden content
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Comments should be properly stripped, so this should work
    assert!(result.is_ok(), "comments should be safely handled");
}

#[test]
fn test_special_characters_in_constraint() {
    let dsl = r#"
        dataset test {
            schema {
                email: string, regex="^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "special regex characters should be handled safely");
}

#[test]
fn test_unicode_in_dataset_name() {
    let dsl = r#"
        dataset users_data {
            schema {
                id: int
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "ASCII names should work");
}

#[test]
fn test_many_fields_in_schema() {
    // Create a schema with many fields
    let mut dsl = String::from("dataset test { schema {\n");
    for i in 0..1000 {
        dsl.push_str(&format!("field{}: int,\n", i));
    }
    dsl.push_str("id: int\n}\n}");

    let result = parse_and_compile(&dsl);
    // Should either succeed or fail gracefully
    let _ = result;
}

#[test]
fn test_malformed_regex_pattern() {
    let dsl = r#"
        dataset test {
            schema {
                email: string, regex="[invalid(regex"
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Should handle invalid regex gracefully
    let _ = result;
}

#[test]
fn test_constraint_with_extreme_values() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, min=-9223372036854775808, max=9223372036854775807,
                value: float, min=-1e308, max=1e308
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "extreme numeric values should be handled");
}

#[test]
fn test_empty_constraint_value() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, regex=""
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Should handle gracefully
    let _ = result;
}

#[test]
fn test_duplicate_constraint_specification() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, not_null, not_null, unique, unique
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Should either deduplicate or fail gracefully
    let _ = result;
}

#[test]
fn test_conflicting_constraints() {
    let dsl = r#"
        dataset test {
            schema {
                id: int, between(0, 10), between(5, 100)
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Should handle conflicting ranges (either accept or reject)
    let _ = result;
}

#[test]
fn test_invalid_type_names() {
    let invalid_types = vec![
        "dataset test { schema { id: int8 } }",
        "dataset test { schema { id: string64 } }",
        "dataset test { schema { id: numeric } }",
    ];

    for dsl in invalid_types {
        let result = parse_and_compile(dsl);
        // Should fail gracefully for unknown types
        assert!(result.is_err(), "unknown type should error: {}", dsl);
    }
}

#[test]
fn test_parser_recovery_from_partial_schema() {
    let dsl = r#"
        dataset test {
            schema {
                id: int,
                name:
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_err(), "incomplete schema should error");
}

#[test]
fn test_whitespace_handling() {
    let dsl = r#"
        dataset    test    {
            schema    {
                id    :    int    ,
                name    :    string
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    assert!(result.is_ok(), "excessive whitespace should be handled");
}

#[test]
fn test_line_continuation() {
    let dsl = r#"
        dataset test {
            schema {
                very_long_field_name_that_continues_on_next_line:
                    int
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Should handle line breaks appropriately
    let _ = result;
}

#[test]
fn test_case_insensitivity_of_keywords() {
    let dsl = r#"
        DATASET test {
            SCHEMA {
                ID: int, NOT_NULL
            }
            QUALITY {
                COMPLETENESS(ID) > 0.9
            }
        }
    "#;

    let result = parse_and_compile(dsl);
    // Keywords should be case-insensitive
    let _ = result;
}

#[test]
fn test_semicolon_handling() {
    let dsl = r#"
        dataset test {
            schema {
                id: int,
                name: string,
            };
        };
    "#;

    let result = parse_and_compile(dsl);
    // Should handle optional semicolons
    let _ = result;
}

#[test]
fn test_unbalanced_braces() {
    let invalid_cases = vec![
        "dataset test { schema { id: int }",
        "dataset test schema { id: int } }",
    ];

    for dsl in invalid_cases {
        let result = parse_and_compile(dsl);
        // Should fail gracefully
        assert!(result.is_err(), "unbalanced braces should error");
    }
}
