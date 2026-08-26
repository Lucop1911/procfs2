use crate::error::{Error, Result};

/// Reads an entire file into a `Vec<u8>`.
///
/// This is the primary I/O entry point for all `/proc` and `/sys` reads.
/// Errors are wrapped into [`Error::Io`].
pub fn read_file(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(Error::Io)
}

/// Trait for types that can be parsed from a raw byte buffer.
///
/// Implementors parse directly from `&[u8]` to avoid UTF-8 conversion
/// overhead for fields that are purely numeric.
pub trait ParseFromBytes: Sized {
    fn parse_from_bytes(bytes: &[u8]) -> Result<Self>;
}

/// Splits a byte slice at the first occurrence of `byte`.
///
/// Returns `(before, after)` where `after` starts past the delimiter.
/// If the byte is not found, returns `(slice, &[])`.
pub fn split_at_byte(slice: &[u8], byte: u8) -> (&[u8], &[u8]) {
    match memchr(byte, slice) {
        Some(idx) => (&slice[..idx], &slice[idx + 1..]),
        None => (slice, &[]),
    }
}

/// Finds the first occurrence of `byte` in `slice`.
///
/// Equivalent to `slice.iter().position(|&b| b == byte)` but named
/// consistently with the C library function.
pub fn memchr(byte: u8, slice: &[u8]) -> Option<usize> {
    slice.iter().position(|&b| b == byte)
}

/// Trims trailing whitespace: space, tab, newline, carriage return.
pub fn trim_end(slice: &[u8]) -> &[u8] {
    let end = slice
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r');
    match end {
        Some(idx) => &slice[..=idx],
        None => &[],
    }
}

/// Trims leading whitespace: space, tab, newline, carriage return.
pub fn trim_start(slice: &[u8]) -> &[u8] {
    let start = slice
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r');
    match start {
        Some(idx) => &slice[idx..],
        None => &[],
    }
}

/// Trims both leading and trailing whitespace.
pub fn trim(slice: &[u8]) -> &[u8] {
    trim_start(trim_end(slice))
}

/// Parses a `Key: Value` line, returning the key and value as subslices.
///
/// The key is everything before the first `:`, trimmed on the right.
/// The value is everything after the `:`, trimmed on the right.
/// Returns `None` if no colon is found.
pub fn parse_key_value_line(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let idx = memchr(b':', line)?;
    let key = trim_end(&line[..idx]);
    let value = trim_end(&line[idx + 1..]);
    Some((key, value))
}

/// Parses a hexadecimal integer from a byte slice.
///
/// Accepts optional `0x` prefix. Trailing whitespace is ignored.
pub fn parse_hex_u64(s: &[u8]) -> Result<u64> {
    let s = trim_end(s);
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<hex>"),
            line: 0,
            msg: "empty hex value",
        });
    }
    let s = if s.starts_with(b"0x") { &s[2..] } else { s };
    u64::from_str_radix(
        std::str::from_utf8(s).map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<hex>"),
            line: 0,
            msg: "invalid utf8 in hex",
        })?,
        16,
    )
    .map_err(|_| Error::Parse {
        path: std::path::PathBuf::from("<hex>"),
        line: 0,
        msg: "invalid hex",
    })
}

/// Parses a decimal `u64` from a byte slice.
pub fn parse_dec_u64(s: &[u8]) -> Result<u64> {
    let s = trim_end(s);
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "empty decimal value",
        });
    }
    std::str::from_utf8(s)
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "invalid utf8 in decimal",
        })?
        .parse::<u64>()
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "invalid decimal",
        })
}

/// Parses a decimal `u32` from a byte slice.
pub fn parse_dec_u32(s: &[u8]) -> Result<u32> {
    parse_dec_u64(s).map(|v| v as u32)
}

/// Parses a decimal `i64` from a byte slice.
///
/// Used for fields like `/proc/PID/stat`'s `cutime` and `cstime`
/// which can be negative on some kernels.
pub fn parse_dec_i64(s: &[u8]) -> Result<i64> {
    let s = trim_end(s);
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "empty decimal value",
        });
    }
    std::str::from_utf8(s)
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "invalid utf8 in decimal",
        })?
        .parse::<i64>()
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "invalid decimal",
        })
}

/// Parses a decimal `f32` from a byte slice.
pub fn parse_dec_f32(s: &[u8]) -> Result<f32> {
    let s = trim_end(s);
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "empty float value",
        });
    }
    std::str::from_utf8(s)
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "invalid utf8 in float",
        })?
        .parse::<f32>()
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "invalid float",
        })
}

/// Parses a decimal `f64` from a byte slice.
///
/// Mostly used where sub-second precision matters.
pub fn parse_dec_f64(s: &[u8]) -> Result<f64> {
    let s = trim_end(s);
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "empty float value",
        });
    }
    std::str::from_utf8(s)
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "invalid utf8 in float",
        })?
        .parse::<f64>()
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<float>"),
            line: 0,
            msg: "invalid float",
        })
}

/// Splits a byte slice on runs of spaces and tabs.
///
/// Unlike `split(|&b| b == b' ' || b == b'\t')`, this does not
/// produce empty segments for consecutive whitespace.
pub fn split_spaces(slice: &[u8]) -> Vec<&[u8]> {
    slice
        .split(|&b| b == b' ' || b == b'\t')
        .filter(|f| !f.is_empty())
        .collect()
}

/// An allocation-free split of a byte slice into whitespace-separated fields.
///
/// Fields are stored in a fixed-size stack buffer rather than a `Vec`, so
/// splitting inside per-line loops does not pay a heap allocation for every
/// line. If a line contains more than `N` fields, the excess fields are
/// ignored; no `/proc` line exceeds `N` in practice.
///
/// Splitting semantics match [`split_spaces`]: runs of spaces and tabs are
/// collapsed and empty segments are skipped.
pub struct SplitFields<'a, const N: usize> {
    fields: [&'a [u8]; N],
    len: usize,
}

impl<'a, const N: usize> SplitFields<'a, N> {
    /// Splits `slice` on runs of spaces and tabs.
    pub fn new(slice: &'a [u8]) -> Self {
        const EMPTY: &[u8] = &[];
        let mut fields = [EMPTY; N];
        let mut len = 0;
        for field in slice.split(|&b| b == b' ' || b == b'\t') {
            if field.is_empty() {
                continue;
            }
            if len >= N {
                break;
            }
            fields[len] = field;
            len += 1;
        }
        SplitFields { fields, len }
    }
}

impl<'a, const N: usize> std::ops::Deref for SplitFields<'a, N> {
    type Target = [&'a [u8]];

    fn deref(&self) -> &Self::Target {
        &self.fields[..self.len]
    }
}

/// Returns the first whitespace-delimited token of `slice`.
///
/// Used to strip unit suffixes (e.g. the ` kB` in `9764412 kB`)
/// from key-value fields in `/proc/meminfo` and `/proc/PID/status`
/// before parsing the numeric part.
pub fn first_token(slice: &[u8]) -> &[u8] {
    match memchr(b' ', slice).or_else(|| memchr(b'\t', slice)) {
        Some(idx) => &slice[..idx],
        None => slice,
    }
}

/// Splits a byte slice on newlines, filtering out empty trailing lines.
pub fn split_lines(slice: &[u8]) -> Vec<&[u8]> {
    slice
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .collect()
}
