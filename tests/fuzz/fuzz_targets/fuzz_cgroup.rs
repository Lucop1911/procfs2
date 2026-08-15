#![no_main]

use libfuzzer_sys::fuzz_target;
use procfs2::proc::process::CgroupEntry;

// Fuzz the /proc/PID/cgroup parser.
fuzz_target!(|data: &[u8]| {
    let _ = CgroupEntry::parse_all(data);
});
