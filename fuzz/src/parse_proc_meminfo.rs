#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(info) = procfs2::proc::meminfo::MemInfo::from_bytes(data) {
        let _ = info.total.0;
        let _ = info.free.0;
        let _ = info.available.0;
    }
});
