#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::ProcessIo;

// Fuzz the /proc/PID/io parser.
fuzz_target!(|data: &[u8]| {
    let _ = ProcessIo::from_bytes(data);
});
