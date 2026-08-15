#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::ProcessStat;

// Fuzz the /proc/PID/stat parser.
fuzz_target!(|data: &[u8]| {
    let _ = ProcessStat::from_bytes(data);
});
