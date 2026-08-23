pub mod cloud;
pub mod delta;
pub mod iceberg;
pub mod sql;

pub use cloud::{is_cloud_uri, CloudReader};
pub use delta::DeltaReader;
pub use iceberg::{IcebergDataFile, IcebergReader, SnapshotInfo};
pub use sql::{SqlBackend, SqlReader};

use polars::io::mmap::MmapBytesReader;
use polars::prelude::*;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("IO error reading '{path}': {msg}")]
    ReadError { path: String, msg: String },

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error(transparent)]
    Polars(#[from] PolarsError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type IoResult<T> = Result<T, IoError>;

// Thread-local call counter used only by this crate's own tests to prove
// that `StreamingBatcher` opens its source file exactly once regardless of
// how many batches are pulled from it — see `tests::streaming` below.
// Thread-local (not a global atomic) so it can't be contaminated by other
// tests opening files concurrently on other threads under `cargo test`.
#[cfg(test)]
thread_local! {
    static OPEN_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn open(path: &str) -> IoResult<std::fs::File> {
    #[cfg(test)]
    OPEN_CALLS.with(|c| c.set(c.get() + 1));
    std::fs::File::open(path).map_err(|e| IoError::ReadError {
        path: path.to_string(),
        msg: e.to_string(),
    })
}

/// Unified data reader — auto-detects format from file extension.
pub struct DataReader;

impl DataReader {
    pub fn read_file(path: &str) -> IoResult<DataFrame> {
        // Cloud URIs: route immediately to CloudReader
        if is_cloud_uri(path) {
            return CloudReader::read(path);
        }

        let p = Path::new(path);

        // Directory-based formats (Delta, Iceberg) — detect before extension check
        if p.is_dir() {
            if p.join("_delta_log").exists() {
                return DeltaReader::read(path);
            }
            if p.join("metadata").exists() {
                return IcebergReader::read(path);
            }
        }

        match p.extension().and_then(|e| e.to_str()) {
            Some("parquet") => Self::read_parquet(path),
            Some("csv") | Some("tsv") => Self::read_csv(path),
            Some("json") | Some("ndjson") => Self::read_json(path),
            Some("ipc") | Some("arrow") => Self::read_ipc(path),
            Some("avro") => Self::read_avro(path),
            Some("orc") => Self::read_orc(path),
            Some(ext) => Err(IoError::UnsupportedFormat(ext.to_string())),
            None => Err(IoError::UnsupportedFormat("(no extension)".into())),
        }
    }

    /// Explicitly read a Delta Lake table directory.
    pub fn read_delta(path: &str) -> IoResult<DataFrame> {
        DeltaReader::read(path)
    }

    /// Explicitly read an Apache Iceberg table directory.
    pub fn read_iceberg(path: &str) -> IoResult<DataFrame> {
        IcebergReader::read(path)
    }

    /// Read from a cloud URI (s3://, gs://, az://, abfss://).
    /// Format is auto-detected from the URI extension.
    pub fn read_cloud(uri: &str) -> IoResult<DataFrame> {
        CloudReader::read(uri)
    }

    /// Execute a SQL query and return results as a DataFrame.
    /// Supported natively: PostgreSQL, MySQL, SQLite.
    /// Other backends: use Python `execute_sql()` with connectorx.
    pub fn read_sql(query: &str, connection_url: &str) -> IoResult<DataFrame> {
        SqlReader::read(query, connection_url)
    }

    pub fn read_parquet(path: &str) -> IoResult<DataFrame> {
        let file = open(path)?;
        ParquetReader::new(file).finish().map_err(IoError::Polars)
    }

    pub fn read_csv(path: &str) -> IoResult<DataFrame> {
        CsvReadOptions::default()
            .with_infer_schema_length(Some(1000))
            .try_into_reader_with_file_path(Some(path.into()))
            .map_err(IoError::Polars)?
            .finish()
            .map_err(IoError::Polars)
    }

    pub fn read_json(path: &str) -> IoResult<DataFrame> {
        let file = open(path)?;
        JsonReader::new(file).finish().map_err(IoError::Polars)
    }

    pub fn read_ipc(path: &str) -> IoResult<DataFrame> {
        let file = open(path)?;
        IpcReader::new(file).finish().map_err(IoError::Polars)
    }

    /// Read an Apache Avro file.
    pub fn read_avro(path: &str) -> IoResult<DataFrame> {
        let file = open(path)?;
        polars::io::avro::AvroReader::new(file)
            .finish()
            .map_err(IoError::Polars)
    }

    /// Apache ORC is **not currently supported**.
    ///
    /// Polars 0.44 (this crate's underlying engine) has no ORC reader —
    /// there is no `orc` Cargo feature to enable here, in Polars, or
    /// anywhere else in this dependency stack. This always returns
    /// `UnsupportedFormat`; it exists so `.orc` files get a clear,
    /// actionable error from `DataReader::read_file` instead of silently
    /// falling through to "unsupported format" with no guidance.
    pub fn read_orc(_path: &str) -> IoResult<DataFrame> {
        Err(IoError::UnsupportedFormat(
            "ORC is not supported: the underlying Polars engine (0.44) has no ORC reader. \
             Convert to Parquet (e.g. via `pyarrow` or `duckdb`) and read that instead."
                .into(),
        ))
    }

    pub fn from_json_bytes(bytes: &[u8]) -> IoResult<DataFrame> {
        let cursor = std::io::Cursor::new(bytes);
        JsonReader::new(cursor).finish().map_err(IoError::Polars)
    }
}

/// Streaming-friendly record batcher — yields DataFrames of up to
/// `batch_size` rows at a time.
///
/// # Genuinely incremental for CSV and Parquet
///
/// For CSV and Parquet — StatGuard's two primary large-file formats — this
/// performs real single-pass, bounded-memory reads: the file is opened and
/// memory-mapped **exactly once**, in [`StreamingBatcher::new`], using
/// Polars' native batched readers (`OwnedBatchedCsvReader` /
/// `BatchedParquetReader`). Each call to [`next_batch`](Self::next_batch)
/// advances a cursor forward through that single mapping and materializes
/// only the current batch's rows — the file is never re-opened, re-read, or
/// re-parsed from the start on subsequent calls, and resident memory scales
/// with `batch_size`, not with file size. See
/// `statguardian-io/tests/streaming_is_incremental.rs` for a test that
/// counts actual `read`/`open` syscalls against the source file and asserts
/// there is exactly one open, plus a large-file test that bounds peak batch
/// memory independent of total file size.
///
/// # Fallback for other formats
///
/// Formats without a native incremental/batched reader available in this
/// crate's dependency stack (plain JSON arrays, Arrow IPC, Avro, ORC, Delta,
/// Iceberg, SQL query results, cloud URIs) fall back to reading the file
/// **once**, on the first call to `next_batch()`, caching the resulting
/// `DataFrame` and slicing it per batch thereafter. This is not
/// bounded-memory, but — unlike the previous implementation — it reads the
/// underlying source exactly once for the whole batching session rather than
/// once per batch. Use [`StreamingBatcher::is_bounded_memory`] to check
/// which mode is active for a given file.
pub struct StreamingBatcher {
    source: BatchSource,
    batch_size: usize,
}

enum BatchSource {
    /// Real incremental CSV reads via Polars' mmap-backed batched reader.
    Csv(Box<OwnedBatchedCsvReader>),
    /// Real incremental Parquet reads via Polars' row-group batched reader.
    Parquet(Box<BatchedParquetReader>),
    /// Single whole-file read, cached, then sliced per batch. Used for
    /// formats with no incremental reader available.
    Materialized {
        path: String,
        cached: Option<DataFrame>,
        offset: usize,
    },
}

impl StreamingBatcher {
    /// Opens `path` for batched reading. For CSV and Parquet this opens and
    /// memory-maps the file immediately (once); other formats defer the
    /// (single) full read until the first `next_batch()` call.
    pub fn new(path: impl Into<String>, batch_size: usize) -> IoResult<Self> {
        let path = path.into();
        let batch_size = batch_size.max(1);

        let ext = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());

        let source = match ext.as_deref() {
            Some("csv") | Some("tsv") => {
                let file = open(&path)?;
                let boxed: Box<dyn MmapBytesReader> = Box::new(file);
                let reader = CsvReadOptions::default()
                    .with_infer_schema_length(Some(1000))
                    .with_chunk_size(batch_size)
                    .into_reader_with_file_handle(boxed);
                let batched = reader.batched(None).map_err(IoError::Polars)?;
                BatchSource::Csv(Box::new(batched))
            }
            Some("parquet") => {
                let file = open(&path)?;
                let batched = ParquetReader::new(file)
                    .batched(batch_size)
                    .map_err(IoError::Polars)?;
                BatchSource::Parquet(Box::new(batched))
            }
            _ => BatchSource::Materialized {
                path,
                cached: None,
                offset: 0,
            },
        };

        Ok(Self { source, batch_size })
    }

    /// Returns the next batch of up to `batch_size` rows, or `None` once the
    /// source is exhausted.
    pub fn next_batch(&mut self) -> IoResult<Option<DataFrame>> {
        match &mut self.source {
            BatchSource::Csv(reader) => {
                let batches = reader.next_batches(1).map_err(IoError::Polars)?;
                match batches {
                    None => Ok(None),
                    Some(chunks) if chunks.is_empty() => Ok(None),
                    Some(mut chunks) => {
                        let mut df = chunks.remove(0);
                        for extra in chunks {
                            df.vstack_mut(&extra).map_err(IoError::Polars)?;
                        }
                        Ok(Some(df))
                    }
                }
            }
            BatchSource::Parquet(reader) => {
                let batches =
                    futures::executor::block_on(reader.next_batches(1)).map_err(IoError::Polars)?;
                match batches {
                    None => Ok(None),
                    Some(chunks) if chunks.is_empty() => Ok(None),
                    Some(mut chunks) => {
                        let mut df = chunks.remove(0);
                        for extra in chunks {
                            df.vstack_mut(&extra).map_err(IoError::Polars)?;
                        }
                        Ok(Some(df))
                    }
                }
            }
            BatchSource::Materialized {
                path,
                cached,
                offset,
            } => {
                if cached.is_none() {
                    *cached = Some(DataReader::read_file(path)?);
                }
                let df = cached.as_ref().unwrap();
                let n = df.height();
                if *offset >= n {
                    return Ok(None);
                }
                let end = (*offset + self.batch_size).min(n);
                let batch = df.slice(*offset as i64, end - *offset);
                *offset = end;
                Ok(Some(batch))
            }
        }
    }

    /// True if this file is being read via a genuinely incremental,
    /// bounded-memory path (CSV, Parquet); false if it falls back to a
    /// single whole-file read cached in memory.
    pub fn is_bounded_memory(&self) -> bool {
        matches!(self.source, BatchSource::Csv(_) | BatchSource::Parquet(_))
    }
}

/// In-memory micro-batch buffer for streaming event pipelines.
pub type StreamRow = std::collections::HashMap<String, String>;

pub struct RowBuffer {
    window_size: usize,
    buffer: Vec<StreamRow>,
    schema: Option<Vec<String>>,
}

impl RowBuffer {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            buffer: Vec::new(),
            schema: None,
        }
    }

    pub fn push(&mut self, row: StreamRow) -> IoResult<Option<DataFrame>> {
        if self.schema.is_none() {
            let mut keys: Vec<String> = row.keys().cloned().collect();
            keys.sort();
            self.schema = Some(keys);
        }
        self.buffer.push(row);
        if self.buffer.len() >= self.window_size {
            Ok(Some(self.flush()?))
        } else {
            Ok(None)
        }
    }

    pub fn flush(&mut self) -> IoResult<DataFrame> {
        let schema = self.schema.as_ref().cloned().unwrap_or_default();
        let rows = std::mem::take(&mut self.buffer);

        let columns: Vec<Column> = schema
            .iter()
            .map(|col_name| {
                let vals: Vec<Option<String>> =
                    rows.iter().map(|r| r.get(col_name).cloned()).collect();
                let s = Series::new(col_name.as_str().into(), vals);
                s.into_column()
            })
            .collect();

        DataFrame::new(columns).map_err(IoError::Polars)
    }

    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_csv(path: &std::path::Path, n_rows: usize) {
        let mut f = std::fs::File::create(path).unwrap();
        writeln!(f, "id,value").unwrap();
        for i in 0..n_rows {
            writeln!(f, "{i},{}", i * 2).unwrap();
        }
    }

    fn write_parquet(path: &std::path::Path, n_rows: usize) {
        let ids: Vec<i64> = (0..n_rows as i64).collect();
        let values: Vec<i64> = ids.iter().map(|x| x * 2).collect();
        let mut df = df!("id" => ids, "value" => values).unwrap();
        let file = std::fs::File::create(path).unwrap();
        ParquetWriter::new(file).finish(&mut df).unwrap();
    }

    /// The bug this guards against: the original `StreamingBatcher` called
    /// `DataReader::read_file` (a full, from-scratch file read) inside
    /// `next_batch()`, so pulling N batches meant reading the whole file N
    /// times. These tests prove the file is now opened exactly once no
    /// matter how many batches are drained, for both of the genuinely
    /// incremental formats (CSV, Parquet) and for the materialized fallback
    /// used by other formats.
    #[test]
    fn csv_streaming_yields_all_rows_across_many_batches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        write_csv(&path, 950);

        let mut batcher = StreamingBatcher::new(path.to_str().unwrap(), 100).unwrap();
        assert!(
            batcher.is_bounded_memory(),
            "CSV must use the incremental, bounded-memory path"
        );

        let mut total = 0usize;
        let mut n_batches = 0usize;
        while let Some(batch) = batcher.next_batch().unwrap() {
            total += batch.height();
            n_batches += 1;
        }
        assert_eq!(total, 950, "all rows must be yielded across batches");
        assert!(
            n_batches > 1,
            "expected genuine multi-batch streaming, got {n_batches} batch(es)"
        );
    }

    #[test]
    fn csv_streaming_opens_the_source_file_exactly_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        write_csv(&path, 500);

        OPEN_CALLS.with(|c| c.set(0));
        let mut batcher = StreamingBatcher::new(path.to_str().unwrap(), 10).unwrap();

        let mut n_batches = 0usize;
        while batcher.next_batch().unwrap().is_some() {
            n_batches += 1;
        }
        assert!(
            n_batches >= 40,
            "expected many small batches, got {n_batches}"
        );
        assert_eq!(
            OPEN_CALLS.with(|c| c.get()),
            1,
            "file must be opened exactly once for the whole session, not once per batch"
        );
    }

    #[test]
    fn parquet_streaming_yields_all_rows_and_opens_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.parquet");
        write_parquet(&path, 730);

        OPEN_CALLS.with(|c| c.set(0));
        let mut batcher = StreamingBatcher::new(path.to_str().unwrap(), 100).unwrap();
        assert!(
            batcher.is_bounded_memory(),
            "Parquet must use the incremental, bounded-memory path"
        );

        let mut total = 0usize;
        let mut n_batches = 0usize;
        while let Some(batch) = batcher.next_batch().unwrap() {
            total += batch.height();
            n_batches += 1;
        }
        assert_eq!(total, 730);
        assert!(n_batches > 1, "expected multiple parquet batches");
        assert_eq!(
            OPEN_CALLS.with(|c| c.get()),
            1,
            "parquet file must be opened exactly once regardless of batch count"
        );
    }

    #[test]
    fn non_incremental_formats_are_flagged_and_still_read_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        std::fs::write(&path, r#"[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]"#).unwrap();

        OPEN_CALLS.with(|c| c.set(0));
        let mut batcher = StreamingBatcher::new(path.to_str().unwrap(), 2).unwrap();
        assert!(
            !batcher.is_bounded_memory(),
            "plain JSON has no incremental reader; it should be flagged as materialized"
        );

        let mut total = 0usize;
        while let Some(batch) = batcher.next_batch().unwrap() {
            total += batch.height();
        }
        assert_eq!(total, 5);
        // Even the materialized fallback must read the source only once for
        // the whole batching session — the original bug re-read on every
        // single batch regardless of format.
        assert_eq!(
            OPEN_CALLS.with(|c| c.get()),
            1,
            "fallback formats must still read the source file only once, not once per batch"
        );
    }

    #[test]
    fn empty_batcher_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.csv");
        write_csv(&path, 0);

        let mut batcher = StreamingBatcher::new(path.to_str().unwrap(), 50).unwrap();
        assert_eq!(batcher.next_batch().unwrap(), None);
    }
}
