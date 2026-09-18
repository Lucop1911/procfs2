use crate::error::Result;
use crate::util::parse::{SplitFields, parse_dec_u64};

/// Memory usage of a process in pages.
///
/// Parsed from `/proc/PID/statm`. A single line of seven
/// space-separated page counts, in the order `size resident
/// shared text lib data dt`. All values are in pages, not bytes;
/// multiply by the page size (`sysconf(_SC_PAGESIZE)`) to
/// convert.
///
/// The four fields sum to `size` on Linux 4.0 and later: code,
/// data, stack, and shared pages. On older kernels `resident`
/// reported the page counts of shared and text pages instead of
/// the resident set, so don't read too much into it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statm {
    /// Total program size in pages.
    ///
    /// Always reported, even for kernel threads. Matches the
    /// `VmSize` field of `/proc/PID/status`, scaled by the page
    /// size.
    pub size: u64,
    /// Number of resident pages.
    ///
    /// The kernel has reported the true resident set size since
    /// version 4.0; before that this field accumulated shared and
    /// text pages instead. Matches `VmRSS` in `/proc/PID/status`.
    pub resident: u64,
    /// Number of pages backed by a file, and thus shared between
    /// processes mapping the same object.
    ///
    /// Anonymous memory (heap, stack, `mmap` without a file) is
    /// not counted here.
    pub shared: u64,
    /// Number of pages holding executable code.
    pub text: u64,
    /// Number of pages holding shared libraries.
    ///
    /// Always zero: since Linux 2.6 these are counted in `text`
    /// instead.
    pub lib: u64,
    /// Number of pages holding data and stack.
    pub data: u64,
    /// Number of dirty pages.
    ///
    /// Always zero on current kernels.
    pub dt: u64,
}

impl Statm {
    /// Parses a `/proc/PID/statm` line from raw bytes.
    ///
    /// Expects exactly seven space-separated decimal page counts.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let fields = SplitFields::<7>::new(bytes);

        if fields.len() < 7 {
            return Err(crate::error::Error::Parse {
                path: std::path::PathBuf::from("<statm>"),
                line: 1,
                msg: "expected 7 fields in statm",
            });
        }

        Ok(Statm {
            size: parse_dec_u64(fields[0])?,
            resident: parse_dec_u64(fields[1])?,
            shared: parse_dec_u64(fields[2])?,
            text: parse_dec_u64(fields[3])?,
            lib: parse_dec_u64(fields[4])?,
            data: parse_dec_u64(fields[5])?,
            dt: parse_dec_u64(fields[6])?,
        })
    }
}
