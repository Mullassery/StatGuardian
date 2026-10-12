/// Resource limits applied to a stream. All limits are optional except the
/// per-record byte cap and batch size, which have safe defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamLimits {
    /// Stop after this many records have been *read* (successfully parsed or
    /// not). `None` means unbounded.
    pub max_records: Option<u64>,
    /// Stop after this many source bytes have been consumed. `None` means
    /// unbounded.
    pub max_bytes: Option<u64>,
    /// Maximum accepted size of a single line, in bytes, before the newline.
    /// Oversized lines are handled by the configured oversized policy and the
    /// remainder of the line is discarded without allocating it.
    pub max_record_bytes: usize,
    /// Number of records accumulated into one validation micro-batch.
    pub batch_size: usize,
}

impl Default for StreamLimits {
    fn default() -> Self {
        Self {
            max_records: None,
            max_bytes: None,
            max_record_bytes: 8 * 1024 * 1024,
            batch_size: 1_000,
        }
    }
}

impl StreamLimits {
    /// Convenience constructor used by tests and callers that only care about
    /// the record cap.
    pub fn with_max_records(max_records: u64) -> Self {
        Self {
            max_records: Some(max_records),
            ..Default::default()
        }
    }
}

/// What to do when a record cannot be parsed, is not UTF-8, or exceeds the
/// per-record byte cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedPolicy {
    /// Abort the whole run with [`crate::StreamError`]; the CLI maps this to
    /// exit code 2.
    FailFast,
    /// Silently continue, but keep a count of skipped records in the report.
    Skip,
    /// Collect up to `max` malformed records (with diagnostics) and keep
    /// reading; further malformed records are only counted.
    Collect { max: usize },
}

impl Default for MalformedPolicy {
    fn default() -> Self {
        MalformedPolicy::Collect { max: 100 }
    }
}
