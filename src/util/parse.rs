use crate::error::{Error, Result};
use memchr::{memchr as memchr_simd, memchr2};

/// Reads an entire file into a `Vec<u8>`.
///
/// This is the primary I/O entry point for all `/proc` and `/sys` reads.
/// Errors are wrapped into [`Error::Io`].
pub fn read_file(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|e| Error::Io {
        path: Some(path.to_path_buf()),
        error: e,
    })
}

/// Trait for types that can be parsed from a raw byte buffer.
///
/// Implementors parse directly from `&[u8]` to avoid UTF-8 conversion
/// overhead for fields that are purely numeric.
pub trait ParseFromBytes: Sized {
    /// Parses a value from a raw byte buffer.
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
#[inline]
pub fn memchr(byte: u8, slice: &[u8]) -> Option<usize> {
    memchr_simd(byte, slice)
}

/// Counts the occurrences of `byte` in `slice`.
///
/// Used to size result buffers before splitting a file's contents line
/// by line, avoiding repeated reallocation while pushing rows.
#[inline]
pub(crate) fn count_byte(byte: u8, slice: &[u8]) -> usize {
    memchr::memchr_iter(byte, slice).count()
}

/// Trims trailing whitespace: space, tab, newline, carriage return.
#[inline]
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
#[inline]
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
#[inline]
pub fn trim(slice: &[u8]) -> &[u8] {
    trim_start(trim_end(slice))
}

/// Parses a `Key: Value` line, returning the key and value as subslices.
///
/// The key is everything before the first `:`, trimmed on the right.
/// The value is everything after the `:`, trimmed on the right.
/// Returns `None` if no colon is found.
#[inline]
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
    if s.is_empty() {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<hex>"),
            line: 0,
            msg: "empty hex value",
        });
    }

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
        .parse::<u32>()
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<dec>"),
            line: 0,
            msg: "invalid decimal",
        })
}

/// Fast hexadecimal integer parser over raw bytes.
///
/// Used by the `/proc/net` parsers where every field of every line is
/// numeric. Unlike [`parse_hex_u64`] there is no UTF-8 round-trip or
/// validation, and the result is truncated to the caller's target width
/// with a plain cast. Returns `0` for empty or non-hex input rather than
/// erroring, matching the `unwrap_or(0)` handling the net parsers use.
#[inline]
pub(crate) fn parse_hex_fast(s: &[u8]) -> u64 {
    let mut value = 0;
    for &byte in s {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return 0,
        };
        value = (value << 4) | digit as u64;
    }
    value
}

/// Fast decimal integer parser over raw bytes.
///
/// The decimal counterpart of [`parse_hex_fast`]. Skips UTF-8 conversion
/// and returns `0` for empty or non-digit input.
#[inline]
pub(crate) fn parse_dec_fast(s: &[u8]) -> u64 {
    let mut value = 0;
    for &byte in s {
        if !byte.is_ascii_digit() {
            return 0;
        }
        value = value * 10 + (byte - b'0') as u64;
    }
    value
}

/// Strict hexadecimal parser over raw bytes.
///
/// Single-pass like [`parse_hex_fast`] but keeps the contract of
/// [`parse_hex_u64`]: empty or non-hex input yields an error instead of
/// silently decoding to `0`. Overflow wraps silently, which kernel
/// fields never approach.
#[inline]
pub(crate) fn parse_hex_u64_fast(s: &[u8]) -> Result<u64> {
    let err = || Error::Parse {
        path: std::path::PathBuf::from("<hex>"),
        line: 0,
        msg: "invalid hex value",
    };
    if s.is_empty() {
        return Err(err());
    }
    let mut value = 0u64;
    for &byte in s {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => return Err(err()),
        };
        value = (value << 4) | digit as u64;
    }
    Ok(value)
}

/// Strict decimal parser over raw bytes.
///
/// Single-pass like [`parse_dec_fast`] but keeps the contract of
/// [`parse_dec_u64`]: empty or non-digit input yields an error instead
/// of silently decoding to `0`. Overflow wraps silently.
#[inline]
pub(crate) fn parse_dec_u64_fast(s: &[u8]) -> Result<u64> {
    let err = || Error::Parse {
        path: std::path::PathBuf::from("<dec>"),
        line: 0,
        msg: "invalid decimal value",
    };
    if s.is_empty() {
        return Err(err());
    }
    let mut value = 0u64;
    for &byte in s {
        if !byte.is_ascii_digit() {
            return Err(err());
        }
        value = value * 10 + (byte - b'0') as u64;
    }
    Ok(value)
}

/// Strict decimal `u32` parser over raw bytes.
#[inline]
pub(crate) fn parse_dec_u32_fast(s: &[u8]) -> Result<u32> {
    parse_dec_u64_fast(s).map(|v| v as u32)
}

/// Strict decimal `i64` parser over raw bytes.
///
/// Like [`parse_dec_u64_fast`] but accepts an optional leading `-`,
/// mirroring [`parse_dec_i64`]. Overflow wraps silently.
#[inline]
pub(crate) fn parse_dec_i64_fast(s: &[u8]) -> Result<i64> {
    let err = || Error::Parse {
        path: std::path::PathBuf::from("<dec>"),
        line: 0,
        msg: "invalid decimal value",
    };
    let (rest, negative) = match s.first() {
        Some(b'-') => (&s[1..], true),
        _ => (s, false),
    };
    if rest.is_empty() {
        return Err(err());
    }
    let mut value = 0u64;
    for &byte in rest {
        if !byte.is_ascii_digit() {
            return Err(err());
        }
        value = value * 10 + (byte - b'0') as u64;
    }
    Ok(if negative {
        (value as i64).wrapping_neg()
    } else {
        value as i64
    })
}

/// Decodes a little-endian hex IPv4 address into bytes.
///
/// The kernel writes the address byte-for-byte reversed (`0100000A` is
/// `10.0.0.1`), so the nibbles are read back-to-front. The result feeds
/// straight into `Ipv4Addr::from`. The caller is expected to have already
/// checked that the slice is exactly 8 hex digits.
#[inline]
pub(crate) fn decode_ipv4_fast(s: &[u8]) -> [u8; 4] {
    [
        hex_byte(s[6], s[7]),
        hex_byte(s[4], s[5]),
        hex_byte(s[2], s[3]),
        hex_byte(s[0], s[1]),
    ]
}

/// Decodes a little-endian hex IPv6 address into bytes.
///
/// The kernel stores the 128-bit address as four little-endian 32-bit
/// words, each written byte-reversed like the IPv4 case. The result feeds
/// straight into `Ipv6Addr::from`. The caller is expected to have already
/// checked that the slice is exactly 32 hex digits.
#[inline]
pub(crate) fn decode_ipv6_fast(s: &[u8]) -> [u8; 16] {
    let mut out = [0; 16];
    let mut i = 0;
    while i < 4 {
        let offset = i * 8;
        out[i * 4] = hex_byte(s[offset + 6], s[offset + 7]);
        out[i * 4 + 1] = hex_byte(s[offset + 4], s[offset + 5]);
        out[i * 4 + 2] = hex_byte(s[offset + 2], s[offset + 3]);
        out[i * 4 + 3] = hex_byte(s[offset], s[offset + 1]);
        i += 1;
    }
    out
}

/// Combines two hex nibbles into a byte.
#[inline]
fn hex_byte(high: u8, low: u8) -> u8 {
    (hex_nibble(high) << 4) | hex_nibble(low)
}

/// Tests a byte against the ASCII hex alphabet and returns its value.
///
/// Branchless: the `(b | 0x20)` lowercases the letter so one comparison
/// covers both cases, and `& 0x0f` extracts the low nibble for `0-9`.
#[inline]
fn hex_nibble(byte: u8) -> u8 {
    (byte & 0x0f) + 9 * ((byte | 0x20) > b'9') as u8
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

/// Splits a byte slice on runs of spaces and tabs, returning the
/// fields in a heap-allocated `Vec`.
///
/// Unlike `split(|&b| b == b' ' || b == b'\t')`, this does not
/// produce empty segments for consecutive whitespace.
///
/// Prefer the allocation-free [`SplitFields`] when the number of
/// fields is bounded: most `/proc` writers emit a fixed column count,
/// and `SplitFields` keeps the fields in a stack buffer rather than
/// paying a heap allocation for every line. Reserve `split_spaces`
/// for rows whose field count genuinely varies with the system
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
    ///
    /// Uses `memchr2` to skip straight to the next delimiter instead of
    /// scanning byte-by-byte, which keeps the split cheap on rows from
    /// large files like `/proc/net/unix`.
    #[inline]
    pub fn new(slice: &'a [u8]) -> Self {
        const EMPTY: &[u8] = &[];
        let mut fields = [EMPTY; N];
        let mut len = 0;
        let mut rest = slice;
        while len < N {
            match memchr2(b' ', b'\t', rest) {
                Some(0) => rest = &rest[1..],
                Some(idx) => {
                    fields[len] = &rest[..idx];
                    len += 1;
                    rest = &rest[idx + 1..];
                }
                None => {
                    if !rest.is_empty() {
                        fields[len] = rest;
                        len += 1;
                    }
                    break;
                }
            }
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
#[inline]
pub fn first_token(slice: &[u8]) -> &[u8] {
    match memchr2(b' ', b'\t', slice) {
        Some(idx) => &slice[..idx],
        None => slice,
    }
}
