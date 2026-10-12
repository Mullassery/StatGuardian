use thiserror::Error;

/// Errors produced by StreamGuard / AgentGuard.
#[derive(Debug, Error)]
pub enum StreamError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("record {record} is malformed: {reason}")]
    Malformed { record: u64, reason: String },

    #[error("record {record} exceeds max_record_bytes ({limit} bytes); line was at least {actual} bytes")]
    OversizedRecord {
        record: u64,
        limit: usize,
        actual: u64,
    },

    #[error("stream exceeded max_records limit ({limit})")]
    RecordLimitExceeded { limit: u64 },

    #[error("stream exceeded max_bytes limit ({limit} bytes)")]
    ByteLimitExceeded { limit: u64 },

    #[error("record {record} is not valid UTF-8")]
    InvalidUtf8 { record: u64 },

    #[error("contract error: {0}")]
    Core(#[from] statguardian_core::CoreError),

    #[error("failed to materialize batch {batch} into a DataFrame: {reason}")]
    Materialize { batch: usize, reason: String },
}

pub type StreamResult<T> = Result<T, StreamError>;
