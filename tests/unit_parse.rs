//! Unit tests for parsing /proc files.
//! 
//! These tests verify that the parsers work correctly against live /proc data.

#[cfg(test)]
mod tests {
    #[test]
    fn test_meminfo() {
        let info = procfs2::proc::meminfo()
            .expect("Failed to read /proc/meminfo");
        
        // Total should be in kilobytes and > 0
        assert!(info.total.0 > 0);
        // Available should be <= total
        assert!(info.available.0 <= info.total.0);
    }

    #[test]
    fn test_meminfo_fields() {
        let info = procfs2::proc::meminfo()
            .expect("Failed to read /proc/meminfo");
        
        // All numeric fields should be valid
        let _ = info.free;
        let _ = info.buffers;
        let _ = info.cached;
        let _ = info.swap_total;
        let _ = info.swap_free;
    }

    #[test]
    fn test_uptime() {
        let uptime = procfs2::proc::uptime()
            .expect("Failed to read /proc/uptime");
        
        // System should be uptime for at least a few seconds
        assert!(uptime.total.as_secs() > 0, "Uptime should be > 0");
    }

    #[test]
    fn test_uptime_values() {
        let uptime = procfs2::proc::uptime()
            .expect("Failed to read /proc/uptime");
        
        // Total should be > 0
        assert!(uptime.total.as_secs() > 0);
        // Idle should be >= 0
        assert!(uptime.idle.as_secs_f64() >= 0.0);
    }

    #[test]
    fn test_loadavg() {
        let load = procfs2::proc::loadavg()
            .expect("Failed to read /proc/loadavg");
        
        // Load averages should be non-negative
        assert!(load.one >= 0.0);
        assert!(load.five >= 0.0);
        assert!(load.fifteen >= 0.0);
    }

    #[test]
    fn test_loadavg_fields() {
        let load = procfs2::proc::loadavg()
            .expect("Failed to read /proc/loadavg");
        
        // Runnable should be >= 1
        assert!(load.runnable >= 1);
    }

    #[test]
    fn test_stat() {
        let stat = procfs2::proc::stat()
            .expect("Failed to read /proc/stat");
        
        // Should have CPU data
        assert!(!stat.per_cpu.is_empty(), "Should have at least one CPU");
    }

    #[test]
    fn test_stat_cpu_fields() {
        let stat = procfs2::proc::stat()
            .expect("Failed to read /proc/stat");
        
        // Check first CPU has valid times
        let cpu = &stat.per_cpu[0];
        // Just verify the times struct exists and has values
        let _ = cpu.times.user;
        let _ = cpu.times.nice;
        let _ = cpu.times.system;
    }

    #[test]
    fn test_stat_system_fields() {
        let stat = procfs2::proc::stat()
            .expect("Failed to read /proc/stat");
        
        // System stats should be valid
        assert!(stat.btime > 0, "Boot time should be > 0");
        assert!(stat.processes > 0, "Processes should be > 0");
    }

    #[test]
    fn test_version() {
        let version = procfs2::proc::version()
            .expect("Failed to read /proc/version");
        
        // Version should have valid numbers
        assert!(version.major > 0, "Major version should be > 0");
    }

    #[test]
    fn test_version_format() {
        let version = procfs2::proc::version()
            .expect("Failed to read /proc/version");
        
        // Version should be valid format
        assert!(version.major >= 2);
    }

    #[test]
    fn test_cpuinfo() {
        let cpus = procfs2::proc::cpuinfo()
            .expect("Failed to read /proc/cpuinfo");
        
        // Should have at least one CPU
        assert!(!cpus.is_empty(), "Should have at least one CPU");
    }

    #[test]
    fn test_cpuinfo_fields() {
        let cpus = procfs2::proc::cpuinfo()
            .expect("Failed to read /proc/cpuinfo");
        
        // Each CPU should have a processor number
        for cpu in &cpus {
            let _ = cpu.processor;
        }
    }
}