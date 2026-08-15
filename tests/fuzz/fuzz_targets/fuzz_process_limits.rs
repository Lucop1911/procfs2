#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::ProcessLimits;

// Fuzz the /proc/PID/limits parser (fixed-width table format).
fuzz_target!(|data: &[u8]| {
    let _ = ProcessLimits::from_bytes(data);
});
