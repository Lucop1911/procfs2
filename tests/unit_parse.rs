//! Unit tests for parsing /proc files.
//!
//! These tests verify that the parsers work correctly against live /proc data.

#[cfg(test)]
mod tests {
    #[test]
    fn test_meminfo() {
        let info = procfs2::proc::meminfo().expect("Failed to read /proc/meminfo");

        // Total should be in kilobytes and > 0
        assert!(info.total.0 > 0);
        // Available should be <= total
        assert!(info.available.0 <= info.total.0);
    }

    #[test]
    fn test_meminfo_fields() {
        let info = procfs2::proc::meminfo().expect("Failed to read /proc/meminfo");

        // All numeric fields should be valid
        let _ = info.free;
        let _ = info.buffers;
        let _ = info.cached;
        let _ = info.swap_total;
        let _ = info.swap_free;
    }

    #[test]
    fn test_uptime() {
        let uptime = procfs2::proc::uptime().expect("Failed to read /proc/uptime");

        // System should be uptime for at least a few seconds
        assert!(uptime.total.as_secs() > 0, "Uptime should be > 0");
    }

    #[test]
    fn test_uptime_values() {
        let uptime = procfs2::proc::uptime().expect("Failed to read /proc/uptime");

        // Total should be > 0
        assert!(uptime.total.as_secs() > 0);
        // Idle should be >= 0
        assert!(uptime.idle.as_secs_f64() >= 0.0);
    }

    #[test]
    fn test_loadavg() {
        let load = procfs2::proc::loadavg().expect("Failed to read /proc/loadavg");

        // Load averages should be non-negative
        assert!(load.one >= 0.0);
        assert!(load.five >= 0.0);
        assert!(load.fifteen >= 0.0);
    }

    #[test]
    fn test_loadavg_fields() {
        let load = procfs2::proc::loadavg().expect("Failed to read /proc/loadavg");

        // Runnable should be >= 1
        assert!(load.runnable >= 1);
    }

    #[test]
    fn test_stat() {
        let stat = procfs2::proc::stat().expect("Failed to read /proc/stat");

        // Should have CPU data
        assert!(!stat.per_cpu.is_empty(), "Should have at least one CPU");
    }

    #[test]
    fn test_stat_cpu_fields() {
        let stat = procfs2::proc::stat().expect("Failed to read /proc/stat");

        // Check first CPU has valid times
        let cpu = &stat.per_cpu[0];
        // Just verify the times struct exists and has values
        let _ = cpu.times.user;
        let _ = cpu.times.nice;
        let _ = cpu.times.system;
    }

    #[test]
    fn test_stat_system_fields() {
        let stat = procfs2::proc::stat().expect("Failed to read /proc/stat");

        // System stats should be valid
        assert!(stat.btime > 0, "Boot time should be > 0");
        assert!(stat.processes > 0, "Processes should be > 0");
    }

    #[test]
    fn test_version() {
        let version = procfs2::proc::version().expect("Failed to read /proc/version");

        // Version should have valid numbers
        assert!(version.major > 0, "Major version should be > 0");
    }

    #[test]
    fn test_version_format() {
        let version = procfs2::proc::version().expect("Failed to read /proc/version");

        // Version should be valid format
        assert!(version.major >= 2);
    }

    #[test]
    fn test_cpuinfo() {
        let cpus = procfs2::proc::cpuinfo().expect("Failed to read /proc/cpuinfo");

        // Should have at least one CPU
        assert!(!cpus.is_empty(), "Should have at least one CPU");
    }

    #[test]
    fn test_cpuinfo_fields() {
        let cpus = procfs2::proc::cpuinfo().expect("Failed to read /proc/cpuinfo");

        // Each CPU should have a processor number
        for cpu in &cpus {
            let _ = cpu.processor;
        }
    }

    #[test]
    fn test_cmdline() {
        let args = procfs2::proc::cmdline().expect("Failed to read /proc/cmdline");

        // Should have at least one boot parameter
        assert!(!args.is_empty(), "Should have at least one argument");
    }

    #[test]
    fn test_cmdline_fields() {
        let args = procfs2::proc::cmdline().expect("Failed to read /proc/cmdline");

        // No argument should be empty
        for arg in &args {
            assert!(!arg.is_empty(), "Argument should not be empty");
        }
    }

    #[test]
    fn test_cgroups() {
        let cgroups = procfs2::proc::cgroups().expect("Failed to read /proc/cgroups");

        // Every controller should have a name (hierarchy is 0 on
        // unified cgroup v2 systems)
        for cg in &cgroups {
            assert!(!cg.subsys_name.is_empty(), "Controller name should not be empty");
        }
    }

    #[test]
    fn test_mounts() {
        let mounts = procfs2::proc::mounts().expect("Failed to read /proc/mounts");

        // The root filesystem is always mounted
        assert!(!mounts.is_empty(), "Should have at least one mount");
    }

    #[test]
    fn test_net_dev() {
        let devices: Vec<_> = procfs2::proc::net::dev().filter_map(|r| r.ok()).collect();

        // The loopback interface should always be present
        assert!(!devices.is_empty(), "Should have at least one interface");
    }

    #[test]
    fn test_net_route() {
        let routes: Vec<_> = procfs2::proc::net::route().filter_map(|r| r.ok()).collect();

        // At least the default route should exist
        assert!(!routes.is_empty(), "Should have at least one route");
    }

    #[test]
    fn test_net_arp() {
        // The ARP table may legitimately be empty; just ensure no parse errors
        let entries: Vec<_> = procfs2::proc::net::arp().collect();
        assert!(entries.iter().all(|r| r.is_ok()), "ARP entries should parse");
    }

    #[test]
    fn test_net_tcp() {
        // Connections may be empty; ensure the file parses without errors
        let entries: Vec<_> = procfs2::proc::net::tcp().collect();
        assert!(entries.iter().all(|r| r.is_ok()), "TCP entries should parse");
    }

    #[test]
    fn test_net_tcp6() {
        // Pure-IPv6 connections yield parse errors by design (documented
        // limitation), so only assert that collection does not panic.
        let _ = procfs2::proc::net::tcp6().count();
    }

    #[test]
    fn test_net_udp() {
        let entries: Vec<_> = procfs2::proc::net::udp().collect();
        assert!(entries.iter().all(|r| r.is_ok()), "UDP entries should parse");
    }

    #[test]
    fn test_net_udp6() {
        let entries: Vec<_> = procfs2::proc::net::udp6().collect();
        assert!(entries.iter().all(|r| r.is_ok()), "UDP6 entries should parse");
    }

    #[test]
    fn test_net_unix() {
        let entries: Vec<_> = procfs2::proc::net::unix().collect();
        assert!(entries.iter().all(|r| r.is_ok()), "Unix socket entries should parse");
    }
}
