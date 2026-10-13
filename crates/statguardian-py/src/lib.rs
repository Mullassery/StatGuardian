use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_polars::PyDataFrame;

use statguardian_core::compiler::dag::ExecutionDag;
use statguardian_core::{parse_and_compile, DataContract};
use statguardian_engine::Engine;
use statguardian_lineage::{LineageStore, LineageVersion, SQLiteLineageStore};
use statguardian_metrics::report::ValidationReport;
use statguardian_stream::{MalformedPolicy, StreamLimits, StreamOptions, StreamValidator};

// ── PyDataContract ─────────────────────────────────────────────────────────────

/// A compiled statguardian data contract ready for execution.
///
/// Create with `DataContract.from_dsl(dsl_string)` or `DataContract.from_file(path)`.
#[pyclass(name = "DataContract", module = "statguardian")]
pub struct PyDataContract {
    inner: DataContract,
    dag: ExecutionDag,
}

#[pymethods]
impl PyDataContract {
    /// Parse and compile a contract from DSL source text.
    #[staticmethod]
    fn from_dsl(dsl: &str) -> PyResult<Self> {
        let pairs = parse_and_compile(dsl)
            .map_err(|e| PyValueError::new_err(format!("DSL parse error: {e}")))?;
        let (contract, dag) = pairs
            .into_iter()
            .next()
            .ok_or_else(|| PyValueError::new_err("no datasets defined in DSL"))?;
        Ok(Self {
            inner: contract,
            dag,
        })
    }

    /// Load contract DSL from a file path.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let dsl = std::fs::read_to_string(path)
            .map_err(|e| PyRuntimeError::new_err(format!("cannot read file '{path}': {e}")))?;
        Self::from_dsl(&dsl)
    }

    /// The dataset name from the contract.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Number of schema fields.
    #[getter]
    fn field_count(&self) -> usize {
        self.inner.schema.len()
    }

    /// Number of quality rules.
    #[getter]
    fn quality_rule_count(&self) -> usize {
        self.inner.quality_rules.len()
    }

    /// Number of compiled DAG nodes.
    #[getter]
    fn dag_node_count(&self) -> usize {
        self.dag.node_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "DataContract(name='{}', fields={}, quality_rules={}, dag_nodes={})",
            self.inner.name,
            self.inner.schema.len(),
            self.inner.quality_rules.len(),
            self.dag.node_count(),
        )
    }
}

// ── PyValidationReport ─────────────────────────────────────────────────────────

/// Structured result of executing a contract against a dataset.
#[pyclass(name = "ValidationReport", module = "statguardian")]
pub struct PyValidationReport {
    inner: ValidationReport,
}

#[pymethods]
impl PyValidationReport {
    /// Unique report ID (UUID4).
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// Dataset name.
    #[getter]
    fn dataset(&self) -> &str {
        &self.inner.dataset
    }

    /// Whether all blocking checks passed.
    #[getter]
    fn passed(&self) -> bool {
        self.inner.passed
    }

    /// Health score in [0, 1].
    #[getter]
    fn health_score(&self) -> f64 {
        self.inner.health.score
    }

    /// Letter grade: A/B/C/D/F.
    #[getter]
    fn grade(&self) -> &str {
        &self.inner.health.grade
    }

    /// Number of rows processed.
    #[getter]
    fn row_count(&self) -> usize {
        self.inner.row_count
    }

    /// Total violation count.
    #[getter]
    fn violation_count(&self) -> usize {
        self.inner.violations.len()
    }

    /// Execution time in milliseconds.
    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    /// Full report as a JSON string.
    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    /// Full report as a pretty-printed JSON string.
    fn to_json_pretty(&self) -> String {
        self.inner.to_json_pretty()
    }

    /// Prometheus text exposition format.
    fn to_prometheus(&self) -> String {
        self.inner.to_prometheus()
    }

    /// Violations as a list of dicts (column, check, message, severity).
    fn violations(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .violations
            .iter()
            .map(|v| {
                let d = PyDict::new(py);
                d.set_item("column", &v.column)?;
                d.set_item("check", &v.check)?;
                d.set_item("message", &v.message)?;
                d.set_item("severity", format!("{:?}", v.severity))?;
                d.set_item("observed", v.observed)?;
                d.set_item("expected", v.expected)?;
                Ok(d.into())
            })
            .collect()
    }

    /// Drift results as a list of dicts.
    fn drift_results(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .drift_results
            .iter()
            .map(|r| {
                let d = PyDict::new(py);
                d.set_item("column", &r.column)?;
                d.set_item("stat", &r.stat)?;
                d.set_item("reference_value", r.reference_value)?;
                d.set_item("current_value", r.current_value)?;
                d.set_item("drift", r.drift)?;
                d.set_item("threshold", r.threshold)?;
                d.set_item("passed", r.passed)?;
                d.set_item("psi", r.psi)?;
                d.set_item("ks_stat", r.ks_stat)?;
                Ok(d.into())
            })
            .collect()
    }

    /// Column profiles as a list of dicts.
    fn column_profiles(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .column_profiles
            .iter()
            .map(|p| {
                let d = PyDict::new(py);
                d.set_item("name", &p.name)?;
                d.set_item("dtype", &p.dtype)?;
                d.set_item("row_count", p.row_count)?;
                d.set_item("null_count", p.null_count)?;
                d.set_item("null_rate", p.null_rate)?;
                d.set_item("distinct_count", p.distinct_count)?;
                d.set_item("min", p.min)?;
                d.set_item("max", p.max)?;
                d.set_item("mean", p.mean)?;
                d.set_item("std", p.std)?;
                d.set_item("median", p.median)?;
                d.set_item("p05", p.p05)?;
                d.set_item("p95", p.p95)?;
                d.set_item("p99", p.p99)?;
                Ok(d.into())
            })
            .collect()
    }

    fn summary(&self) -> String {
        self.inner.summary().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "ValidationReport(passed={}, score={:.3}, grade='{}', violations={}, rows={})",
            self.inner.passed,
            self.inner.health.score,
            self.inner.health.grade,
            self.inner.violations.len(),
            self.inner.row_count,
        )
    }
}

// ── PyStreamReport ──────────────────────────────────────────────────────────────

/// Result of incremental NDJSON validation via `execute_stream_ndjson`.
#[pyclass(name = "StreamReport", module = "statguardian")]
pub struct PyStreamReport {
    inner: statguardian_stream::StreamReport,
}

#[pymethods]
impl PyStreamReport {
    #[getter]
    fn dataset(&self) -> &str {
        &self.inner.dataset
    }

    /// Records read from the stream (including malformed/oversized lines).
    #[getter]
    fn records_read(&self) -> u64 {
        self.inner.records_read
    }

    /// Records that were parsed and passed to the engine.
    #[getter]
    fn records_validated(&self) -> u64 {
        self.inner.records_validated
    }

    #[getter]
    fn malformed_count(&self) -> u64 {
        self.inner.malformed_count
    }

    #[getter]
    fn oversized_count(&self) -> u64 {
        self.inner.oversized_count
    }

    #[getter]
    fn batches(&self) -> usize {
        self.inner.batches
    }

    #[getter]
    fn bytes_consumed(&self) -> u64 {
        self.inner.bytes_consumed
    }

    /// True when a resource limit stopped the run before end-of-input.
    #[getter]
    fn truncated(&self) -> bool {
        self.inner.truncated
    }

    /// True when there are no blocking findings.
    #[getter]
    fn passed(&self) -> bool {
        self.inner.passed
    }

    #[getter]
    fn duration_ms(&self) -> u64 {
        self.inner.duration_ms
    }

    #[getter]
    fn blocking_count(&self) -> usize {
        self.inner.blocking_count()
    }

    #[getter]
    fn warning_count(&self) -> usize {
        self.inner.warning_count()
    }

    /// Total field-level sanitization events applied (never contains raw values).
    #[getter]
    fn sanitization_events(&self) -> u64 {
        self.inner.sanitization.total_events
    }

    /// Process exit code: 0 passed, 1 validation failed.
    ///
    /// Execution errors (exit 2) are raised as Python exceptions instead.
    #[pyo3(signature = (fail_on_warning=false))]
    fn exit_code(&self, fail_on_warning: bool) -> i32 {
        self.inner.exit_code(fail_on_warning)
    }

    /// Findings as a list of dicts.
    fn findings(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .findings
            .iter()
            .map(|f| {
                let d = PyDict::new(py);
                d.set_item("record", f.record)?;
                d.set_item("batch", f.batch)?;
                d.set_item("column", &f.column)?;
                d.set_item("check", &f.check)?;
                d.set_item("message", &f.message)?;
                d.set_item("severity", format!("{:?}", f.severity))?;
                Ok(d.into())
            })
            .collect()
    }

    /// Collected malformed-record diagnostics (empty unless the policy is `collect`).
    fn malformed(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.inner
            .malformed
            .iter()
            .map(|m| {
                let d = PyDict::new(py);
                d.set_item("record", m.record)?;
                d.set_item("kind", format!("{:?}", m.kind))?;
                d.set_item("reason", &m.reason)?;
                d.set_item("preview", &m.preview)?;
                Ok(d.into())
            })
            .collect()
    }

    fn to_json(&self) -> String {
        self.inner.to_json()
    }

    fn to_json_pretty(&self) -> String {
        self.inner.to_json_pretty()
    }

    fn to_prometheus(&self) -> String {
        self.inner.to_prometheus()
    }

    fn summary(&self) -> String {
        self.inner.summary()
    }

    fn __repr__(&self) -> String {
        format!(
            "StreamReport(passed={}, records={}/{}, findings={}, malformed={}, truncated={})",
            self.inner.passed,
            self.inner.records_validated,
            self.inner.records_read,
            self.inner.findings.len(),
            self.inner.malformed_count,
            self.inner.truncated,
        )
    }
}

// ── Module-level functions ────────────────────────────────────────────────────

/// Execute a contract against a Polars DataFrame.
///
/// Args:
///     contract: A compiled `DataContract` object.
///     df:       A Polars `DataFrame` to validate.
///     reference: Optional Polars `DataFrame` to use as drift reference.
///
/// Returns:
///     A `ValidationReport` object.
#[pyfunction]
#[pyo3(signature = (contract, df, reference=None))]
fn execute(
    contract: &PyDataContract,
    df: PyDataFrame,
    reference: Option<PyDataFrame>,
) -> PyResult<PyValidationReport> {
    let frame: polars::prelude::DataFrame = df.into();
    let ref_frame: Option<polars::prelude::DataFrame> = reference.map(|r| r.into());

    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let report = engine.execute(&frame, ref_frame.as_ref());

    Ok(PyValidationReport { inner: report })
}

/// Execute a contract against a file (auto-detects format: parquet, csv, json, ipc).
///
/// Args:
///     contract:        A compiled `DataContract`.
///     path:            Path to the data file.
///     reference_path:  Optional path to a reference data file for drift detection.
#[pyfunction]
#[pyo3(signature = (contract, path, reference_path=None))]
fn execute_file(
    contract: &PyDataContract,
    path: &str,
    reference_path: Option<&str>,
) -> PyResult<PyValidationReport> {
    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let report = engine
        .execute_file(path, reference_path)
        .map_err(|e| PyRuntimeError::new_err(format!("IO error: {e}")))?;
    Ok(PyValidationReport { inner: report })
}

/// Execute a contract in streaming mode, processing the file in chunks.
///
/// Args:
///     contract:   A compiled `DataContract`.
///     path:       Path to the data file.
///     batch_size: Number of rows per batch (default: 10000).
///
/// Returns:
///     A list of `ValidationReport` objects, one per batch.
#[pyfunction]
#[pyo3(signature = (contract, path, batch_size=10000))]
fn execute_streaming(
    contract: &PyDataContract,
    path: &str,
    batch_size: usize,
) -> PyResult<Vec<PyValidationReport>> {
    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let reports = engine
        .execute_streaming(path, batch_size)
        .map_err(|e| PyRuntimeError::new_err(format!("streaming error: {e}")))?;
    Ok(reports
        .into_iter()
        .map(|r| PyValidationReport { inner: r })
        .collect())
}

// ── Streaming NDJSON (StreamGuard) ─────────────────────────────────────────────

/// Incrementally validate an NDJSON file with bounded memory.
///
/// Records are read one line at a time, buffered into micro-batches of
/// `batch_size`, and validated with the same engine used by `execute`.
///
/// Args:
///     contract:                A compiled `DataContract`.
///     path:                    Path to an NDJSON (newline-delimited JSON) file.
///     batch_size:              Records per validation micro-batch (default: 1000).
///     max_records:             Optional cap on records read.
///     max_bytes:               Optional cap on source bytes consumed.
///     max_record_bytes:        Per-line byte cap (default: 8 MiB).
///     malformed:               One of "fail_fast", "skip", "collect" (default: "collect").
///     collect_max:             Max malformed diagnostics retained when collecting (default: 100).
///     allow_raw_diagnostics:   Attach raw line previews to malformed diagnostics (default: False).
///
/// Returns:
///     A `StreamReport`.
#[pyfunction]
#[pyo3(signature = (
    contract,
    path,
    batch_size=None,
    max_records=None,
    max_bytes=None,
    max_record_bytes=None,
    malformed="collect",
    collect_max=100,
    allow_raw_diagnostics=false,
))]
#[allow(clippy::too_many_arguments)]
fn execute_stream_ndjson(
    contract: &PyDataContract,
    path: &str,
    batch_size: Option<usize>,
    max_records: Option<u64>,
    max_bytes: Option<u64>,
    max_record_bytes: Option<usize>,
    malformed: &str,
    collect_max: usize,
    allow_raw_diagnostics: bool,
) -> PyResult<PyStreamReport> {
    let policy = match malformed.to_ascii_lowercase().as_str() {
        "fail_fast" | "failfast" => MalformedPolicy::FailFast,
        "skip" => MalformedPolicy::Skip,
        "collect" => MalformedPolicy::Collect { max: collect_max },
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown malformed policy '{other}' (expected fail_fast|skip|collect)"
            )))
        }
    };

    let defaults = StreamLimits::default();
    let options = StreamOptions {
        limits: StreamLimits {
            max_records,
            max_bytes,
            max_record_bytes: max_record_bytes.unwrap_or(defaults.max_record_bytes),
            batch_size: batch_size.unwrap_or(defaults.batch_size),
        },
        malformed: policy.clone(),
        oversized: policy,
        sanitizer: Default::default(),
        allow_raw_diagnostics,
    };

    let validator = StreamValidator::new(contract.inner.clone(), contract.dag.clone(), options);
    let report = validator
        .validate_path(path)
        .map_err(|e| PyRuntimeError::new_err(format!("stream error: {e}")))?;
    Ok(PyStreamReport { inner: report })
}

// ── Delta Lake ────────────────────────────────────────────────────────────────

/// Execute a contract against a Delta Lake table directory.
///
/// Args:
///     contract:         A compiled `DataContract`.
///     table_path:       Path to the Delta table root directory (contains `_delta_log/`).
///     version:          Optional Delta version for time-travel (default: latest).
///     reference_path:   Optional Delta table path to use as drift reference.
///     reference_version: Optional version of the reference table.
#[pyfunction]
#[pyo3(signature = (contract, table_path, version=None, reference_path=None, reference_version=None))]
fn execute_delta(
    contract: &PyDataContract,
    table_path: &str,
    version: Option<u64>,
    reference_path: Option<&str>,
    reference_version: Option<u64>,
) -> PyResult<PyValidationReport> {
    let df = statguardian_io::DeltaReader::read_version(table_path, version)
        .map_err(|e| PyRuntimeError::new_err(format!("Delta read error: {e}")))?;

    let ref_df = match (reference_path, reference_version) {
        (Some(rp), rv) => Some(
            statguardian_io::DeltaReader::read_version(rp, rv)
                .map_err(|e| PyRuntimeError::new_err(format!("Delta reference read error: {e}")))?,
        ),
        _ => None,
    };

    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let report = engine.execute(&df, ref_df.as_ref());
    Ok(PyValidationReport { inner: report })
}

/// Compare two Delta Lake versions for drift analysis.
///
/// Convenience wrapper around `execute_delta` that takes explicit version numbers
/// and sets up drift detection automatically.
///
/// Args:
///     contract:          A compiled `DataContract` (must have `stats` rules).
///     table_path:        Path to the Delta table root.
///     current_version:   The version to validate (default: latest).
///     reference_version: The version to compare against.
#[pyfunction]
#[pyo3(signature = (contract, table_path, reference_version, current_version=None))]
fn compare_delta_versions(
    contract: &PyDataContract,
    table_path: &str,
    reference_version: u64,
    current_version: Option<u64>,
) -> PyResult<PyValidationReport> {
    let (reference, current) = statguardian_io::DeltaReader::read_two_versions(
        table_path,
        reference_version,
        current_version.unwrap_or(u64::MAX),
    )
    .map_err(|e| PyRuntimeError::new_err(format!("Delta compare error: {e}")))?;

    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let report = engine.execute(&current, Some(&reference));
    Ok(PyValidationReport { inner: report })
}

// ── Apache Iceberg ────────────────────────────────────────────────────────────

/// Execute a contract against an Apache Iceberg table directory.
///
/// Args:
///     contract:            A compiled `DataContract`.
///     table_path:          Path to the Iceberg table root (contains `metadata/`).
///     snapshot_id:         Optional snapshot ID for time-travel.
///     reference_snapshot:  Optional snapshot ID to use as drift reference.
#[pyfunction]
#[pyo3(signature = (contract, table_path, snapshot_id=None, reference_snapshot=None))]
fn execute_iceberg(
    contract: &PyDataContract,
    table_path: &str,
    snapshot_id: Option<i64>,
    reference_snapshot: Option<i64>,
) -> PyResult<PyValidationReport> {
    let df = statguardian_io::IcebergReader::read_snapshot(table_path, snapshot_id)
        .map_err(|e| PyRuntimeError::new_err(format!("Iceberg read error: {e}")))?;

    let ref_df = match reference_snapshot {
        Some(ref_id) => Some(
            statguardian_io::IcebergReader::read_snapshot(table_path, Some(ref_id)).map_err(
                |e| PyRuntimeError::new_err(format!("Iceberg reference read error: {e}")),
            )?,
        ),
        None => None,
    };

    let engine = Engine::new(contract.inner.clone(), contract.dag.clone());
    let report = engine.execute(&df, ref_df.as_ref());
    Ok(PyValidationReport { inner: report })
}

/// List all snapshots of an Iceberg table.
///
/// Returns a list of dicts with keys: snapshot_id, timestamp_ms,
/// parent_snapshot_id, operation.
#[pyfunction]
fn list_iceberg_snapshots(table_path: &str, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
    let snapshots = statguardian_io::IcebergReader::list_snapshots(table_path)
        .map_err(|e| PyRuntimeError::new_err(format!("Iceberg error: {e}")))?;

    snapshots
        .iter()
        .map(|s| {
            let d = PyDict::new(py);
            d.set_item("snapshot_id", s.snapshot_id)?;
            d.set_item("timestamp_ms", s.timestamp_ms)?;
            d.set_item("parent_snapshot_id", s.parent_snapshot_id)?;
            d.set_item("operation", s.operation.as_deref())?;
            Ok(d.into())
        })
        .collect()
}

/// Parse and validate DSL syntax without executing.
/// Returns the contract name if valid, raises ValueError on parse error.
#[pyfunction]
fn validate_dsl(dsl: &str) -> PyResult<String> {
    let pairs =
        parse_and_compile(dsl).map_err(|e| PyValueError::new_err(format!("DSL error: {e}")))?;
    let name = pairs
        .first()
        .map(|(c, _)| c.name.clone())
        .unwrap_or_default();
    Ok(name)
}

// ── Lineage ────────────────────────────────────────────────────────────────────
//
// Bindings for statguardian-lineage's SQLite-backed lineage store. The FFI
// boundary is JSON strings (the crate already implements Serialize/Deserialize
// for LineageVersion via serde), which lets the pure-Python side reuse the
// LineageVersion/LineageGraph dataclasses and `*_from_dict` helpers it already
// defines in `statguardian/_lineage.py` instead of duplicating them as pyclasses.

/// Persist a lineage version snapshot to the SQLite store at `db_path`.
///
/// Args:
///     db_path:      Path to the SQLite lineage database (created if missing).
///     version_json: A LineageVersion serialized as JSON.
#[pyfunction]
fn lineage_save_version(db_path: &str, version_json: &str) -> PyResult<()> {
    let version: LineageVersion = serde_json::from_str(version_json)
        .map_err(|e| PyValueError::new_err(format!("invalid lineage version JSON: {e}")))?;

    let mut store = SQLiteLineageStore::new(db_path)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage store error: {e}")))?;

    store
        .save_version(&version)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage save error: {e}")))?;

    for change in &version.changes_from_previous {
        store
            .save_change(change, &version.version_id)
            .map_err(|e| PyRuntimeError::new_err(format!("lineage change save error: {e}")))?;
    }

    Ok(())
}

/// Get a specific lineage version for a warehouse, as JSON. Returns None if not found.
#[pyfunction]
fn lineage_get_version(
    db_path: &str,
    warehouse_id: &str,
    version_number: u32,
) -> PyResult<Option<String>> {
    let store = SQLiteLineageStore::new(db_path)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage store error: {e}")))?;

    let version = store
        .get_version(warehouse_id, version_number)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage query error: {e}")))?;

    version
        .map(|v| {
            serde_json::to_string(&v)
                .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
        })
        .transpose()
}

/// Get the latest lineage version for a warehouse, as JSON. Returns None if none exist.
#[pyfunction]
fn lineage_get_latest_version(db_path: &str, warehouse_id: &str) -> PyResult<Option<String>> {
    let store = SQLiteLineageStore::new(db_path)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage store error: {e}")))?;

    let version = store
        .get_latest_version(warehouse_id)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage query error: {e}")))?;

    version
        .map(|v| {
            serde_json::to_string(&v)
                .map_err(|e| PyRuntimeError::new_err(format!("serialization error: {e}")))
        })
        .transpose()
}

/// List version numbers for a warehouse, most recent first.
#[pyfunction]
fn lineage_list_versions(db_path: &str, warehouse_id: &str, limit: usize) -> PyResult<Vec<u32>> {
    let store = SQLiteLineageStore::new(db_path)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage store error: {e}")))?;

    store
        .list_versions(warehouse_id, limit)
        .map_err(|e| PyRuntimeError::new_err(format!("lineage query error: {e}")))
}

// ── Module definition ─────────────────────────────────────────────────────────

#[pymodule]
fn _statguardian(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDataContract>()?;
    m.add_class::<PyValidationReport>()?;
    m.add_class::<PyStreamReport>()?;

    // Core execution
    m.add_function(wrap_pyfunction!(execute, m)?)?;
    m.add_function(wrap_pyfunction!(execute_file, m)?)?;
    m.add_function(wrap_pyfunction!(execute_streaming, m)?)?;

    // Streaming NDJSON (StreamGuard)
    m.add_function(wrap_pyfunction!(execute_stream_ndjson, m)?)?;

    // Delta Lake
    m.add_function(wrap_pyfunction!(execute_delta, m)?)?;
    m.add_function(wrap_pyfunction!(compare_delta_versions, m)?)?;

    // Apache Iceberg
    m.add_function(wrap_pyfunction!(execute_iceberg, m)?)?;
    m.add_function(wrap_pyfunction!(list_iceberg_snapshots, m)?)?;

    // Utilities
    m.add_function(wrap_pyfunction!(validate_dsl, m)?)?;

    // Lineage
    m.add_function(wrap_pyfunction!(lineage_save_version, m)?)?;
    m.add_function(wrap_pyfunction!(lineage_get_version, m)?)?;
    m.add_function(wrap_pyfunction!(lineage_get_latest_version, m)?)?;
    m.add_function(wrap_pyfunction!(lineage_list_versions, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
