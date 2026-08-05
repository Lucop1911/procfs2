#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(maps) = procfs2::proc::process::maps::parse_maps(data) {
        for map in maps {
            let _ = map.address.start;
            let _ = map.address.end;
            let _ = map.perms;
            let _ = map.pathname;
        }
    }
});
