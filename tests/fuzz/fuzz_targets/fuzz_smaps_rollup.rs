#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::SmapsRollup;

// Fuzz the /proc/PID/smaps_rollup parser.
fuzz_target!(|data: &[u8]| {
    let _ = SmapsRollup::from_bytes(data);
});
