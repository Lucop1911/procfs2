#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::MemoryMap;

// Fuzz the /proc/PID/maps parser.
// Ensures malformed maps data never panics.
fuzz_target!(|data: &[u8]| {
    let _ = MemoryMap::parse_all(data);
});
