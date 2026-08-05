//! Fuzz targets for procfs2 parsers.
//! 
//! These targets ensure that malformed /proc file data never causes panics.
//! Run with: cargo fuzz run <target_name>

#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::util::parse::ParseFromBytes;
use procfs2::proc::meminfo::MemInfo;
use procfs2::proc::uptime::Uptime;
use procfs2::proc::loadavg::LoadAvg;

// Fuzz target for meminfo parser
// Ensures malformed meminfo data doesn't panic
fuzz_target!(|data: &[u8]| {
    // Try to parse the data - we don't care about the result,
    // just that it doesn't panic
    let _ = MemInfo::parse_from_bytes(data);
});

// Fuzz target for uptime parser
fuzz_target!(|data: &[u8]| {
    // Try to parse uptime data
    let _ = Uptime::parse_from_bytes(data);
});

// Fuzz target for loadavg parser  
fuzz_target!(|data: &[u8]| {
    // Try to parse load average data
    let _ = LoadAvg::parse_from_bytes(data);
});

// Fuzz target for general key-value parsing
// Tests the underlying parse_key_value_line function
fuzz_target!(|data: &[u8]| {
    // Try to parse each line individually
    for line in data.split(|&b| b == b'\n') {
        if !line.is_empty() {
            let _ = procfs2::util::parse::parse_key_value_line(line);
        }
    }
});

// Fuzz target for numeric parsing functions
fuzz_target!(|data: &[u8]| {
    // Test decimal parsing
    let _ = procfs2::util::parse::parse_dec_u64(data);
    let _ = procfs2::util::parse::parse_dec_u32(data);
    let _ = procfs2::util::parse::parse_dec_i64(data);
    
    // Test hex parsing
    let _ = procfs2::util::parse::parse_hex_u64(data);
    
    // Test float parsing
    let _ = procfs2::util::parse::parse_dec_f32(data);
    let _ = procfs2::util::parse::parse_dec_f64(data);
});