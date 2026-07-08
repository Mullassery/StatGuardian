use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("parse error: {0}")]
    Parse(#[from] Box<pest::error::Error<crate::parser::Rule>>),

    #[error("compile error: {message}")]
    Compile { message: String },

    #[error("unsupported construct: {0}")]
    Unsupported(String),

    #[error("unknown column '{column}' referenced in rule")]
    UnknownColumn { column: String },

    #[error("type mismatch: column '{column}' has type {actual}, expected {expected}")]
    TypeMismatch {
        column: String,
        actual: String,
        expected: String,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;
