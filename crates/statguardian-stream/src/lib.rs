//! # StatGuardian StreamGuard / AgentGuard
//!
//! Optional, additive extensions to StatGuardian for:
//!
//! - **StreamGuard** — incremental, bounded-memory validation of NDJSON
//!   records. Records are buffered into micro-batches and run through the
//!   existing validation engine, so contracts, rules, findings, and reporting
//!   are reused rather than reimplemented.
//! - **Sanitization** — explicit, opt-in field redaction and normalization
//!   that never mutates the source and never leaks raw payloads in
//!   diagnostics.
//! - **AgentGuard** ([`agent`]) — an optional, SDK-free model for agent-runtime
//!   telemetry (tool calls, tokens, cost, retries, duration) with deterministic
//!   policies that distinguish measured from estimated data.
//!
//! This crate does not depend on any LLM SDK, network client, or async runtime.
//!
//! ## Example
//!
//! ```no_run
//! use std::io::BufReader;
//! use statguardian_core::parse_and_compile;
//! use statguardian_stream::{StreamValidator, StreamOptions};
//!
//! let (contract, dag) = parse_and_compile(
//!     "dataset events { schema { id: int, not_null } }",
//! )
//! .unwrap()
//! .into_iter()
//! .next()
//! .unwrap();
//!
//! let file = std::fs::File::open("events.ndjson").unwrap();
//! let validator = StreamValidator::new(contract, dag, StreamOptions::default());
//! let report = validator.validate_reader(BufReader::new(file)).unwrap();
//! println!("{}", report.summary());
//! std::process::exit(report.exit_code(false));
//! ```

pub mod agent;
pub mod error;
pub mod limits;
pub mod ndjson;
pub mod report;
pub mod sanitize;
pub mod validator;

pub use error::{StreamError, StreamResult};
pub use limits::{MalformedPolicy, StreamLimits};
pub use ndjson::{NdjsonSource, RawLine};
pub use report::{MalformedKind, MalformedRecord, StreamFinding, StreamLimitHit, StreamReport};
pub use sanitize::{
    NormalizationPolicy, RedactionPolicy, RedactionRule, RedactionStrategy, SanitizationEvent,
    SanitizationKind, SanitizationSummary, Sanitizer,
};
pub use validator::{StreamOptions, StreamValidator};
