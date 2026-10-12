use std::io::{self, BufRead};

/// One raw NDJSON line read from a source, whether or not it is valid.
///
/// `record` is a 0-based index over *non-blank* lines only. `content` is only
/// populated when the line is both valid UTF-8 and within `max_record_bytes`;
/// otherwise `utf8` is `false` or `exceeded` is `true` and `content` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLine {
    pub record: u64,
    pub bytes: u64,
    pub exceeded: bool,
    pub utf8: bool,
    pub content: String,
}

/// Incremental, bounded-memory NDJSON reader over any [`BufRead`].
///
/// Blank/whitespace-only lines are skipped and do not advance the record
/// index. Each line is read with a hard byte cap: once the cap is reached the
/// rest of the line is consumed and discarded without being buffered, so a
/// hostile multi-GB line cannot exhaust memory.
pub struct NdjsonSource<R: BufRead> {
    reader: R,
    max_record_bytes: usize,
    record_index: u64,
    bytes_consumed: u64,
}

impl<R: BufRead> NdjsonSource<R> {
    pub fn new(reader: R, max_record_bytes: usize) -> Self {
        Self {
            reader,
            max_record_bytes: max_record_bytes.max(1),
            record_index: 0,
            bytes_consumed: 0,
        }
    }

    /// Total source bytes consumed so far (including skipped blank lines and
    /// the discarded remainder of oversized lines).
    pub fn bytes_consumed(&self) -> u64 {
        self.bytes_consumed
    }

    /// Number of non-blank lines emitted so far.
    pub fn records_read(&self) -> u64 {
        self.record_index
    }

    /// Reads the next non-blank line, or `None` at end of input.
    pub fn next_line(&mut self) -> io::Result<Option<RawLine>> {
        let mut buf: Vec<u8> = Vec::new();
        loop {
            let read = read_line_capped(&mut self.reader, self.max_record_bytes, &mut buf)?;
            let (bytes, exceeded) = match read {
                None => return Ok(None),
                Some(v) => v,
            };
            self.bytes_consumed += bytes;

            if exceeded {
                let record = self.record_index;
                self.record_index += 1;
                return Ok(Some(RawLine {
                    record,
                    bytes,
                    exceeded: true,
                    utf8: false,
                    content: String::new(),
                }));
            }

            let decoded = std::str::from_utf8(&buf);
            match decoded {
                Err(_) => {
                    let record = self.record_index;
                    self.record_index += 1;
                    return Ok(Some(RawLine {
                        record,
                        bytes,
                        exceeded: false,
                        utf8: false,
                        content: String::new(),
                    }));
                }
                Ok(s) => {
                    // Trim the trailing newline / CR for parsing and blank
                    // detection.
                    let trimmed = s.trim_end_matches(['\n', '\r']);
                    if trimmed.trim().is_empty() {
                        // Blank line: skip, do not advance the record index.
                        continue;
                    }
                    let record = self.record_index;
                    self.record_index += 1;
                    return Ok(Some(RawLine {
                        record,
                        bytes,
                        exceeded: false,
                        utf8: true,
                        content: trimmed.to_string(),
                    }));
                }
            }
        }
    }
}

/// Reads one line into `out`, capped at `cap` bytes.
///
/// Returns `Ok(None)` at end of input with no bytes read, otherwise
/// `Ok(Some((bytes_read, exceeded)))`. When `exceeded` is true, `out` holds at
/// most the first `cap` bytes and the remainder of the line has been consumed.
fn read_line_capped<R: BufRead>(
    reader: &mut R,
    cap: usize,
    out: &mut Vec<u8>,
) -> io::Result<Option<(u64, bool)>> {
    out.clear();
    let mut total: u64 = 0;
    let mut exceeded = false;

    loop {
        let available = match reader.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };

        if available.is_empty() {
            // EOF. If we have partial data it is a valid final line.
            if total == 0 {
                return Ok(None);
            }
            return Ok(Some((total, exceeded)));
        }

        let newline = available.iter().position(|&b| b == b'\n');
        let used = match newline {
            Some(i) => i + 1,
            None => available.len(),
        };

        if !exceeded {
            let remaining = cap.saturating_sub(out.len());
            let copy = used.min(remaining);
            out.extend_from_slice(&available[..copy]);
            if copy < used {
                exceeded = true;
            }
        }

        total += used as u64;
        reader.consume(used);

        if newline.is_some() {
            return Ok(Some((total, exceeded)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn collect(input: &str, cap: usize) -> Vec<RawLine> {
        let mut src = NdjsonSource::new(Cursor::new(input.as_bytes().to_vec()), cap);
        let mut out = Vec::new();
        while let Some(line) = src.next_line().unwrap() {
            out.push(line);
        }
        out
    }

    #[test]
    fn reads_records_and_skips_blank_lines() {
        let lines = collect("{\"a\":1}\n\n   \n{\"a\":2}\n", 1024);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].record, 0);
        assert_eq!(lines[0].content, "{\"a\":1}");
        assert_eq!(lines[1].record, 1);
    }

    #[test]
    fn handles_missing_trailing_newline() {
        let lines = collect("{\"a\":1}\n{\"a\":2}", 1024);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].content, "{\"a\":2}");
    }

    #[test]
    fn oversized_line_is_flagged_and_discarded() {
        let big = format!("{{\"a\":\"{}\"}}", "x".repeat(5000));
        let lines = collect(&format!("{big}\n{{\"a\":1}}\n"), 64);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].exceeded);
        assert!(lines[0].content.is_empty());
        assert_eq!(lines[1].content, "{\"a\":1}");
    }

    #[test]
    fn invalid_utf8_is_flagged() {
        let mut bytes = vec![b'{', 0xff, 0xfe, b'}', b'\n'];
        let mut src = NdjsonSource::new(Cursor::new(std::mem::take(&mut bytes)), 1024);
        let line = src.next_line().unwrap().unwrap();
        assert!(!line.utf8);
        assert!(!line.exceeded);
    }

    #[test]
    fn unicode_content_is_preserved() {
        let lines = collect("{\"name\":\"héllo 世界 🚀\"}\n", 1024);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].content.contains("世界"));
    }

    #[test]
    fn bytes_consumed_counts_discarded_and_blank() {
        let input = "\n{\"a\":1}\n";
        let mut src = NdjsonSource::new(Cursor::new(input.as_bytes().to_vec()), 1024);
        src.next_line().unwrap();
        assert_eq!(src.bytes_consumed(), input.len() as u64);
    }
}
