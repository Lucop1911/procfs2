#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzz the low-level byte parsing utilities shared by every /proc parser.
// Malformed input must never panic (errors are returned instead).
fuzz_target!(|data: &[u8]| {
    use procfs2::util::parse;

    let _ = parse::parse_dec_u64(data);
    let _ = parse::parse_dec_u32(data);
    let _ = parse::parse_dec_i64(data);
    let _ = parse::parse_dec_f64(data);
    let _ = parse::parse_hex_u64(data);

    for line in parse::split_lines(data) {
        let _ = parse::parse_key_value_line(line);
        let _ = parse::parse_dec_u64(line);
    }
});
