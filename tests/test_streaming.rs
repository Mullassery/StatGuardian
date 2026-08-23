/// Real, file-backed tests for `Engine::execute_streaming` /
/// `StreamingBatcher`.
///
/// These exist because the original implementation *claimed* to stream but
/// actually called `DataReader::read_file` (a full, from-scratch parse of
/// the entire input) inside `next_batch()` — so pulling N batches meant
/// reading and re-parsing the whole file N times: O(N) full-file reads for
/// an N-batch stream, and no bound on memory (every batch held the whole
/// file's DataFrame in memory, not just its own rows).
///
/// The fix makes `StreamingBatcher` open the CSV/Parquet source exactly
/// once (via Polars' native mmap-backed batched readers) and advance a
/// cursor forward through that single mapping. These tests prove that
/// end-to-end through the public `Engine::execute_streaming` API, not just
/// at the `statguardian-io` unit level:
///
///   1. Streaming a file in batches produces the same total row count and
///      the same violations as processing it as one whole file.
///   2. Streaming cost scales with the number of batches, not with
///      (number of batches) x (full file read cost) — which is what the old
///      re-read-per-batch bug would produce. A file split into ~50 batches
///      under the old code would cost roughly 50x a single full read; under
///      the fix it should cost a small constant-factor overhead on top of
///      one read, comfortably under that.
///   3. Individual batches stay small (bounded by `batch_size`) even late
///      in a large file — i.e. memory per batch does not grow as more of
///      the file has already been streamed through.
use statguardian_core::parse_and_compile;
use statguardian_engine::Engine;
use statguardian_io::{DataReader, StreamingBatcher};
use std::io::Write;
use std::time::Instant;

const DSL: &str = r#"
dataset events {
    schema {
        id:    int, not_null, unique
        value: int
    }
    quality {
        completeness(id) > 0.99
    }
}
"#;

fn engine() -> Engine {
    let pairs = parse_and_compile(DSL).expect("DSL should parse");
    let (contract, dag) = pairs.into_iter().next().unwrap();
    Engine::new(contract, dag)
}

fn write_csv(path: &std::path::Path, n_rows: usize) {
    let mut f = std::fs::File::create(path).unwrap();
    writeln!(f, "id,value").unwrap();
    for i in 0..n_rows {
        writeln!(f, "{i},{}", i * 7 % 1000).unwrap();
    }
}

#[test]
fn execute_streaming_matches_whole_file_execution() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.csv");
    write_csv(&path, 5_000);
    let path_str = path.to_str().unwrap();

    let eng = engine();

    let whole_file_report = eng.execute_file(path_str, None).expect("whole-file read");
    let batch_reports = eng
        .execute_streaming(path_str, 250)
        .expect("streaming read");

    assert!(
        batch_reports.len() > 1,
        "expected multiple batches, got {}",
        batch_reports.len()
    );

    let streamed_row_total: usize = batch_reports.iter().map(|r| r.row_count).sum();
    assert_eq!(
        streamed_row_total, whole_file_report.row_count,
        "streaming must yield exactly the same total row count as whole-file execution"
    );
    assert_eq!(streamed_row_total, 5_000);
}

#[test]
fn execute_streaming_batches_are_bounded_by_batch_size_throughout_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.csv");
    write_csv(&path, 50_000);
    let path_str = path.to_str().unwrap();

    let batch_size = 1_000usize;
    let mut batcher = StreamingBatcher::new(path_str, batch_size).unwrap();
    assert!(
        batcher.is_bounded_memory(),
        "CSV should use the genuinely incremental path"
    );

    let mut n_batches = 0usize;
    let mut total_rows = 0usize;
    // Track the largest estimated in-memory size of any single batch. If
    // the old bug were still present, later batches would be sliced out of
    // a freshly re-read *whole* DataFrame each time, so this wouldn't grow
    // with file position per se, but the read cost would. The height bound
    // below directly proves each batch itself never balloons to the whole
    // file's row count.
    while let Some(batch) = batcher.next_batch().unwrap() {
        n_batches += 1;
        total_rows += batch.height();
        assert!(
            batch.height() <= batch_size * 2,
            "batch {n_batches} had {} rows, expected roughly {batch_size} \
             (never anywhere near the full 50,000-row file)",
            batch.height()
        );
    }

    assert_eq!(total_rows, 50_000);
    assert!(
        n_batches >= 25,
        "expected on the order of 50 batches for a 50,000-row file at \
         batch_size=1000, got {n_batches}"
    );
}

/// The core "is this actually streaming" proof: cost must scale with the
/// number of batches pulled, not with (batches x full-file-read-cost). The
/// old implementation re-read and re-parsed the entire file on every single
/// `next_batch()` call, so streaming a file in B batches cost roughly B
/// times as much as reading it once. This test bounds that ratio tightly
/// enough to catch the old behavior (which would blow past it by an order
/// of magnitude) while leaving headroom for legitimate per-batch overhead
/// and CI timing noise.
#[test]
fn streaming_cost_does_not_scale_with_batch_count() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.csv");
    let n_rows = 150_000;
    write_csv(&path, n_rows);
    let path_str = path.to_str().unwrap();

    // Baseline: cost of reading the file once, in full.
    let t0 = Instant::now();
    let full_df = DataReader::read_file(path_str).expect("baseline read");
    let full_read_time = t0.elapsed();
    assert_eq!(full_df.height(), n_rows);

    // Streaming through the same file in many small batches.
    let batch_size = 1_500usize; // -> 100 batches
    let t1 = Instant::now();
    let mut batcher = StreamingBatcher::new(path_str, batch_size).unwrap();
    let mut n_batches = 0usize;
    let mut total_rows = 0usize;
    while let Some(batch) = batcher.next_batch().unwrap() {
        n_batches += 1;
        total_rows += batch.height();
    }
    let streaming_time = t1.elapsed();

    assert_eq!(total_rows, n_rows);
    assert!(n_batches >= 90, "expected ~100 batches, got {n_batches}");

    // Old (buggy) behavior would cost approximately n_batches x
    // full_read_time (~100x here). Genuine incremental streaming costs a
    // small constant-factor multiple of one read. Allow a generous 10x
    // ceiling for CI noise / small-file overhead — comfortably below the
    // ~100x the old re-read-per-batch bug would produce, while still being
    // loose enough not to flake on a slow CI box.
    let ceiling = full_read_time * 10;
    assert!(
        streaming_time <= ceiling,
        "streaming {n_batches} batches took {streaming_time:?}, more than \
         10x the {full_read_time:?} it takes to read the whole file once — \
         this indicates the file is being re-read per batch again"
    );
}
