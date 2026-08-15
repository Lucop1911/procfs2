#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::MemoryMapDetail;

// Fuzz the /proc/PID/smaps parser (multi-region key-value format).
fuzz_target!(|data: &[u8]| {
    let _ = MemoryMapDetail::parse_all(data);
});
