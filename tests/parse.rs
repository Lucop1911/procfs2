//! Unit tests for the byte-slice parsers in `util::parse`.
//!
//! Unlike `tests/integration.rs`, these run against fixed in-memory
//! buffers so edge cases (empty input, bad digits, overflow boundaries)
//! are exercised deterministically without depending on the kernel.

use procfs2::util::parse::{
    SplitFields, first_token, memchr, parse_dec_f32, parse_dec_f64, parse_dec_i64, parse_dec_u32,
    parse_dec_u64, parse_hex_u64, parse_key_value_line, split_at_byte, split_spaces, trim,
    trim_end, trim_start,
};

#[test]
fn parse_dec_u64_basic() {
    assert_eq!(parse_dec_u64(b"42"), Ok(42));
    assert_eq!(parse_dec_u64(b"18446744073709551615"), Ok(u64::MAX));
}

#[test]
fn parse_dec_u64_trim_end() {
    // Trailing whitespace is stripped.
    assert_eq!(parse_dec_u64(b"42\n"), Ok(42));
    assert_eq!(parse_dec_u64(b"42 \t\r\n"), Ok(42));
}

#[test]
fn parse_dec_u64_errors() {
    // Overflow, garbage, and empty input all error.
    assert!(parse_dec_u64(b"18446744073709551616").is_err());
    assert!(parse_dec_u64(b"12x").is_err());
    assert!(parse_dec_u64(b"").is_err());
    assert!(parse_dec_u64(b" ").is_err());
}

#[test]
fn parse_dec_u64_leading_digits() {
    assert_eq!(parse_dec_u64(b"007"), Ok(7));
}

#[test]
fn parse_dec_u32_basic() {
    assert_eq!(parse_dec_u32(b"1234"), Ok(1234));
    assert_eq!(parse_dec_u32(b"4294967295"), Ok(u32::MAX));
}

#[test]
fn parse_dec_u32_errors() {
    assert!(parse_dec_u32(b"-1").is_err());
    assert!(parse_dec_u32(b"4294967296").is_err());
    assert!(parse_dec_u32(b"abc").is_err());
}

#[test]
fn parse_dec_i64_signed() {
    assert_eq!(parse_dec_i64(b"123"), Ok(123));
    assert_eq!(parse_dec_i64(b"-123"), Ok(-123));
    assert_eq!(parse_dec_i64(b"-0"), Ok(0));
}

#[test]
fn parse_dec_i64_overflow() {
    assert!(parse_dec_i64(b"9223372036854775808").is_err());
    assert!(parse_dec_i64(b"-").is_err());
    assert!(parse_dec_i64(b"12abc").is_err());
}

#[test]
fn parse_dec_i64_leading_plus() {
    // std accepts a leading '+'; so does parse_dec_i64.
    assert_eq!(parse_dec_i64(b"+5"), Ok(5));
}

#[test]
fn parse_dec_f32_f64() {
    assert_eq!(parse_dec_f32(b"1.5"), Ok(1.5));
    assert_eq!(parse_dec_f64(b"2.25"), Ok(2.25));
    assert!(parse_dec_f32(b"").is_err());
    assert!(parse_dec_f64(b"\n").is_err());
}

#[test]
fn parse_hex_u64_basic() {
    assert_eq!(parse_hex_u64(b"ff"), Ok(255));
    assert_eq!(parse_hex_u64(b"0x10"), Ok(16));
    assert_eq!(parse_hex_u64(b"FF"), Ok(255));
}

#[test]
fn parse_hex_u64_errors() {
    assert!(parse_hex_u64(b"0x").is_err());
    assert!(parse_hex_u64(b"").is_err());
    assert!(parse_hex_u64(b"12g").is_err());
}

#[test]
fn trim_end_only_removes_ws() {
    assert_eq!(trim_end(b"  text  \n"), &b"  text"[..]);
    assert_eq!(trim_end(b"text"), &b"text"[..]);
    assert_eq!(trim_end(b"\t\n\r  "), &b""[..]);
}

#[test]
fn trim_start_only_removes_ws() {
    assert_eq!(trim_start(b"  text  \n"), &b"text  \n"[..]);
    assert_eq!(trim_start(b"text"), &b"text"[..]);
}

#[test]
fn trim_both_ends() {
    assert_eq!(trim(b"  text  \n"), &b"text"[..]);
    assert_eq!(trim(b"\t\n\r "), &b""[..]);
}

#[test]
fn split_at_byte_middle() {
    assert_eq!(split_at_byte(b"ab:cd", b':'), (&b"ab"[..], &b"cd"[..]));
}

#[test]
fn split_at_byte_not_found() {
    assert_eq!(split_at_byte(b"abcd", b':'), (&b"abcd"[..], &b""[..]));
}

#[test]
fn split_at_byte_first_and_last() {
    assert_eq!(split_at_byte(b":ab", b':'), (&b""[..], &b"ab"[..]));
    assert_eq!(split_at_byte(b"ab:", b':'), (&b"ab"[..], &b""[..]));
}

#[test]
fn memchr_finds_first() {
    assert_eq!(memchr(b'a', b"banana"), Some(1));
    assert_eq!(memchr(b'z', b"banana"), None);
}

#[test]
fn parse_key_value_line_basic() {
    // The value is trimmed on the right only; a leading space stays.
    assert_eq!(
        parse_key_value_line(b"key: value"),
        Some((&b"key"[..], &b" value"[..]))
    );
    assert_eq!(
        parse_key_value_line(b"key:value"),
        Some((&b"key"[..], &b"value"[..]))
    );
}

#[test]
fn parse_key_value_line_extra_colons() {
    // Splits on the first colon only.
    assert_eq!(
        parse_key_value_line(b"a:b:c"),
        Some((&b"a"[..], &b"b:c"[..]))
    );
}

#[test]
fn parse_key_value_line_missing_colon() {
    assert_eq!(parse_key_value_line(b"no colon here"), None);
}

#[test]
fn split_spaces_collapses_runs() {
    assert_eq!(
        split_spaces(b"a  b\tc"),
        vec![&b"a"[..], &b"b"[..], &b"c"[..]]
    );
}

#[test]
fn split_spaces_skips_leading_trailing() {
    assert_eq!(split_spaces(b"  leading"), vec![&b"leading"[..]]);
    assert_eq!(split_spaces(b"trailing  "), vec![&b"trailing"[..]]);
}

#[test]
fn split_spaces_empty() {
    assert!(split_spaces(b"").is_empty());
    assert!(split_spaces(b"   ").is_empty());
}

#[test]
fn split_fields_merges_runs() {
    let fields = SplitFields::<8>::new(b"a  b\tc");
    assert_eq!(&fields[..], &[&b"a"[..], &b"b"[..], &b"c"[..]]);
}

#[test]
fn split_fields_caps_at_n() {
    let fields = SplitFields::<2>::new(b"a b c");
    assert_eq!(&fields[..], &[&b"a"[..], &b"b"[..]]);
}

#[test]
fn split_fields_empty() {
    let fields = SplitFields::<4>::new(b"");
    assert!(fields.is_empty());
}

#[test]
fn first_token_basic() {
    assert_eq!(first_token(b"hello world"), &b"hello"[..]);
    assert_eq!(first_token(b"single"), &b"single"[..]);
}

#[test]
fn first_token_kibibytes_suffix() {
    // Strips the " kB" unit suffix from meminfo/status values.
    assert_eq!(first_token(b"9764412 kB"), &b"9764412"[..]);
}
