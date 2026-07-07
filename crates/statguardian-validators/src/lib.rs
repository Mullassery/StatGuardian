pub mod rules;
pub mod schema;

pub use rules::RuleEngine;
pub use schema::SchemaValidator;

use serde::{Deserialize, Serialize};
use statguardian_core::ast::Severity;

/// A single validation finding produced by any validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub column: String,
    pub check: String,
    pub message: String,
    pub severity: Severity,
    /// Row indices that triggered the violation (empty = aggregate check).
    pub row_indices: Vec<usize>,
    /// The observed value (numeric representation for reporting).
    pub observed: Option<f64>,
    /// The expected / threshold value.
    pub expected: Option<f64>,
}

impl Violation {
    pub fn new(
        column: impl Into<String>,
        check: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            column: column.into(),
            check: check.into(),
            message: message.into(),
            severity,
            row_indices: Vec::new(),
            observed: None,
            expected: None,
        }
    }

    pub fn with_rows(mut self, rows: Vec<usize>) -> Self {
        self.row_indices = rows;
        self
    }

    pub fn with_values(mut self, observed: f64, expected: f64) -> Self {
        self.observed = Some(observed);
        self.expected = Some(expected);
        self
    }

    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Blocking
    }
}
