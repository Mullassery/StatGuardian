use std::io::{BufRead, BufReader, Cursor};
use std::time::Instant;

use polars::prelude::*;
use serde_json::Value;
use statguardian_engine::Engine as SgEngine;

use statguardian_core::compiler::dag::ExecutionDag;
use statguardian_core::DataContract;

use crate::error::{StreamError, StreamResult};
use crate::limits::{MalformedPolicy, StreamLimits};
use crate::ndjson::NdjsonSource;
use crate::report::{MalformedKind, MalformedRecord, StreamFinding, StreamLimitHit, StreamReport};
use crate::sanitize::{SanitizationEvent, SanitizationSummary, Sanitizer};

/// Configuration for a stream validation run.
#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub limits: StreamLimits,
    /// Policy for JSON parse errors and non-UTF-8 lines.
    pub malformed: MalformedPolicy,
    /// Policy for lines that exceed `limits.max_record_bytes`.
    pub oversized: MalformedPolicy,
    pub sanitizer: Sanitizer,
    /// When `false` (the default) no raw record content is ever attached to
    /// diagnostics. Set to `true` only when raw payloads are safe to log.
    pub allow_raw_diagnostics: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            limits: StreamLimits::default(),
            malformed: MalformedPolicy::default(),
            oversized: MalformedPolicy::default(),
            sanitizer: Sanitizer::default(),
            allow_raw_diagnostics: false,
        }
    }
}

/// Validates an NDJSON stream record-by-record, reusing the existing
/// StatGuardian engine per micro-batch.
pub struct StreamValidator {
    contract: DataContract,
    dag: ExecutionDag,
    options: StreamOptions,
}

impl StreamValidator {
    pub fn new(contract: DataContract, dag: ExecutionDag, options: StreamOptions) -> Self {
        Self {
            contract,
            dag,
            options,
        }
    }

    pub fn options(&self) -> &StreamOptions {
        &self.options
    }

    /// Validate a file on disk.
    pub fn validate_path(&self, path: &str) -> StreamResult<StreamReport> {
        let file = std::fs::File::open(path)?;
        self.validate_reader(BufReader::new(file))
    }

    /// Validate an in-memory byte buffer.
    pub fn validate_bytes(&self, bytes: &[u8]) -> StreamResult<StreamReport> {
        self.validate_reader(Cursor::new(bytes))
    }

    /// Validate any [`BufRead`] source. Only `batch_size` records are held in
    /// memory at once.
    pub fn validate_reader<R: BufRead>(&self, reader: R) -> StreamResult<StreamReport> {
        let started = Instant::now();
        let engine = SgEngine::new(self.contract.clone(), self.dag.clone());
        let mut src = NdjsonSource::new(reader, self.options.limits.max_record_bytes);
        let mut state = RunState::new(&self.contract.name);

        let mut batch: Vec<(u64, Value)> = Vec::with_capacity(self.options.limits.batch_size);

        loop {
            if let Some(max) = self.options.limits.max_records {
                if src.records_read() >= max {
                    if src.next_line()?.is_some() {
                        state.truncated = true;
                        state.limit_hit = Some(StreamLimitHit {
                            limit: "max_records".into(),
                            value: max,
                        });
                    }
                    break;
                }
            }
            if let Some(max) = self.options.limits.max_bytes {
                if src.bytes_consumed() >= max {
                    if src.next_line()?.is_some() {
                        state.truncated = true;
                        state.limit_hit = Some(StreamLimitHit {
                            limit: "max_bytes".into(),
                            value: max,
                        });
                    }
                    break;
                }
            }

            let line = match src.next_line()? {
                None => break,
                Some(line) => line,
            };

            if line.exceeded {
                state.reject(
                    &self.options.oversized,
                    self.options.allow_raw_diagnostics,
                    line.record,
                    MalformedKind::Oversized,
                    format!(
                        "line exceeded max_record_bytes ({}) and was discarded",
                        self.options.limits.max_record_bytes
                    ),
                    None,
                )?;
                continue;
            }

            if !line.utf8 {
                state.reject(
                    &self.options.malformed,
                    self.options.allow_raw_diagnostics,
                    line.record,
                    MalformedKind::Utf8,
                    "line is not valid UTF-8".to_string(),
                    None,
                )?;
                continue;
            }

            let parsed: Result<Value, serde_json::Error> = serde_json::from_str(&line.content);
            let value = match parsed {
                Ok(v) => v,
                Err(e) => {
                    let preview = self
                        .options
                        .allow_raw_diagnostics
                        .then(|| truncate_preview(&line.content, 200));
                    state.reject(
                        &self.options.malformed,
                        self.options.allow_raw_diagnostics,
                        line.record,
                        MalformedKind::Json,
                        format!("invalid JSON: {e}"),
                        preview,
                    )?;
                    continue;
                }
            };

            let (sanitized, events) = self.options.sanitizer.sanitize(&value);
            state.record_sanitization(&events);

            batch.push((line.record, sanitized));
            if batch.len() >= self.options.limits.batch_size {
                flush_batch(&engine, &mut batch, &mut state)?;
            }
        }

        flush_batch(&engine, &mut batch, &mut state)?;

        state.records_read = src.records_read();
        state.bytes_consumed = src.bytes_consumed();
        Ok(state.finish(started.elapsed().as_millis() as u64))
    }
}

fn flush_batch(
    engine: &SgEngine,
    batch: &mut Vec<(u64, Value)>,
    state: &mut RunState,
) -> StreamResult<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let records = std::mem::take(batch);
    let record_ids: Vec<u64> = records.iter().map(|(r, _)| *r).collect();

    let mut buf: Vec<u8> = Vec::new();
    for (_, value) in &records {
        serde_json::to_writer(&mut buf, value).map_err(|e| StreamError::Materialize {
            batch: state.batches,
            reason: e.to_string(),
        })?;
        buf.push(b'\n');
    }

    let df = JsonReader::new(Cursor::new(buf))
        .with_json_format(JsonFormat::JsonLines)
        .finish()
        .map_err(|e| StreamError::Materialize {
            batch: state.batches,
            reason: e.to_string(),
        })?;

    let report = engine.execute(&df, None);
    let count = records.len() as u64;

    for violation in &report.violations {
        if violation.row_indices.is_empty() {
            state.findings.push(StreamFinding {
                record: None,
                batch: state.batches,
                column: violation.column.clone(),
                check: violation.check.clone(),
                message: violation.message.clone(),
                severity: violation.severity.clone(),
            });
        } else {
            for &local in &violation.row_indices {
                if local < records.len() {
                    state.findings.push(StreamFinding {
                        record: Some(record_ids[local]),
                        batch: state.batches,
                        column: violation.column.clone(),
                        check: violation.check.clone(),
                        message: violation.message.clone(),
                        severity: violation.severity.clone(),
                    });
                }
            }
        }
    }

    state.records_validated += count;
    state.batches += 1;
    Ok(())
}

fn truncate_preview(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// Mutable accumulated state for one run.
struct RunState {
    dataset: String,
    records_read: u64,
    records_validated: u64,
    malformed_count: u64,
    oversized_count: u64,
    batches: usize,
    bytes_consumed: u64,
    truncated: bool,
    findings: Vec<StreamFinding>,
    malformed: Vec<MalformedRecord>,
    sanitization: SanitizationSummary,
    limit_hit: Option<StreamLimitHit>,
}

impl RunState {
    fn new(dataset: &str) -> Self {
        Self {
            dataset: dataset.to_string(),
            records_read: 0,
            records_validated: 0,
            malformed_count: 0,
            oversized_count: 0,
            batches: 0,
            bytes_consumed: 0,
            truncated: false,
            findings: Vec::new(),
            malformed: Vec::new(),
            sanitization: SanitizationSummary::default(),
            limit_hit: None,
        }
    }

    fn record_sanitization(&mut self, events: &[SanitizationEvent]) {
        for e in events {
            self.sanitization.record(e);
        }
    }

    fn reject(
        &mut self,
        policy: &MalformedPolicy,
        _allow_raw: bool,
        record: u64,
        kind: MalformedKind,
        reason: String,
        preview: Option<String>,
    ) -> StreamResult<()> {
        self.malformed_count += 1;
        if kind == MalformedKind::Oversized {
            self.oversized_count += 1;
        }

        match policy {
            MalformedPolicy::FailFast => {
                return Err(match kind {
                    MalformedKind::Oversized => StreamError::OversizedRecord {
                        record,
                        limit: 0,
                        actual: 0,
                    },
                    MalformedKind::Utf8 => StreamError::InvalidUtf8 { record },
                    MalformedKind::Json => StreamError::Malformed { record, reason },
                });
            }
            MalformedPolicy::Skip => {}
            MalformedPolicy::Collect { max } => {
                if self.malformed.len() < *max {
                    self.malformed.push(MalformedRecord {
                        record,
                        kind,
                        reason,
                        preview,
                    });
                }
            }
        }
        Ok(())
    }

    fn finish(mut self, duration_ms: u64) -> StreamReport {
        self.findings.sort_by(|a, b| {
            a.record
                .cmp(&b.record)
                .then_with(|| a.column.cmp(&b.column))
                .then_with(|| a.check.cmp(&b.check))
                .then_with(|| a.batch.cmp(&b.batch))
        });
        self.malformed.sort_by_key(|m| m.record);

        let passed = !self
            .findings
            .iter()
            .any(|f| f.severity == statguardian_core::ast::Severity::Blocking);

        StreamReport {
            dataset: self.dataset,
            records_read: self.records_read,
            records_validated: self.records_validated,
            malformed_count: self.malformed_count,
            oversized_count: self.oversized_count,
            batches: self.batches,
            bytes_consumed: self.bytes_consumed,
            truncated: self.truncated,
            passed,
            duration_ms,
            findings: self.findings,
            malformed: self.malformed,
            sanitization: self.sanitization,
            limit_hit: self.limit_hit,
        }
    }
}
