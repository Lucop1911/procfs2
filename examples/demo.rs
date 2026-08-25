use procfs2::proc;
use procfs2::sys;

fn main() {
    println!("=== uptime ===");
    let uptime = proc::uptime().unwrap();
    println!("Total: {:?}, Idle: {:?}", uptime.total, uptime.idle);

    println!("\n=== loadavg ===");
    let load = proc::loadavg().unwrap();
    println!("Load: {:.2} {:.2} {:.2}", load.one, load.five, load.fifteen);
    println!("Runnable: {}/{}", load.runnable, load.total);

    println!("\n=== meminfo ===");
    let mem = proc::meminfo().unwrap();
    println!("Total: {} kB", mem.total.0);
    println!("Free: {} kB", mem.free.0);
    println!("Available: {} kB", mem.available.0);

    println!("\n=== /proc/cmdline ===");
    let cmdline = proc::cmdline().unwrap();
    for (i, arg) in cmdline.iter().enumerate() {
        println!("  {}: {}", i, arg);
    }

    println!("\n=== /proc/self/stat ===");
    let me = proc::Process::current().unwrap();
    let stat = me.stat().unwrap();
    println!("PID: {}", stat.pid);
    println!("Comm: {}", stat.comm);
    println!("State: {}", stat.state);
    println!("PPID: {}", stat.ppid);
    println!("Threads: {}", stat.num_threads);
    println!("UTime: {} jiffies", stat.utime);
    println!("STime: {} jiffies", stat.stime);

    println!("\n=== /proc/self/status ===");
    let status = me.status().unwrap();
    println!("Name: {}", status.name);
    println!("State: {:?}", status.state);
    println!(
        "UID: real={}, effective={}",
        status.uid.real, status.uid.effective
    );
    println!("VmRSS: {} kB", status.vm_rss.0);
    println!("Threads: {}", status.threads);

    println!("\n=== /proc/self/cmdline ===");
    let cmdline = me.cmdline().unwrap();
    println!("Args: {:?}", cmdline);

    println!("\n=== /proc/self/environ (first 5) ===");
    let environ = me.environ().unwrap();
    for (key, val) in environ.iter().take(5) {
        println!("  {}={}", key.to_string_lossy(), val.to_string_lossy());
    }

    println!("\n=== /proc/self/exe / cwd ===");
    println!("exe: {:?}", me.exe().unwrap());
    println!("cwd: {:?}", me.cwd().unwrap());

    println!("\n=== /proc/self/smaps_rollup ===");
    let rollup = me.smaps_rollup().unwrap();
    println!("RSS: {} kB", rollup.rss_kb);
    println!("PSS: {} kB", rollup.pss_kb);
    println!("Swap: {} kB", rollup.swap_kb);

    println!("\n=== /proc/self/maps (first 5) ===");
    let maps = me.maps().unwrap();
    for map in maps.iter().take(5) {
        println!(
            "  {:016x}-{:016x} {:?} {:?}",
            map.address.start, map.address.end, map.perms, map.pathname
        );
    }

    println!("\n=== /proc/self/io ===");
    let io = me.io().unwrap();
    println!("rchar: {} bytes", io.rchar.0);
    println!("wchar: {} bytes", io.wchar.0);
    println!("read_bytes: {} bytes", io.read_bytes.0);
    println!("write_bytes: {} bytes", io.write_bytes.0);

    println!("\n=== /proc/self/fd (count) ===");
    let fds = me.fds().unwrap();
    println!("Open FDs: {}", fds.len());

    println!("\n=== /proc/self/limits (first 4) ===");
    let limits = me.limits().unwrap();
    let limit_rows = [
        ("CPU time", &limits.max_cpu_time),
        ("File size", &limits.max_fsize),
        ("Stack", &limits.max_stack),
        ("Open files", &limits.max_open_files),
    ];
    for (name, limit) in limit_rows {
        let soft = limit
            .soft
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".into());
        let hard = limit
            .hard
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".into());
        println!("  {}: soft={} hard={} {:?}", name, soft, hard, limit.unit);
    }

    println!("\n=== /proc/self/task (threads) ===");
    let thread_count = me.threads().filter(|r| r.is_ok()).count();
    println!("Threads: {}", thread_count);

    println!("\n=== /proc/self/cgroup ===");
    let cgroups = me.cgroup().unwrap();
    for cg in &cgroups {
        println!(
            "  hierarchy={} controllers={:?} path={:?}",
            cg.hierarchy, cg.controllers, cg.path
        );
    }

    println!("\n=== /proc/self/mountinfo (first 3) ===");
    let mounts = me.mountinfo().unwrap();
    for m in mounts.iter().take(3) {
        println!(
            "  {} on {} type {}",
            m.source, m.mount_point, m.filesystem_type
        );
    }

    println!("\n=== /proc/self/namespaces ===");
    let ns = me.namespaces().unwrap();
    println!("  mnt: {:?}", ns.mnt);
    println!("  pid: {:?}", ns.pid);
    println!("  net: {:?}", ns.net);
    println!("  uts: {:?}", ns.uts);

    println!("\n=== /proc/stat (CPU) ===");
    let sys_stat = proc::stat().unwrap();
    println!("Context switches: {}", sys_stat.ctxt);
    println!("Boot time: {}", sys_stat.btime);
    println!("CPU total user: {} jiffies", sys_stat.cpu_total.user.0);
    println!("Per-CPU count: {}", sys_stat.per_cpu.len());

    println!("\n=== /proc/cpuinfo (first core) ===");
    let cpus = proc::cpuinfo().unwrap();
    if let Some(cpu) = cpus.first() {
        println!("Model: {}", cpu.model_name);
        println!("MHz: {:.2}", cpu.cpu_mhz);
        println!("Flags count: {}", cpu.flags.len());
    }

    println!("\n=== kernel version ===");
    let ver = proc::version().unwrap();
    println!("{}.{}.{}", ver.major, ver.minor, ver.patch);

    println!("\n=== /proc/mounts (first 5) ===");
    let all_mounts = proc::mounts().unwrap();
    for m in all_mounts.iter().take(5) {
        println!("  {} on {} type {}", m.spec, m.file, m.vfstype);
    }

    println!("\n=== /proc/cgroups ===");
    let cg_stats = proc::cgroups().unwrap();
    for cg in &cg_stats {
        println!(
            "  {} hierarchy={} cgroups={} enabled={}",
            cg.subsys_name, cg.hierarchy, cg.num_cgroups, cg.enabled
        );
    }

    println!("\n=== /proc/devices ===");
    let devices = proc::devices().unwrap();
    for dev in &devices {
        println!("  [{:3}] {:?}: {}", dev.major, dev.kind, dev.name);
    }

    println!("\n=== /proc/filesystems ===");
    let fs_list = proc::filesystems().unwrap();
    for fs in &fs_list {
        println!("  [{}] {}", if fs.dev { "dev" } else { "nodev" }, fs.name);
    }

    println!("\n=== /proc/swaps ===");
    let swaps = proc::swaps().unwrap();
    if swaps.is_empty() {
        println!("  (none configured)");
    }
    for s in &swaps {
        println!(
            "  {:?}: {} used / {} KiB, priority {}",
            s.kind, s.used.0, s.size.0, s.priority
        );
    }

    println!("\n=== /proc/partitions ===");
    let partitions = proc::partitions().unwrap();
    for p in &partitions {
        println!(
            "  {:3}:{:3} {:>10} KiB {}",
            p.major, p.minor, p.blocks.0, p.name
        );
    }

    println!("\n=== /proc/diskstats ===");
    let diskstats = proc::diskstats().unwrap();
    for s in &diskstats {
        println!("  {:3}:{:3} {}", s.major, s.minor, s.name);
        println!(
            "    reads: {} completed, {} merged, {} sectors, {} ms",
            s.reads_completed, s.reads_merged, s.sectors_read, s.time_reading.0
        );
        println!(
            "    writes: {} completed, {} merged, {} sectors, {} ms",
            s.writes_completed, s.writes_merged, s.sectors_written, s.time_writing.0
        );
        println!(
            "    I/O: {} in flight, {} ms, {} ms weighted",
            s.io_in_progress, s.time_io.0, s.weighted_time_io.0
        );
        println!(
            "    discards: {} completed, {} merged, {} sectors, {} ms",
            s.discards_completed, s.discards_merged, s.sectors_discarded, s.time_discarding.0
        );
        println!(
            "    flush: {} completed, {} ms",
            s.flush_completed, s.time_flushing.0
        );
    }

    println!("\n=== /proc/vmstat (notable counters) ===");
    let vmstat = proc::vmstat().unwrap();
    println!("Total counters: {}", vmstat.len());
    for key in [
        "nr_free_pages",
        "nr_dirty",
        "pgpgin",
        "pgpgout",
        "pswpin",
        "pswpout",
        "oom_kill",
    ] {
        if let Some(value) = vmstat.get(key) {
            println!("  {}: {}", key, value);
        }
    }

    println!("\n=== /proc/zoneinfo ===");
    let zones = proc::zoneinfo().unwrap();
    for z in &zones {
        println!(
            "  node {} zone {:8} free={:<10} present={:<10} managed={:<10} stats={} pagesets={} protection={:?}",
            z.node,
            z.name,
            z.pages.free,
            z.pages.present,
            z.pages.managed,
            z.stats.len(),
            z.pagesets.len(),
            z.protection
        );
        if z.name.as_ref() == "Normal" {
            println!(
                "    watermarks: min={} low={} high={} promo={}",
                z.pages.min, z.pages.low, z.pages.high, z.pages.promo
            );
            if let Some(hit) = z.stats.get("numa_hit") {
                println!("    numa_hit: {}", hit);
            }
        }
    }

    println!("\n=== /proc/buddyinfo ===");
    let buddy = proc::buddyinfo().unwrap();
    for b in &buddy {
        let total: u64 = b
            .free_lists
            .iter()
            .enumerate()
            .map(|(order, &count)| count << order)
            .sum();
        println!(
            "  node {} zone {:8} orders={} total_free={}",
            b.node,
            b.zone,
            b.free_lists.len(),
            total
        );
        println!("    free_lists: {:?}", b.free_lists);
    }

    println!("\n=== /proc/pagetypeinfo ===");
    match proc::pagetypeinfo() {
        Ok(info) => {
            println!(
                "  page block order: {} ({} pages per block)",
                info.page_block_order, info.pages_per_block
            );
            println!("  free-list rows: {}", info.free.len());
            println!("  block-count rows: {}", info.blocks.len());
            let orders = info.free.first().map(|f| f.orders.len()).unwrap_or(0);
            println!("  orders per row: {}", orders);
            for f in info.free.iter().take(6) {
                let total: u64 = f
                    .orders
                    .iter()
                    .enumerate()
                    .map(|(order, &count)| count << order)
                    .sum();
                println!(
                    "  node {} zone {:8} type {:12} free_pages={}",
                    f.node, f.zone, f.migrate_type, total
                );
            }
            if info.free.len() > 6 {
                println!("  ... ({} more)", info.free.len() - 6);
            }
        }
        Err(e) => println!("  (skipped: {e})"),
    }

    println!("\n=== /proc/softirqs ===");
    let softirqs = proc::softirqs().unwrap();
    println!("  CPUs: {:?}", softirqs.cpus);
    for row in &softirqs.rows {
        let total: u64 = row.per_cpu.iter().sum();
        println!(
            "  {:10} total={:<12} per_cpu={:?}",
            row.name, total, row.per_cpu
        );
    }

    println!("\n=== /proc/interrupts ===");
    let irq = proc::interrupts().unwrap();
    println!("  CPUs: {:?}", irq.cpus);
    let mut irq_rows: Vec<_> = irq.rows.iter().collect();
    irq_rows.sort_by_key(|r| std::cmp::Reverse(r.per_cpu.iter().sum::<u64>()));
    for r in irq_rows.iter().take(10) {
        let total: u64 = r.per_cpu.iter().sum();
        println!("  {:>4}: {:>12}  {}", r.irq, total, r.info);
    }
    if irq.rows.len() > 10 {
        println!("  ... ({} more)", irq.rows.len() - 10);
    }
    for c in &irq.counts {
        let total: u64 = c.values.iter().sum();
        println!("  {:>4}: {:>12}  {}", c.name, total, c.description);
    }

    println!("\n=== /proc/iomem (first 10) ===");
    let iomem = proc::iomem().unwrap();
    let indent = |depth: usize| "  ".repeat(depth);
    for r in iomem.iter().take(10) {
        println!(
            "  {}{:08x}-{:08x} : {}",
            indent(r.depth),
            r.start,
            r.end,
            r.name
        );
    }
    if iomem.len() > 10 {
        println!("  ... ({} more)", iomem.len() - 10);
    }

    println!("\n=== /proc/net/tcp ===");
    let tcp_conns: Vec<_> = proc::net::tcp().collect();
    println!("Active TCP connections: {}", tcp_conns.len());

    println!("\n=== /proc/net/tcp6 (count) ===");
    let tcp6_conns: Vec<_> = proc::net::tcp6().collect();
    println!("Active TCP6 connections: {}", tcp6_conns.len());

    println!("\n=== /proc/net/udp (count) ===");
    let udp_socks: Vec<_> = proc::net::udp().collect();
    println!("UDP sockets: {}", udp_socks.len());

    println!("\n=== /proc/net/udp6 (count) ===");
    let udp6_socks: Vec<_> = proc::net::udp6().collect();
    println!("UDP6 sockets: {}", udp6_socks.len());

    println!("\n=== /proc/net/dev ===");
    for dev in proc::net::dev().filter_map(|r| r.ok()) {
        println!(
            "  {}: RX {} bytes, TX {} bytes",
            dev.name, dev.rx_bytes.0, dev.tx_bytes.0
        );
    }

    println!("\n=== /proc/net/arp ===");
    let arp_entries: Vec<_> = proc::net::arp().filter_map(|r| r.ok()).collect();
    println!("ARP entries: {}", arp_entries.len());

    println!("\n=== /proc/net/route ===");
    for route in proc::net::route().filter_map(|r| r.ok()) {
        println!(
            "  {} -> {} via {} (flags=0x{:x})",
            route.iface, route.destination, route.gateway, route.flags
        );
    }

    println!("\n=== /proc/net/unix (count) ===");
    let unix_socks: Vec<_> = proc::net::unix().filter_map(|r| r.ok()).collect();
    println!("Unix sockets: {}", unix_socks.len());

    println!("\n=== /sys/block ===");
    for dev in sys::BlockDevice::all().filter_map(|r| r.ok()) {
        println!("  {}", dev.name);
        if let Ok(stat) = dev.stat() {
            println!(
                "    reads={} writes={} sectors_read={} sectors_written={}",
                stat.reads_completed,
                stat.writes_completed,
                stat.sectors_read,
                stat.sectors_written
            );
        }
        if let Ok(size) = dev.size() {
            println!("    size: {} bytes ({} GiB)", size.0, size.as_gib());
        }
        if let Ok(params) = dev.queue_params() {
            println!(
                "    scheduler={:?} rotational={} read_ahead={}kB",
                params.scheduler, params.rotational, params.read_ahead_kb
            );
        }
    }

    println!("\n=== /sys/class/net ===");
    for iface in sys::NetInterface::all().filter_map(|r| r.ok()) {
        println!("  {}:", iface.name);
        if let Ok(stats) = iface.stats() {
            println!(
                "    RX: {} bytes, {} pkts, {} err, {} drop",
                stats.rx_bytes.0, stats.rx_packets, stats.rx_errors, stats.rx_drop
            );
            println!(
                "    TX: {} bytes, {} pkts, {} err, {} drop",
                stats.tx_bytes.0, stats.tx_packets, stats.tx_errors, stats.tx_drop
            );
        }
        if let Ok(state) = iface.operstate() {
            println!("    operstate: {:?}", state);
        }
        if let Ok(mac) = iface.address() {
            println!(
                "    mac: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]
            );
        }
        if let Ok(mtu) = iface.mtu() {
            println!("    mtu: {}", mtu);
        }
        if let Ok(flags) = iface.flags() {
            println!("    flags: {:?}", flags);
        }
        if let Ok(speed) = iface.speed() {
            println!("    speed: {:?} Mbps", speed);
        }
    }

    println!("\n=== /sys/class/power_supply ===");
    let supplies: Vec<_> = sys::PowerSupply::all().filter_map(|r| r.ok()).collect();
    if supplies.is_empty() {
        println!("  (none found)");
    }
    for ps in &supplies {
        println!("  {}:", ps.name);
        if let Ok(kind) = ps.kind() {
            println!("    type: {:?}", kind);
        }
        if let Ok(status) = ps.status() {
            println!("    status: {:?}", status);
        }
        if let Ok(capacity) = ps.capacity() {
            println!("    capacity: {:?}%", capacity);
        }
        if let Ok(voltage) = ps.voltage_now() {
            println!("    voltage: {:?} uV", voltage);
        }
        if let Ok(current) = ps.current_now() {
            println!("    current: {:?} uA", current);
        }
    }

    println!("\n=== /sys/devices/system/cpu ===");
    if let Ok(count) = sys::cpu_count() {
        println!("  CPU count: {}", count);
    }
    if let Ok(cpus) = sys::online_cpus() {
        println!("  Online CPUs: {:?}", cpus);
    }
    for cpu_id in 0..2 {
        if let Ok(freq) = sys::cpu_freq(cpu_id) {
            println!(
                "  CPU {}: {} MHz (min: {} MHz, max: {} MHz, gov: {})",
                freq.cpu,
                freq.current_khz / 1000,
                freq.min_khz / 1000,
                freq.max_khz / 1000,
                freq.governor
            );
        }
    }
}
