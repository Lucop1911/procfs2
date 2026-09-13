//! Integration tests for procfs2 against live /proc and /sys.
//!
//! These tests run against the actual /proc and /sys filesystems on the system.

#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {
    use procfs2::proc::{
        ConsoleFlags, DeviceKind, Process, buddyinfo, consoles, cpu_pressure, cpuinfo, crypto,
        devices, diskstats, filesystems, interrupts, io_pressure, iomem, ioports, irq_pressure,
        loadavg, locks, meminfo, memory_pressure, misc, modules, pagetypeinfo, partitions,
        softirqs, stat, swaps, uptime, version, vmstat, zoneinfo,
    };
    use procfs2::sys;

    /// `/proc/pagetypeinfo` is mode 0400 (root-only) on some systems.
    /// Returns `None` when unreadable for that reason; panics on other
    /// failures so tests still catch genuine parse bugs.
    fn pagetypeinfo_or_skip() -> Option<procfs2::proc::PageTypeInfo> {
        match pagetypeinfo() {
            Ok(info) => Some(info),
            Err(procfs2::Error::Io { error: e, .. })
                if e.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                None
            }
            Err(e) => panic!("Failed to read /proc/pagetypeinfo: {e}"),
        }
    }

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
    fn test_live_zoneinfo() {
        let zones = zoneinfo().expect("Failed to read /proc/zoneinfo");

        // Every kernel has at least one memory zone; the Normal zone is
        // always populated on 64-bit systems.
        assert!(!zones.is_empty(), "Should have at least one zone");
        let normal = zones
            .iter()
            .find(|z| z.name.as_ref() == "Normal")
            .expect("Should have a Normal zone");
        assert!(normal.pages.present > 0, "Normal zone should be populated");
        assert!(
            !normal.protection.is_empty(),
            "Normal zone should have protection values"
        );
        assert!(
            normal.stats.contains_key("nr_free_pages"),
            "Normal zone should have nr_free_pages"
        );
    }

    #[test]
    fn test_live_zoneinfo_fields() {
        let zones = zoneinfo().expect("Failed to read /proc/zoneinfo");

        // Every zone has a non-empty name; populated zones (managed > 0)
        // must have their `free` page count match the per-zone counter.
        for z in &zones {
            assert!(!z.name.is_empty(), "Zone name should not be empty");
            if z.pages.managed > 0 {
                assert_eq!(
                    z.pages.free,
                    z.stats.get("nr_free_pages").copied().unwrap_or(0),
                    "free pages should match nr_free_pages for {}",
                    z.name
                );
            }
        }
    }

    #[test]
    fn test_live_zoneinfo_pagesets() {
        let zones = zoneinfo().expect("Failed to read /proc/zoneinfo");

        // Every populated zone has one pageset per online CPU, with
        // ascending CPU ids starting at 0.
        for z in zones.iter().filter(|z| z.pages.managed > 0) {
            assert!(!z.pagesets.is_empty(), "{} should have pagesets", z.name);
            for (i, pcp) in z.pagesets.iter().enumerate() {
                assert_eq!(pcp.cpu, i as u32, "{} pageset CPU ids", z.name);
            }
        }
    }

    #[test]
    fn test_live_buddyinfo() {
        let buddy = buddyinfo().expect("Failed to read /proc/buddyinfo");

        // Every kernel has at least one populated zone with buddy lists,
        // and the Normal zone is always present on 64-bit systems.
        assert!(!buddy.is_empty(), "Should have at least one zone");
        let normal = buddy
            .iter()
            .find(|b| b.zone.as_ref() == "Normal")
            .expect("Should have a Normal zone");
        assert!(
            !normal.free_lists.is_empty(),
            "Normal zone should have free lists"
        );
    }

    #[test]
    fn test_live_buddyinfo_fields() {
        let buddy = buddyinfo().expect("Failed to read /proc/buddyinfo");

        // Every entry has a non-empty whitespace-free zone name and at
        // least one buddy order count.
        for b in &buddy {
            assert!(!b.zone.is_empty(), "Zone name should not be empty");
            assert!(
                !b.zone.contains(char::is_whitespace),
                "Zone name should not contain whitespace"
            );
            assert!(
                !b.free_lists.is_empty(),
                "{} should have at least one free list",
                b.zone
            );
            let _ = b.node;
        }
    }

    #[test]
    fn test_live_buddyinfo_orders() {
        let buddy = buddyinfo().expect("Failed to read /proc/buddyinfo");

        // The kernel emits the same MAX_ORDER column count on every line,
        // so all zones share a consistent free-list length.
        let orders = buddy[0].free_lists.len();
        assert!(orders > 0, "Should have at least one order");
        for b in &buddy {
            assert_eq!(
                b.free_lists.len(),
                orders,
                "{} free-list length should match",
                b.zone
            );
        }
    }

    #[test]
    fn test_live_pagetypeinfo() {
        let Some(info) = pagetypeinfo_or_skip() else {
            return;
        };

        // The page-block header describes the allocator's block size,
        // and both tables are present on every kernel.
        assert!(info.page_block_order > 0, "Page block order should be > 0");
        assert!(info.pages_per_block > 0, "Pages per block should be > 0");
        assert!(!info.free.is_empty(), "Should have free-list rows");
        assert!(!info.blocks.is_empty(), "Should have block-count rows");

        // A page block covers 1 << page_block_order pages.
        assert_eq!(
            info.pages_per_block,
            1u64 << info.page_block_order,
            "pages_per_block should equal 1 << page_block_order"
        );
    }

    #[test]
    fn test_live_pagetypeinfo_free() {
        let Some(info) = pagetypeinfo_or_skip() else {
            return;
        };

        // Every free-list row names a zone and migrate type, carries a
        // consistent number of orders (MAX_ORDER), and the zone field
        // matches one of the block-count rows.
        let orders = info.free[0].orders.len();
        assert!(orders > 0, "Should have at least one order");
        for f in &info.free {
            assert!(!f.zone.is_empty(), "Zone name should not be empty");
            assert!(
                !f.migrate_type.is_empty(),
                "Migrate type should not be empty"
            );
            assert_eq!(
                f.orders.len(),
                orders,
                "{} {} free-list length should match",
                f.zone,
                f.migrate_type
            );
            assert!(
                info.blocks.iter().any(|b| b.zone == f.zone),
                "{} should appear in block counts",
                f.zone
            );
        }
    }

    #[test]
    fn test_live_pagetypeinfo_blocks() {
        let Some(info) = pagetypeinfo_or_skip() else {
            return;
        };

        // Every block-count row has a consistent number of columns,
        // matching the migrate-type header (varies by kernel config).
        let types = info.blocks[0].types.len();
        assert!(types > 0, "Should have at least one migrate type");
        for b in &info.blocks {
            assert!(!b.zone.is_empty(), "Zone name should not be empty");
            assert_eq!(
                b.types.len(),
                types,
                "{} block-count length should match",
                b.zone
            );
        }
    }

    #[test]
    fn test_live_softirqs() {
        let si = softirqs().expect("Failed to read /proc/softirqs");

        // The header lists the online CPUs, and every kernel registers
        // at least the classic softirq types below.
        assert!(!si.cpus.is_empty(), "Should have at least one CPU");
        assert!(!si.rows.is_empty(), "Should have at least one softirq");
        for key in ["HI", "TIMER", "NET_TX", "NET_RX", "BLOCK", "SCHED", "RCU"] {
            assert!(
                si.rows.iter().any(|r| r.name.as_ref() == key),
                "softirqs should contain {key}"
            );
        }
    }

    #[test]
    fn test_live_softirqs_fields() {
        let si = softirqs().expect("Failed to read /proc/softirqs");

        // Names are whitespace-free tokens without the trailing colon,
        // and each row carries one counter per listed CPU.
        for r in &si.rows {
            assert!(!r.name.is_empty(), "Softirq name should not be empty");
            assert!(
                !r.name.contains(':') && !r.name.contains(char::is_whitespace),
                "Softirq name should be a bare token"
            );
            assert_eq!(
                r.per_cpu.len(),
                si.cpus.len(),
                "{} should have one count per CPU",
                r.name
            );
        }

        // Header CPU ids are unique and ascending.
        for pair in si.cpus.windows(2) {
            assert!(pair[0] < pair[1], "CPU ids should ascend");
        }

        // Timers fire constantly on any running system.
        let timer = si
            .rows
            .iter()
            .find(|r| r.name.as_ref() == "TIMER")
            .expect("TIMER softirq should exist");
        assert!(
            timer.per_cpu.iter().sum::<u64>() > 0,
            "TIMER counts should be > 0"
        );
    }

    #[test]
    fn test_live_softirqs_monotonic() {
        let first = softirqs().expect("Failed to read /proc/softirqs");
        let second = softirqs().expect("Failed to re-read /proc/softirqs");

        // Counters are cumulative since boot, so a second sample can
        // only stay equal or grow. Rows keep file order, so match by
        // position.
        assert_eq!(
            first.rows.len(),
            second.rows.len(),
            "row count should match"
        );
        for (a, b) in first.rows.iter().zip(&second.rows) {
            assert_eq!(a.name, b.name, "row order should be stable");
            for (&x, &y) in a.per_cpu.iter().zip(&b.per_cpu) {
                assert!(y >= x, "{} counter went backwards: {} -> {}", a.name, x, y);
            }
        }
    }

    #[test]
    fn test_live_interrupts() {
        let irq = interrupts().expect("Failed to read /proc/interrupts");

        // Every running system has at least a timer IRQ and the classic
        // x86 aggregate counters.
        assert!(!irq.cpus.is_empty(), "Should have at least one CPU");
        assert!(!irq.rows.is_empty(), "Should have at least one IRQ row");
        assert!(
            !irq.counts.is_empty(),
            "Should have at least one aggregate counter"
        );
        for key in ["NMI", "LOC", "RES", "CAL", "TLB"] {
            assert!(
                irq.counts.iter().any(|c| c.name.as_ref() == key),
                "interrupts should contain aggregate {key}"
            );
        }
    }

    #[test]
    fn test_live_interrupts_fields() {
        let irq = interrupts().expect("Failed to read /proc/interrupts");

        // Every IRQ row has one count per listed CPU and non-empty info.
        for r in &irq.rows {
            assert_eq!(
                r.per_cpu.len(),
                irq.cpus.len(),
                "IRQ {} should have one count per CPU",
                r.irq
            );
            assert!(!r.info.is_empty(), "IRQ {} should have info", r.irq);
        }

        // Aggregate count names are bare tokens without a colon.
        for c in &irq.counts {
            assert!(!c.name.is_empty(), "Aggregate name should not be empty");
            assert!(
                !c.name.contains(':'),
                "Aggregate name should not contain ':'"
            );
            // Values are either per-CPU or a single system-wide total.
            assert!(
                c.values.len() == 1 || c.values.len() == irq.cpus.len(),
                "{} should have per-cpu or single total values",
                c.name
            );
        }
    }

    #[test]
    fn test_live_interrupts_monotonic() {
        let first = interrupts().expect("Failed to read /proc/interrupts");
        let second = interrupts().expect("Failed to re-read /proc/interrupts");

        // Counters are cumulative since boot, so a second sample can
        // only stay equal or grow.
        assert_eq!(
            first.rows.len(),
            second.rows.len(),
            "IRQ row count should match"
        );
        for (a, b) in first.rows.iter().zip(&second.rows) {
            assert_eq!(a.irq, b.irq, "IRQ numbers should be stable");
            for (&x, &y) in a.per_cpu.iter().zip(&b.per_cpu) {
                assert!(
                    y >= x,
                    "IRQ {} counter went backwards: {} -> {}",
                    a.irq,
                    x,
                    y
                );
            }
        }
        for (a, b) in first.counts.iter().zip(&second.counts) {
            assert_eq!(a.name, b.name, "Aggregate names should be stable");
            for (&x, &y) in a.values.iter().zip(&b.values) {
                assert!(y >= x, "{} counter went backwards: {} -> {}", a.name, x, y);
            }
        }
    }

    #[test]
    fn test_live_iomem() {
        let regions = iomem().expect("Failed to read /proc/iomem");
        assert!(
            !regions.is_empty(),
            "Should have at least one memory region"
        );
    }

    #[test]
    fn test_live_iomem_fields() {
        let regions = iomem().expect("Failed to read /proc/iomem");

        // Every entry spans a valid range with a non-empty description.
        for r in &regions {
            assert!(
                r.start <= r.end,
                "Start address {:x} should be <= end address {:x}",
                r.start,
                r.end,
            );
            assert!(!r.name.is_empty(), "Region name should not be empty");
        }
    }

    #[test]
    fn test_live_iomem_system_ram() {
        let regions = iomem().expect("Failed to read /proc/iomem");

        // Every running system has System RAM and at least one root-level entry.
        assert!(
            regions.iter().any(|r| r.name.as_ref() == "System RAM"),
            "Should have at least one System RAM region"
        );
        assert!(
            regions.iter().any(|r| r.depth == 0),
            "Should have at least one root-level entry"
        );
    }

    #[test]
    fn test_live_ioports() {
        let regions = ioports().expect("Failed to read /proc/ioports");
        assert!(
            !regions.is_empty(),
            "Should have at least one I/O port region"
        );
    }

    #[test]
    fn test_live_ioports_fields() {
        let regions = ioports().expect("Failed to read /proc/ioports");

        // Every entry spans a valid range with a non-empty description.
        for r in &regions {
            assert!(
                r.start <= r.end,
                "Port start {} should be <= end {}",
                r.start,
                r.end,
            );
            assert!(!r.name.is_empty(), "Port region name should not be empty");
        }
    }

    #[test]
    fn test_live_ioports_depth() {
        let regions = ioports().expect("Failed to read /proc/ioports");

        // The file always has at least one root-level entry.
        assert!(
            regions.iter().any(|r| r.depth == 0),
            "Should have at least one root-level entry"
        );

        // Classic ISA devices are always present on x86 systems.
        assert!(
            regions.iter().any(|r| r.name.as_ref() == "dma1"),
            "Should have a dma1 entry"
        );
        assert!(
            regions.iter().any(|r| r.name.as_ref() == "pic1"),
            "Should have a pic1 entry"
        );
    }

    #[test]
    fn test_live_misc() {
        let devices = misc().expect("Failed to read /proc/misc");
        assert!(!devices.is_empty(), "Should have at least one misc device");
    }

    #[test]
    fn test_live_misc_fields() {
        let devices = misc().expect("Failed to read /proc/misc");

        // Every entry has a valid minor number and a non-empty name.
        for d in &devices {
            assert!(!d.name.is_empty(), "Misc device name should not be empty");
            let _ = d.minor;
        }
    }

    #[test]
    fn test_live_misc_known() {
        let devices = misc().expect("Failed to read /proc/misc");

        // fuse is registered on every modern kernel.
        assert!(
            devices.iter().any(|d| d.name.as_ref() == "fuse"),
            "Should have a fuse misc device"
        );
    }

    #[test]
    fn test_live_modules() {
        let mods = modules().expect("Failed to read /proc/modules");
        assert!(!mods.is_empty(), "Should have at least one loaded module");
    }

    #[test]
    fn test_live_modules_fields() {
        let mods = modules().expect("Failed to read /proc/modules");

        // Every entry has a non-empty name, positive size, and valid state.
        for m in &mods {
            assert!(!m.name.is_empty(), "Module name should not be empty");
            assert!(m.size > 0, "Module size should be > 0");
            assert!(
                m.state.as_ref() == "Live"
                    || m.state.as_ref() == "Loading"
                    || m.state.as_ref() == "Unloading",
                "Module state should be Live, Loading, or Unloading"
            );
            let _ = m.ref_count;
            let _ = m.address;
        }
    }

    #[test]
    fn test_live_modules_live() {
        let mods = modules().expect("Failed to read /proc/modules");

        // At least one module must be live with a dependency list.
        assert!(
            mods.iter().any(|m| m.state.as_ref() == "Live"),
            "Should have at least one live module"
        );
        assert!(
            mods.iter().any(|m| m.deps.as_ref() != "-"),
            "Should have at least one module with dependencies"
        );
    }

    #[test]
    fn test_live_crypto() {
        let entries = crypto().expect("Failed to read /proc/crypto");
        assert!(!entries.is_empty(), "Should have at least one crypto entry");
    }

    #[test]
    fn test_live_crypto_fields() {
        let entries = crypto().expect("Failed to read /proc/crypto");
        for entry in &entries {
            assert!(!entry.name.is_empty(), "Crypto name should not be empty");
            assert!(
                !entry.driver.is_empty(),
                "Crypto driver should not be empty"
            );
            assert!(
                !entry.crypto_type.is_empty(),
                "Crypto type should not be empty"
            );
            let _ = entry.priority;
            let _ = entry.ref_count;
        }
    }

    #[test]
    fn test_live_crypto_self_tests() {
        let entries = crypto().expect("Failed to read /proc/crypto");
        assert!(
            entries
                .iter()
                .any(|entry| entry.self_test.as_ref() == "passed"),
            "Should have at least one passed crypto self-test"
        );
    }

    #[test]
    fn test_live_consoles() {
        let cons = consoles().expect("Failed to read /proc/consoles");

        // The kernel lists every registered console; a headless system may
        // have no tty, so we only require the list to be non-empty.
        assert!(!cons.is_empty(), "Should have at least one console");
    }

    #[test]
    fn test_live_consoles_fields() {
        let cons = consoles().expect("Failed to read /proc/consoles");

        // Every console has a non-empty name and a device when one exists.
        for c in &cons {
            assert!(!c.name.is_empty(), "Console name should not be empty");
            match (c.major, c.minor) {
                (Some(major), Some(minor)) => {
                    assert!(major > 0, "Console major should be > 0");
                    let _ = minor;
                }
                (None, None) => {}
                _ => panic!("Console major/minor should both be set or both absent"),
            }
        }
    }

    #[test]
    fn test_live_consoles_flags() {
        let cons = consoles().expect("Failed to read /proc/consoles");

        // The kernel lists every registered console (enabled or not) with an
        // indeterminate flag set, so we only check that the decoded flags are
        // a valid subset of the known bits — i.e. the parser didn't mangle the
        // R/W/U and state flags.
        for c in &cons {
            let unknown = c.flags.bits() & !ConsoleFlags::all().bits();
            assert!(
                unknown == 0,
                "Console flags contain unknown bits: {unknown:#b}"
            );
        }
    }

    #[test]
    fn test_live_locks() {
        let lock_list = locks().expect("Failed to read /proc/locks");

        // Locks may be absent on a quiet system, so don't require any.
        for l in &lock_list {
            assert!(l.pid != 0, "Lock PID should not be 0");
            assert!(l.inode > 0, "Lock inode should be > 0");
        }
    }

    #[test]
    fn test_live_locks_fields() {
        let lock_list = locks().expect("Failed to read /proc/locks");

        for l in &lock_list {
            match l.lock_type {
                procfs2::proc::LockType::Flock
                | procfs2::proc::LockType::Ofdlck
                | procfs2::proc::LockType::Posix => {}
            }
            match l.class {
                procfs2::proc::LockClass::Advisory | procfs2::proc::LockClass::Mandatory => {}
            }
            match l.access {
                procfs2::proc::LockAccess::Read | procfs2::proc::LockAccess::Write => {}
            }
            let _ = l.start;
            let _ = l.end;
        }
    }

    #[test]
    fn test_live_locks_no_zero_pid() {
        let lock_list = locks().expect("Failed to read /proc/locks");

        // OFD locks show -1, POSIX/FLOCK show a positive PID.
        // No lock should have pid == 0.
        for l in &lock_list {
            assert!(
                l.pid != 0,
                "Lock PID should be non-zero (got 0 for {:?})",
                l.lock_type,
            );
        }
    }

    #[test]
    fn test_live_cpu_pressure() {
        let psi = cpu_pressure().expect("Failed to read /proc/pressure/cpu");

        // Averages are percentages in [0, 100]; total is microseconds.
        assert!(psi.some.avg10 >= 0.0, "avg10 should be >= 0");
        assert!(psi.full.avg10 >= 0.0, "full avg10 should be >= 0");
        let _ = psi.some.total;
    }

    #[test]
    fn test_live_memory_and_io_pressure() {
        let mem = memory_pressure().expect("Failed to read /proc/pressure/memory");
        let io = io_pressure().expect("Failed to read /proc/pressure/io");

        // Both files report `some` and `full` lines.
        assert!(mem.some.avg10 >= 0.0);
        assert!(mem.full.avg10 >= 0.0);
        assert!(io.some.avg10 >= 0.0);
        assert!(io.full.avg10 >= 0.0);
    }

    #[test]
    fn test_live_irq_pressure() {
        let psi = match irq_pressure() {
            Ok(psi) => psi,
            // /proc/pressure/irq is absent on some kernels / CI runners.
            Err(procfs2::Error::Io { error: e, .. })
                if e.kind() == std::io::ErrorKind::NotFound =>
            {
                return;
            }
            Err(e) => panic!("Failed to read /proc/pressure/irq: {e}"),
        };

        // IRQ only reports `full`; averages are non-negative percentages.
        assert!(psi.full.avg10 >= 0.0, "avg10 should be >= 0");
        assert!(psi.full.avg60 >= 0.0, "avg60 should be >= 0");
        assert!(psi.full.avg300 >= 0.0, "avg300 should be >= 0");
        let _ = psi.full.total;
    }

    #[test]
    fn test_live_net_snmp() {
        let stats = procfs2::proc::net::snmp().expect("Failed to read /proc/net/snmp");
        assert!(stats.ip.default_ttl > 0, "IP default TTL should be > 0");
        assert!(stats.tcp.rto_min > 0, "TCP minimum RTO should be > 0");
    }

    #[test]
    fn test_live_net_snmp_counters() {
        let stats = procfs2::proc::net::snmp().expect("Failed to read /proc/net/snmp");
        assert!(stats.ip.in_receives >= stats.ip.in_delivers);
        assert!(stats.tcp.in_segs > 0, "TCP input segments should be > 0");
        assert!(
            stats.udp.out_datagrams > 0,
            "UDP output datagrams should be > 0"
        );
    }

    #[test]
    fn test_live_net_snmp_consistent() {
        let first = procfs2::proc::net::snmp().expect("Failed to read /proc/net/snmp");
        let second = procfs2::proc::net::snmp().expect("Failed to reread /proc/net/snmp");
        assert!(second.ip.in_receives >= first.ip.in_receives);
        assert!(second.tcp.in_segs >= first.tcp.in_segs);
        assert!(second.udp.in_datagrams >= first.udp.in_datagrams);
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
