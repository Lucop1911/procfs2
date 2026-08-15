#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::ProcessStatus;

// Fuzz the /proc/PID/status parser.
fuzz_target!(|data: &[u8]| {
    let _ = ProcessStatus::from_bytes(data);
});
