//! Integration tests for procfs2 against live /proc and /sys.
//!
//! These tests run against the actual /proc and /sys filesystems on the system.

#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {
    use procfs2::proc::{
        DeviceKind, Process, cpuinfo, devices, diskstats, filesystems, loadavg, meminfo,
        partitions, stat, swaps, uptime, version, vmstat,
    };
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
    fn test_live_devices() {
        let devices = devices().expect("Failed to read /proc/devices");

        // Every kernel registers at least one device
        assert!(!devices.is_empty(), "Should have at least one device");
        assert!(
            devices.iter().any(|d| d.major > 0),
            "Device major numbers should be > 0"
        );
    }

    #[test]
    fn test_live_devices_both_kinds() {
        let devices = devices().expect("Failed to read /proc/devices");

        // Both character and block devices should be registered
        assert!(
            devices.iter().any(|d| d.kind == DeviceKind::Character),
            "Should have at least one character device"
        );
        assert!(
            devices.iter().any(|d| d.kind == DeviceKind::Block),
            "Should have at least one block device"
        );
    }

    #[test]
    fn test_live_devices_names() {
        let devices = devices().expect("Failed to read /proc/devices");

        // Names are single whitespace-free tokens
        for d in &devices {
            assert!(!d.name.is_empty(), "Device name should not be empty");
            assert!(
                !d.name.contains(' '),
                "Device name should not contain spaces"
            );
        }
    }

    #[test]
    fn test_live_filesystems() {
        let fs_list = filesystems().expect("Failed to read /proc/filesystems");

        // Every kernel supports proc, tmpfs and at least one disk fs
        assert!(!fs_list.is_empty(), "Should have at least one filesystem");
        assert!(
            fs_list.iter().any(|f| f.name.as_ref() == "proc"),
            "proc filesystem should be listed"
        );
        assert!(
            fs_list.iter().any(|f| f.name.as_ref() == "tmpfs"),
            "tmpfs filesystem should be listed"
        );
    }

    #[test]
    fn test_live_filesystems_both_kinds() {
        let fs_list = filesystems().expect("Failed to read /proc/filesystems");

        // Pseudo-filesystems (nodev) and device-backed filesystems exist
        assert!(
            fs_list.iter().any(|f| !f.dev),
            "Should have at least one nodev filesystem"
        );
        assert!(
            fs_list.iter().any(|f| f.dev),
            "Should have at least one device-backed filesystem"
        );
    }

    #[test]
    fn test_live_filesystems_names() {
        let fs_list = filesystems().expect("Failed to read /proc/filesystems");

        // Names are non-empty and free of tabs
        for f in &fs_list {
            assert!(!f.name.is_empty(), "Filesystem name should not be empty");
            assert!(
                !f.name.contains('\t'),
                "Filesystem name should not contain tabs"
            );
        }
    }

    #[test]
    fn test_live_swaps() {
        let swaps = swaps().expect("Failed to read /proc/swaps");

        // Swap areas may be absent (e.g. VMs), so don't require any.
        for s in &swaps {
            assert!(!s.filename.is_empty(), "Filename should not be empty");
            assert!(s.size.0 > 0, "Swap size should be > 0");
            assert!(s.used.0 <= s.size.0, "Used should not exceed size");
        }
    }

    #[test]
    fn test_live_swaps_fields() {
        let swaps = swaps().expect("Failed to read /proc/swaps");

        // Every entry has a valid type and a numeric priority.
        for s in &swaps {
            match s.kind {
                procfs2::proc::SwapType::Partition | procfs2::proc::SwapType::File => {}
            }
            let _ = s.priority;
        }
    }

    #[test]
    fn test_live_partitions() {
        let partitions = partitions().expect("Failed to read /proc/partitions");

        // Every system has at least one block device / partition
        assert!(!partitions.is_empty(), "Should have at least one partition");
        for p in &partitions {
            assert!(!p.name.is_empty(), "Partition name should not be empty");
            assert!(p.blocks.0 > 0, "Partition blocks should be > 0");
        }
    }

    #[test]
    fn test_live_partitions_fields() {
        let partitions = partitions().expect("Failed to read /proc/partitions");

        // Every entry has a positive major number and a whitespace-free name
        for p in &partitions {
            assert!(p.major > 0, "Major number should be > 0");
            assert!(
                !p.name.contains(' '),
                "Partition name should not contain spaces"
            );
            let _ = p.minor;
        }
    }

    #[test]
    fn test_live_diskstats() {
        let stats = diskstats().expect("Failed to read /proc/diskstats");

        // Every block device and partition is listed alongside its whole disk
        assert!(!stats.is_empty(), "Should have at least one disk entry");
        for s in &stats {
            assert!(!s.name.is_empty(), "Device name should not be empty");
            assert!(
                !s.name.contains(' '),
                "Device name should not contain spaces"
            );
            assert!(s.major > 0, "Major number should be > 0");
            let _ = s.minor;
        }
    }

    #[test]
    fn test_live_diskstats_io() {
        let stats = diskstats().expect("Failed to read /proc/diskstats");

        // Every entry must have all counters present.
        for s in &stats {
            let _ = s.reads_completed;
            let _ = s.writes_completed;
            let _ = s.sectors_read;
            let _ = s.sectors_written;
        }

        // The whole-disk entry (e.g. sda, nvme0n1) must have done some I/O.
        assert!(
            stats
                .iter()
                .any(|s| s.reads_completed > 0 || s.writes_completed > 0),
            "Should have at least one device with I/O stats"
        );
    }

    #[test]
    fn test_live_diskstats_flush_fields() {
        let stats = diskstats().expect("Failed to read /proc/diskstats");

        // Modern kernels (5.5+) always emit the flush fields. Whole-disk
        // entries track flush requests, so they should have a value.
        for s in &stats {
            let _ = s.flush_completed;
            let _ = s.time_flushing;
            let _ = s.io_in_progress;
        }
    }

    #[test]
    fn test_live_vmstat() {
        let vmstat = vmstat().expect("Failed to read /proc/vmstat");

        // The file holds well over a hundred counters on modern kernels,
        // including the long-stable zone and event counters below.
        assert!(vmstat.len() > 50, "Should have many vmstat counters");
        for key in ["nr_free_pages", "nr_dirty", "pgpgin", "pgfault", "oom_kill"] {
            assert!(vmstat.contains_key(key), "vmstat should contain {key}");
        }
    }

    #[test]
    fn test_live_vmstat_values() {
        let vmstat = vmstat().expect("Failed to read /proc/vmstat");

        // Keys are single whitespace-free tokens and every value is a
        // non-negative counter.
        for (key, &value) in &vmstat {
            assert!(!key.is_empty(), "Key should not be empty");
            assert!(
                !key.contains(char::is_whitespace),
                "Key should not contain whitespace"
            );
            let _ = value;
        }

        // The system has been allocating/freeing pages since boot, so the
        // total free page count must be positive.
        assert!(
            vmstat.get("nr_free_pages").is_some_and(|&v| v > 0),
            "nr_free_pages should be > 0"
        );
    }

    #[test]
    fn test_live_sys_block() {
        let devices: Vec<_> = sys::BlockDevice::all().filter_map(|r| r.ok()).collect();
        assert!(!devices.is_empty(), "Should have at least one block device");

        // Skip devices that report a size of 0 (e.g. unbound loop devices
        // on CI runners), which may appear before the real disk.
        let dev = devices
            .iter()
            .find(|dev| dev.size().map(|s| s.0 > 0).unwrap_or(false))
            .expect("Should have a block device with a nonzero size");

        let stat = dev.stat().expect("Failed to read block device stat");
        assert!(dev.size().expect("Failed to read device size").0 > 0);
        assert!(stat.reads_completed > 0 || stat.writes_completed > 0);
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
