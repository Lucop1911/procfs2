use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::Bytes;

/// I/O statistics for a process.
///
/// Sourced from `/proc/PID/io`. Counters are cumulative since the
/// process started.
///
/// # rchar/wchar vs read_bytes/write_bytes
///
/// `rchar` and `wchar` count bytes passed to `read()`/`write()`
/// syscalls, including data served from the page cache.
/// `read_bytes` and `write_bytes` count bytes actually transferred
/// to/from the underlying storage layer. The difference between the
/// two pairs reveals page cache hit/miss ratios.
#[derive(Debug)]
pub struct ProcessIo {
    /// Bytes read via `read()` syscalls (including page cache hits).
    pub rchar: Bytes,
    /// Bytes written via `write()` syscalls (including page cache).
    pub wchar: Bytes,
    /// Number of `read()` syscalls issued.
    pub syscr: u64,
    /// Number of `write()` syscalls issued.
    pub syscw: u64,
    /// Bytes actually read from the storage layer.
    pub read_bytes: Bytes,
    /// Bytes actually written to the storage layer.
    pub write_bytes: Bytes,
    /// Bytes of write I/O that were cancelled (e.g. truncated pages).
    pub cancelled_write_bytes: Bytes,
}

impl ProcessIo {
    /// Parses `/proc/PID/io` from raw bytes.
    ///
    /// The file uses a `Key:\tValue` format with one entry per line.
    /// Missing fields default to zero.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut rchar = None;
        let mut wchar = None;
        let mut syscr = None;
        let mut syscw = None;
        let mut read_bytes = None;
        let mut write_bytes = None;
        let mut cancelled_write_bytes = None;

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            let (key, value) = match parse::parse_key_value_line(line) {
                Some(kv) => kv,
                None => continue,
            };

            let val = parse::parse_dec_u64(parse::trim_start(value)).map_err(|_| Error::Parse {
                path: std::path::PathBuf::from("<io>"),
                line: 0,
                msg: "invalid number",
            })?;

            match key {
                b"rchar" => rchar = Some(Bytes(val)),
                b"wchar" => wchar = Some(Bytes(val)),
                b"syscr" => syscr = Some(val),
                b"syscw" => syscw = Some(val),
                b"read_bytes" => read_bytes = Some(Bytes(val)),
                b"write_bytes" => write_bytes = Some(Bytes(val)),
                b"cancelled_write_bytes" => cancelled_write_bytes = Some(Bytes(val)),
                _ => {}
            }
        }

        Ok(ProcessIo {
            rchar: rchar.unwrap_or(Bytes(0)),
            wchar: wchar.unwrap_or(Bytes(0)),
            syscr: syscr.unwrap_or(0),
            syscw: syscw.unwrap_or(0),
            read_bytes: read_bytes.unwrap_or(Bytes(0)),
            write_bytes: write_bytes.unwrap_or(Bytes(0)),
            cancelled_write_bytes: cancelled_write_bytes.unwrap_or(Bytes(0)),
        })
    }
}
