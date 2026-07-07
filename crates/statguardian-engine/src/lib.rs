pub mod batch;

use polars::prelude::DataFrame;
use statguardian_core::{compiler::dag::ExecutionDag, DataContract};
use statguardian_metrics::report::ValidationReport;

pub use batch::BatchExecutor;

/// High-level entry point for contract execution.
/// Wraps `BatchExecutor` and adds contract-level orchestration.
pub struct Engine {
    pub contract: DataContract,
    pub dag: ExecutionDag,
}

impl Engine {
    pub fn new(contract: DataContract, dag: ExecutionDag) -> Self {
        Self { contract, dag }
    }

    /// Execute the contract against a batch DataFrame.
    /// Optionally supply a `reference` DataFrame to enable drift detection.
    pub fn execute(&self, df: &DataFrame, reference: Option<&DataFrame>) -> ValidationReport {
        BatchExecutor::execute(&self.contract, &self.dag, df, reference)
    }

    /// Execute against a file path (auto-detects format).
    pub fn execute_file(
        &self,
        path: &str,
        reference_path: Option<&str>,
    ) -> Result<ValidationReport, statguard_io::IoError> {
        let df = statguard_io::DataReader::read_file(path)?;
        let reference = reference_path
            .map(statguard_io::DataReader::read_file)
            .transpose()?;
        Ok(self.execute(&df, reference.as_ref()))
    }

    /// Streaming execution: process file in batches of `batch_size` rows.
    /// Each batch is validated independently; results are merged.
    pub fn execute_streaming(
        &self,
        path: &str,
        batch_size: usize,
    ) -> Result<Vec<ValidationReport>, statguard_io::IoError> {
        let mut batcher = statguard_io::StreamingBatcher::new(path, batch_size);
        let mut reports = Vec::new();

        while let Some(batch) = batcher.next_batch()? {
            let report = self.execute(&batch, None);
            reports.push(report);
        }

        Ok(reports)
    }
}

/// Convenience function: parse DSL + compile + execute in one call.
pub fn run(dsl: &str, df: &DataFrame) -> Result<ValidationReport, statguard_core::CoreError> {
    let pairs = statguard_core::parse_and_compile(dsl)?;
    // Use first contract
    let (contract, dag) = pairs.into_iter().next().ok_or_else(|| {
        statguard_core::CoreError::Compile {
            message: "no datasets defined in DSL".into(),
        }
    })?;
    let engine = Engine::new(contract, dag);
    Ok(engine.execute(df, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::*;
    use statguardian_core::parse_and_compile;

    fn make_users_df() -> DataFrame {
        df!(
            "id"      => &[1i64, 2, 3, 4, 5],
            "email"   => &["a@b.com", "c@d.com", "e@f.com", "g@h.com", "bad_email"],
            "age"     => &[25i64, 30, 45, 22, 200],
            "country" => &[Some("US"), Some("UK"), Some("DE"), None, Some("FR")]
        )
        .unwrap()
    }

    const DSL: &str = r#"
dataset users {
    schema {
        id: int, not_null, unique
        email: string, regex="^[^@]+@[^@]+\.[^@]+$"
        age: int, between(0, 120)
        country: string, not_null
    }
    quality {
        completeness(id) > 0.99
        uniqueness(email) == 1.0
    }
    anomalies {
        detect_outliers(age, method="iqr")
    }
}
"#;

    #[test]
    fn test_engine_detects_violations() {
        let df = make_users_df();
        let pairs = parse_and_compile(DSL).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        let engine = Engine::new(contract, dag);
        let report = engine.execute(&df, None);

        // age=200 violates between(0,120)
        // country has 1 null
        // email "bad_email" fails regex
        assert!(!report.violations.is_empty());
        assert!(!report.passed || report.health.score < 1.0);
    }

    #[test]
    fn test_engine_passes_clean_data() {
        let df = df!(
            "id"      => &[1i64, 2, 3],
            "email"   => &["a@b.com", "c@d.com", "e@f.com"],
            "age"     => &[25i64, 30, 45],
            "country" => &["US", "UK", "DE"]
        )
        .unwrap();

        let pairs = parse_and_compile(DSL).unwrap();
        let (contract, dag) = pairs.into_iter().next().unwrap();
        let engine = Engine::new(contract, dag);
        let report = engine.execute(&df, None);

        // All schema checks should pass (maybe quality violations remain)
        let blocking: Vec<_> = report.violations.iter()
            .filter(|v| v.severity == statguard_core::ast::Severity::Blocking)
            .collect();
        assert!(blocking.is_empty(), "blocking violations on clean data: {blocking:?}");
    }

    #[test]
    fn test_convenience_run() {
        let df = make_users_df();
        let report = run(DSL, &df).unwrap();
        assert!(!report.id.is_empty());
        assert!(report.row_count == 5);
    }
}
