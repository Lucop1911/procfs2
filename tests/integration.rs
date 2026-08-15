//! Integration tests for procfs2 against live /proc and /sys.
//!
//! These tests run against the actual /proc and /sys filesystems on the system.

#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {
    use procfs2::proc::{Process, cpuinfo, loadavg, meminfo, stat, uptime, version};
    use procfs2::sys;

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

    #[test]
    fn test_process_new() {
        let me = Process::current().expect("Failed to get current process");

        // Valid pid resolves
        assert!(Process::new(me.pid).is_ok());

        // pid 0 never exists in /proc
        match Process::new(0) {
            Err(procfs2::Error::ProcessGone(0)) => {}
            other => panic!("expected ProcessGone for pid 0, got {:?}", other),
        }
    }

    #[test]
    fn test_live_process_cmdline() {
        let me = Process::current().expect("Failed to get current process");
        let cmdline = me.cmdline().expect("Failed to read process cmdline");
        assert!(!cmdline.is_empty(), "cmdline should not be empty");
    }

    #[test]
    fn test_live_process_environ() {
        let me = Process::current().expect("Failed to get current process");
        let environ = me.environ().expect("Failed to read process environ");
        assert!(!environ.is_empty(), "environ should not be empty");
    }

    #[test]
    fn test_live_process_exe() {
        let me = Process::current().expect("Failed to get current process");
        let exe = me.exe().expect("Failed to read process exe");
        assert!(exe.is_absolute(), "exe path should be absolute");
        assert!(exe.exists(), "exe path should exist");
    }

    #[test]
    fn test_live_process_cwd() {
        let me = Process::current().expect("Failed to get current process");
        let cwd = me.cwd().expect("Failed to read process cwd");
        assert!(cwd.is_absolute(), "cwd should be absolute");
        assert!(cwd.is_dir(), "cwd should be a directory");
    }

    #[test]
    fn test_live_process_smaps() {
        let me = Process::current().expect("Failed to get current process");
        let smaps = me.smaps().expect("Failed to read process smaps");
        assert!(!smaps.is_empty(), "smaps should not be empty");
    }

    #[test]
    fn test_live_process_smaps_rollup() {
        let me = Process::current().expect("Failed to get current process");
        let rollup = me.smaps_rollup().expect("Failed to read smaps_rollup");

        // A running process must occupy at least some memory
        assert!(rollup.rss_kb > 0, "rss should be > 0");
    }

    #[test]
    fn test_live_process_fds() {
        let me = Process::current().expect("Failed to get current process");

        // Concurrent test threads open/close fds while we enumerate,
        // so read_link may transiently fail on a closing fd. Retry.
        let mut fds = None;
        for _ in 0..5 {
            if let Ok(v) = me.fds() {
                fds = Some(v);
                break;
            }
        }
        let fds = fds.expect("Failed to read process fds");

        // stdin/stdout/stderr are always open
        assert!(!fds.is_empty(), "should have at least one open fd");
    }

    #[test]
    fn test_live_process_limits() {
        let me = Process::current().expect("Failed to get current process");
        let limits = me.limits().expect("Failed to read process limits");

        // Open files limit should be a sensible value
        assert!(
            limits.max_open_files.soft.is_some(),
            "open files soft limit should be set"
        );
    }

    #[test]
    fn test_live_process_cgroup() {
        let me = Process::current().expect("Failed to get current process");
        let cgroups = me.cgroup().expect("Failed to read process cgroup");
        assert!(
            !cgroups.is_empty(),
            "process should be in at least one cgroup"
        );
    }

    #[test]
    fn test_live_process_namespaces() {
        let me = Process::current().expect("Failed to get current process");
        let ns = me.namespaces().expect("Failed to read process namespaces");

        // These namespaces exist on all modern kernels
        assert!(ns.mnt.is_some(), "mnt namespace should be present");
        assert!(ns.pid.is_some(), "pid namespace should be present");
        assert!(ns.net.is_some(), "net namespace should be present");
    }

    #[test]
    fn test_live_process_threads() {
        let me = Process::current().expect("Failed to get current process");
        let count = me.threads().filter(|r| r.is_ok()).count();
        assert!(count >= 1, "process should have at least one thread");
    }

    #[test]
    fn test_live_sys_block() {
        let devices: Vec<_> = sys::BlockDevice::all().filter_map(|r| r.ok()).collect();
        assert!(!devices.is_empty(), "Should have at least one block device");

        if let Some(dev) = devices.first() {
            let stat = dev.stat().expect("Failed to read block device stat");
            assert!(
                dev.size().expect("Failed to read device size").0 > 0,
                "size should be > 0"
            );
            assert!(stat.reads_completed > 0 || stat.writes_completed > 0);
        }
    }

    #[test]
    fn test_live_sys_cpu() {
        let count = sys::cpu_count().expect("Failed to read cpu count");
        assert!(count > 0, "CPU count should be > 0");

        let cpus = sys::online_cpus().expect("Failed to read online cpus");
        assert!(!cpus.is_empty(), "Should have at least one online CPU");
    }

    #[test]
    fn test_live_sys_net() {
        let ifaces: Vec<_> = sys::NetInterface::all().filter_map(|r| r.ok()).collect();
        assert!(!ifaces.is_empty(), "Should have at least one interface");

        if let Some(iface) = ifaces.first() {
            let stats = iface.stats().expect("Failed to read interface stats");
            assert!(
                iface.mtu().expect("Failed to read mtu") > 0,
                "MTU should be > 0"
            );
            assert!(
                iface
                    .flags()
                    .expect("Failed to read flags")
                    .contains(sys::NetIfFlags::UP),
                "interface should be up"
            );
            let _ = stats.rx_bytes.0;
        }
    }

    #[test]
    fn test_live_sys_power() {
        // Power supplies may be absent (e.g. some VMs); just check that
        // any present supply reports a valid type.
        let supplies: Vec<_> = sys::PowerSupply::all().filter_map(|r| r.ok()).collect();
        for ps in &supplies {
            assert!(ps.kind().is_ok(), "Power supply kind should parse");
        }
    }
}
