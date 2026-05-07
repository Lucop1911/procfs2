//! Integration tests for procfs2 against live /proc and /sys.
//! 
//! These tests run against the actual /proc and /sys filesystems on the system.

#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {
    use procfs2::proc::{meminfo, uptime, loadavg, stat, version, cpuinfo, Process};

    #[test]
    fn test_live_meminfo() {
        let info = meminfo().expect("Failed to read /proc/meminfo");
        assert!(info.total.0 > 0, "Total memory should be > 0");
    }

    #[test]
    fn test_live_uptime() {
        let up = uptime().expect("Failed to read /proc/uptime");
        assert!(up.total.as_secs() > 0, "Uptime should be > 0");
    }

    #[test]
    fn test_live_loadavg() {
        let load = loadavg().expect("Failed to read /proc/loadavg");
        assert!(load.one >= 0.0);
    }

    #[test]
    fn test_live_stat() {
        let st = stat().expect("Failed to read /proc/stat");
        assert!(!st.per_cpu.is_empty());
    }

    #[test]
    fn test_live_version() {
        let ver = version().expect("Failed to read /proc/version");
        assert!(ver.major > 0);
    }

    #[test]
    fn test_live_cpuinfo() {
        let cpus = cpuinfo().expect("Failed to read /proc/cpuinfo");
        assert!(!cpus.is_empty());
    }

    #[test]
    fn test_live_current_process() {
        let process = Process::current().expect("Failed to get current process");
        assert!(process.pid > 0, "PID should be > 0");
    }

    #[test]
    fn test_live_process_status() {
        let process = Process::current().expect("Failed to get current process");
        let status = process.status().expect("Failed to read process status");
        assert!(!status.name.is_empty());
    }

    #[test]
    fn test_live_process_maps() {
        let process = Process::current().expect("Failed to get current process");
        let maps = process.maps().expect("Failed to read process maps");
        assert!(!maps.is_empty());
    }

    #[test]
    fn test_process_iteration() {
        let mut count = 0;
        for result in Process::all() {
            if result.is_ok() {
                count += 1;
            }
        }
        assert!(count >= 1);
    }
}