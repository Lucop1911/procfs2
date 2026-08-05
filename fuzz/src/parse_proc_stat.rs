#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(stat) = procfs2::proc::stat::SystemStat::from_bytes(data) {
        let _ = stat.cpu_total.user.0;
        let _ = stat.cpu_total.system.0;
        let _ = stat.cpu_total.idle.0;
        let _ = stat.cpu_total.iowait.0;
    }
});
