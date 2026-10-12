//! End-to-end tests for StreamGuard: incremental NDJSON validation through the
//! public `StreamValidator` API, exercising malformed handling, resource
//! limits, sanitization reporting, bounded batches, and finding-index stability.

use statguardian_core::parse_and_compile;
use statguardian_stream::{
    MalformedKind, MalformedPolicy, RedactionPolicy, RedactionRule, RedactionStrategy, Sanitizer,
    StreamError, StreamLimits, StreamOptions, StreamValidator,
};

fn validator(dsl: &str, options: StreamOptions) -> StreamValidator {
    let (contract, dag) = parse_and_compile(dsl)
        .expect("DSL should parse")
        .into_iter()
        .next()
        .expect("one contract");
    StreamValidator::new(contract, dag, options)
}

const SIMPLE_DSL: &str = r#"
dataset events {
    schema {
        id:    int, not_null
        value: int
    }
}
"#;

#[test]
fn valid_stream_passes_with_zero_exit_code() {
    let v = validator(SIMPLE_DSL, StreamOptions::default());
    let report = v
        .validate_bytes(b"{\"id\":1,\"value\":10}\n{\"id\":2,\"value\":20}\n")
        .unwrap();

    assert!(report.passed);
    assert_eq!(report.exit_code(false), 0);
    assert_eq!(report.records_validated, 2);
    assert_eq!(report.records_read, 2);
    assert_eq!(report.malformed_count, 0);
    assert!(report.findings.is_empty());
    assert_eq!(report.batches, 1);
}

#[test]
fn missing_column_is_a_blocking_failure() {
    let v = validator(SIMPLE_DSL, StreamOptions::default());
    let report = v
        .validate_bytes(b"{\"id\":1}\n{\"id\":2}\n")
        .unwrap();

    assert!(!report.passed);
    assert_eq!(report.exit_code(false), 1);
    assert!(report.blocking_count() >= 1);
    let missing = report
        .findings
        .iter()
        .find(|f| f.check == "column_exists")
        .expect("expected a column_exists finding for the absent 'value' column");
    assert!(missing.record.is_none(), "aggregate finding has no record");
}

#[test]
fn row_level_findings_carry_correct_global_indices_across_skipped_malformed() {
    // `between` produces per-row findings. The malformed line sits *before*
    // the failing rows, so record numbers are non-contiguous within the batch:
    // a naive `batch_base + local_index` mapping would report the wrong rows.
    let dsl = r#"
        dataset events {
            schema {
                id: int, between(0, 1)
            }
        }
    "#;
    let mut options = StreamOptions {
        malformed: MalformedPolicy::Skip,
        ..StreamOptions::default()
    };
    options.limits.batch_size = 1_000;

    let input = b"{\"id\":1}\nNOT JSON\n{\"id\":5}\n{\"id\":9}\n";
    let report = validator(dsl, options).validate_bytes(input).unwrap();

    assert_eq!(report.records_read, 4);
    assert_eq!(report.records_validated, 3);
    assert_eq!(report.malformed_count, 1);

    let mut records: Vec<u64> = report
        .findings
        .iter()
        .filter_map(|f| f.record)
        .collect();
    records.sort_unstable();
    records.dedup();
    assert_eq!(
        records,
        vec![2, 3],
        "failing global record indices must be 2 and 3, not the malformed line's index"
    );
}

#[test]
fn findings_are_stable_and_sorted_across_batches() {
    let dsl = r#"
        dataset events {
            schema {
                id: int, between(0, 0)
            }
        }
    "#;
    let mut options = StreamOptions::default();
    options.limits.batch_size = 2;

    let input = b"{\"id\":1}\n{\"id\":2}\n{\"id\":3}\n{\"id\":4}\n{\"id\":5}\n";
    let report = validator(dsl, options).validate_bytes(input).unwrap();

    assert_eq!(report.records_validated, 5);
    assert!(report.batches >= 3, "expected multiple batches");

    // The raw findings list must already be in deterministic
    // (record, column, check, batch) order.
    assert!(
        report.findings.windows(2).all(|w| {
            let key = |f: &statguardian_stream::StreamFinding| {
                (f.record, f.column.clone(), f.check.clone(), f.batch)
            };
            key(&w[0]) <= key(&w[1])
        }),
        "findings must be emitted in stable sorted order"
    );

    // Every record must be validated, including the final single-record
    // leftover batch. Each record trips both the `between` schema constraint
    // and the `range_check` DAG node.
    let mut records: Vec<u64> = report.findings.iter().filter_map(|f| f.record).collect();
    records.sort_unstable();
    records.dedup();
    assert_eq!(
        records,
        vec![0, 1, 2, 3, 4],
        "the final 1-record micro-batch must not be silently skipped"
    );
}

#[test]
fn malformed_json_skip_counts_and_continues() {
    let mut options = StreamOptions::default();
    options.malformed = MalformedPolicy::Skip;
    let report = validator(SIMPLE_DSL, options)
        .validate_bytes(b"{\"id\":1,\"value\":1}\n{oops\n{\"id\":2,\"value\":2}\n")
        .unwrap();

    assert_eq!(report.malformed_count, 1);
    assert_eq!(report.records_validated, 2);
    assert!(report.malformed.is_empty(), "Skip collects no diagnostics");
}

#[test]
fn malformed_json_collect_records_diagnostics_without_raw_by_default() {
    let mut options = StreamOptions::default();
    options.malformed = MalformedPolicy::Collect { max: 10 };
    let report = validator(SIMPLE_DSL, options)
        .validate_bytes(b"{oops\n{\"id\":2,\"value\":2}\n")
        .unwrap();

    assert_eq!(report.malformed.len(), 1);
    let m = &report.malformed[0];
    assert_eq!(m.record, 0);
    assert_eq!(m.kind, MalformedKind::Json);
    assert!(m.preview.is_none(), "raw preview must be off by default");
}

#[test]
fn allow_raw_diagnostics_opts_into_preview() {
    let mut options = StreamOptions::default();
    options.malformed = MalformedPolicy::Collect { max: 10 };
    options.allow_raw_diagnostics = true;
    let report = validator(SIMPLE_DSL, options)
        .validate_bytes(b"{oops\n")
        .unwrap();

    assert_eq!(report.malformed.len(), 1);
    assert!(report.malformed[0].preview.as_deref().unwrap().contains("oops"));
}

#[test]
fn fail_fast_returns_error_and_maps_to_execution_failure() {
    let mut options = StreamOptions::default();
    options.malformed = MalformedPolicy::FailFast;
    let err = validator(SIMPLE_DSL, options)
        .validate_bytes(b"{oops\n")
        .unwrap_err();
    assert!(matches!(err, StreamError::Malformed { record: 0, .. }));
}

#[test]
fn oversized_record_is_flagged_and_skipped() {
    let mut options = StreamOptions::default();
    options.limits.max_record_bytes = 32;
    options.oversized = MalformedPolicy::Collect { max: 10 };

    let big = format!("{{\"id\":1,\"value\":\"{}\"}}", "x".repeat(500));
    let input = format!("{big}\n{{\"id\":2,\"value\":2}}\n");
    let report = validator(SIMPLE_DSL, options)
        .validate_bytes(input.as_bytes())
        .unwrap();

    assert_eq!(report.oversized_count, 1);
    assert_eq!(report.records_validated, 1);
    let m = report
        .malformed
        .iter()
        .find(|m| m.kind == MalformedKind::Oversized)
        .expect("oversized diagnostic");
    assert_eq!(m.record, 0);
}

#[test]
fn max_records_truncates_and_marks_limit_hit() {
    let mut options = StreamOptions::default();
    options.limits = StreamLimits::with_max_records(2);

    let input = b"{\"id\":1,\"value\":1}\n{\"id\":2,\"value\":2}\n{\"id\":3,\"value\":3}\n";
    let report = validator(SIMPLE_DSL, options).validate_bytes(input).unwrap();

    assert!(report.truncated);
    assert_eq!(report.records_validated, 2);
    let hit = report.limit_hit.expect("limit hit recorded");
    assert_eq!(hit.limit, "max_records");
    assert_eq!(hit.value, 2);
}

#[test]
fn sanitization_is_applied_and_summarized() {
    let sanitizer = Sanitizer::new(
        RedactionPolicy {
            rules: vec![RedactionRule {
                field: "secret".into(),
                strategy: RedactionStrategy::Mask,
            }],
            recursive: false,
        },
        Default::default(),
    );
    let options = StreamOptions {
        sanitizer,
        ..StreamOptions::default()
    };

    let report = validator(SIMPLE_DSL, options)
        .validate_bytes(b"{\"id\":1,\"value\":1,\"secret\":\"hunter2\"}\n")
        .unwrap();

    assert_eq!(report.sanitization.total_events, 1);
    assert_eq!(report.sanitization.by_kind["masked"], 1);
    assert_eq!(report.sanitization.by_field["secret"], 1);
    // Raw value never appears in the serialized report.
    assert!(!report.to_json().contains("hunter2"));
}

#[test]
fn unicode_records_are_preserved_and_validated() {
    let dsl = r#"
        dataset events {
            schema {
                name: string, not_null
            }
        }
    "#;
    let v = validator(dsl, StreamOptions::default());
    let report = v
        .validate_bytes("{\"name\":\"héllo 世界 🚀\"}\n".as_bytes())
        .unwrap();
    assert!(report.passed);
    assert_eq!(report.records_validated, 1);
}

#[test]
fn report_json_and_prometheus_are_well_formed() {
    let v = validator(SIMPLE_DSL, StreamOptions::default());
    let report = v
        .validate_bytes(b"{\"id\":1,\"value\":1}\n")
        .unwrap();

    let json = report.to_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert_eq!(parsed["dataset"], "events");

    let prom = report.to_prometheus();
    assert!(prom.contains("statguard_stream_passed{dataset=\"events\"} 1"));
    assert!(prom.contains("statguard_stream_records_validated"));

    assert!(report.summary().starts_with("[StreamGuard] PASS"));
}

