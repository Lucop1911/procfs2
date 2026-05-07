# procfs2

A modern, zero-copy, strongly-typed Rust library for reading Linux's `/proc` and `/sys` virtual filesystems.

[![Rust](https://github.com/lucop1911/procfs2/actions/workflows/rust.yml/badge.svg)](https://github.com/lucop1911/procfs2/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/procfs2.svg)](https://crates.io/crates/procfs2)
[![Documentation](https://docs.rs/procfs2/badge.svg)](https://docs.rs/procfs2)
[![License: MIT](https://img.shields.io/crates/l/procfs2.svg)](https://opensource.org/licenses/MIT)

## Features

- **Typed API** — Every kernel file maps to a concrete Rust struct or enum
- **Zero-copy where possible** — Parse directly from `&[u8]` slices
- **Complete `/proc` coverage** — System-wide, per-process, network, cgroups
- **First-class `/sys` support** — Block devices, network interfaces, power supply, CPU info
- **Async-friendly** — Optional tokio-backed async variants
- **Runtime-safe** — No panics; all errors returned as typed `Error` variants

## Quick Start

```rust
use procfs2::{proc, sys, Process};

fn main() -> procfs2::Result<()> {
    // System-wide statistics
    let mem = proc::meminfo()?;
    println!("Total memory: {} KB", mem.total.0);
    println!("Available: {} KB", mem.available.0);

    let load = proc::loadavg()?;
    println!("Load: {:.2} {:.2} {:.2}", load.one, load.five, load.fifteen);

    // Current process
    let me = Process::current()?;
    let status = me.status()?;
    println!("Process: {} ({:?})", status.name, status.state);
    println!("UID: real={}, effective={}", status.uid.real, status.uid.effective);

    // Memory mappings
    for map in me.maps()? {
        println!("{:016x}-{:016x} {:?} {:?}",
            map.address.start, map.address.end, map.perms, map.pathname);
    }

    // Network connections
    for conn in proc::net::tcp().filter_map(|r| r.ok()) {
        println!("{} -> {} [{:?}]", conn.local, conn.remote, conn.state);
    }

    // System devices
    for dev in sys::BlockDevice::all().filter_map(|r| r.ok()) {
        println!("Block device: {}", dev.name);
        if let Ok(stat) = dev.stat() {
            println!("  reads={}, writes={}", stat.reads_completed, stat.writes_completed);
        }
    }

    // Network interfaces
    for iface in sys::NetInterface::all().filter_map(|r| r.ok()) {
        println!("Interface: {}", iface.name);
        if let Ok(stats) = iface.stats() {
            println!("  RX: {} bytes, TX: {} bytes", stats.rx_bytes.0, stats.tx_bytes.0);
        }
    }

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
procfs2 = "0.2"
```

### Feature Flags

Enable additional features as needed:

```toml
[dependencies.procfs2]
version = "0.2"
features = ["async", "serde", "macros", "watch"]
```

| Feature | Description | Extra Dependencies |
|---------|-------------|-------------------|
| `async` | Async read variants + async inotify | `tokio` |
| `watch` | `inotify` watcher API | `libc` |
| `serde` | `Serialize`/`Deserialize` on all structs | `serde` |
| `macros` | `#[derive(ProcKeyValue)]` proc-macro | `procfs2-macros` |

## Module Overview

### `/proc` Module

#### System-wide
- `proc::uptime()` — `Uptime { total, idle }` from `/proc/uptime`
- `proc::loadavg()` — `LoadAvg { one, five, fifteen, runnable, total }` from `/proc/loadavg`
- `proc::meminfo()` — `MemInfo { total, free, available, ... }` from `/proc/meminfo`
- `proc::stat()` — `SystemStat { cpu_total, per_cpu, ctxt, btime, ... }` from `/proc/stat`
- `proc::cpuinfo()` — `Vec<CpuCore>` from `/proc/cpuinfo`
- `proc::version()` — `KernelVersion` from `/proc/version`
- `proc::mounts()` — `Vec<MountEntry>` from `/proc/mounts`
- `proc::cgoups()` — `Vec<CgroupStat>` from `/proc/cgroups`

#### Per-Process (`Process`)
```rust
let process = Process::new(pid)?;
let process = Process::current()?;
let all_procs = Process::all(); // Iterator over all PIDs

// Methods
process.stat()?;       // /proc/PID/stat
process.status()?;     // /proc/PID/status
process.cmdline()?;    // /proc/PID/cmdline
process.environ()?;     // /proc/PID/environ
process.exe()?;         // /proc/PID/exe
process.cwd()?;         // /proc/PID/cwd
process.maps()?;        // /proc/PID/maps
process.smaps()?;       // /proc/PID/smaps
process.fds()?;         // /proc/PID/fd/
process.io()?;          // /proc/PID/io
process.limits()?;      // /proc/PID/limits
process.mountinfo()?;   // /proc/PID/mountinfo
process.cgroup()?;      // /proc/PID/cgroup
process.namespaces()?;  // /proc/PID/ns/
process.threads()?;     // /proc/PID/task/
```

#### Network (`proc::net`)
- `proc::net::tcp()` — `/proc/net/tcp` (IPv4 TCP connections)
- `proc::net::tcp6()` — `/proc/net/tcp6` (IPv6 TCP connections)
- `proc::net::udp()` — `/proc/net/udp` (IPv4 UDP sockets)
- `proc::net::unix()` — `/proc/net/unix` (Unix domain sockets)
- `proc::net::dev()` — `/proc/net/dev` (per-interface stats)
- `proc::net::arp()` — `/proc/net/arp` (ARP table)
- `proc::net::route()` — `/proc/net/route` (routing table)

### `/sys` Module
- `sys::BlockDevice::all()` — `/sys/block/*` (disk stats, queue params, size)
- `sys::NetInterface::all()` — `/sys/class/net/*` (stats, operstate, MAC, MTU, flags, speed)
- `sys::PowerSupply::all()` — `/sys/class/power_supply/*` (type, status, capacity, voltage, current)
- `sys::cpu_count()` — Number of online CPUs
- `sys::online_cpus()` — `Vec<u32>` of online CPU IDs
- `sys::cpu_freq(cpu)` — `CpuFreqInfo` for a specific CPU

### Unit Types

All numeric values carry their unit in the type:

```rust
use procfs2::{Bytes, Kibibytes, Pages, Jiffies, Milliseconds};

let mem = proc::meminfo()?;
let kb: Kibibytes = mem.total;           // 14175680 KiB
let bytes: Bytes = Bytes::from(kb);      // convert to bytes
println!("{} MiB", bytes.as_mib());    // convert to MiB

// Jiffies (clock ticks, typically 10ms each)
let stat = proc::stat()?;
let user_secs = stat.cpu_total.user.0 as f64 / 100.0; // convert to seconds
```

## Error Handling

All operations return `procfs2::Result<T>` which is `Result<T, procfs2::Error>`:

```rust
pub enum Error {
    Io(std::io::Error),
    Parse { path: PathBuf, line: usize, msg: &'static str },
    ProcessGone(u32),
    PermissionDenied(PathBuf),
    UnsupportedKernel { required: KernelVersion, found: KernelVersion },
}
```

## Minimum Supported Rust Version

Rust 1.75 — required for `impl Trait` in trait return types.

## License

Licensed under the [MIT License](LICENSE).

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

### Building

```bash
# Clone the repository
git clone https://github.com/lucop1911/procfs2.git
cd procfs2

# Build with default features
cargo build

# Build with all features
cargo build --features "async,serde,macros,watch"

# Run tests (requires Linux)
cargo test

# Run clippy
cargo clippy --features "async,serde,macros,watch"
```

### Running Benchmarks

```bash
cargo bench
```

### Proc-Macro (Optional)

The `#[derive(ProcKeyValue)]` macro is in the `procfs2-macros` crate.
Enable with the `macros` feature:

```rust
use procfs2::macros::ProcKeyValue;

#[derive(ProcKeyValue)]
pub struct MemInfo {
    #[proc_key = "MemTotal"]
    pub total: Kibibytes,
    #[proc_key = "MemFree"]
    pub free: Kibibytes,
}
```
