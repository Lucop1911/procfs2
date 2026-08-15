#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::MountInfo;

// Fuzz the /proc/PID/mountinfo parser.
fuzz_target!(|data: &[u8]| {
    let _ = MountInfo::parse_all(data);
});
