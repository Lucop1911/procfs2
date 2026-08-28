# procfs2

A modern, zero-copy, strongly-typed Rust library for reading Linux's `/proc` and `/sys` virtual filesystems.

[![Rust](https://github.com/lucop1911/procfs2/actions/workflows/rust.yml/badge.svg)](https://github.com/lucop1911/procfs2/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/procfs2.svg)](https://crates.io/crates/procfs2)
[![Documentation](https://docs.rs/procfs2/badge.svg)](https://docs.rs/procfs2)
[![License: MIT](https://img.shields.io/crates/l/procfs2.svg)](https://opensource.org/licenses/MIT)

## Features

- **Typed API** — Every kernel file maps to a concrete Rust struct or enum
- **Zero-copy where possible** — Parse directly from `&[u8]` slices
- **Broad `/proc` coverage** — System-wide, per-process, network, cgroups
- **First-class `/sys` support** — Block devices, network interfaces, power supply, CPU info
- **Native async support** — Not just async file reads: a generic polling combinator (`watch()`) turns any snapshot function into a `Stream`, plus purpose-built delta-streaming for cumulative counters (see [Live Monitoring](#live-monitoring--async-delta-streaming))
- **Granular, typed errors** — Distinguishes *why* an operation failed (process exited mid-read, unsupported kernel, permission denied, malformed data at a specific line) instead of a single generic I/O failure
- **Runtime-safe** — No panics in the core parsing paths; all errors returned as typed `Error` variants

## Quick Start

```rust
use procfs2::proc;

fn main() -> procfs2::Result<()> {
    // System-wide statistics
    let mem = proc::meminfo()?;
    println!("Total memory: {} KB", mem.total.0);
    println!("Available: {} KB", mem.available.0);

    let load = proc::loadavg()?;
    println!("Load: {:.2} {:.2} {:.2}", load.one, load.five, load.fifteen);

    // Current process
    use procfs2::proc::Process;
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
    use procfs2::sys::{BlockDevice, NetInterface};
    for dev in BlockDevice::all().filter_map(|r| r.ok()) {
        println!("Block device: {}", dev.name);
        if let Ok(stat) = dev.stat() {
            println!("  reads={}, writes={}", stat.reads_completed, stat.writes_completed);
        }
    }

    // Network interfaces
    for iface in NetInterface::all().filter_map(|r| r.ok()) {
        println!("Interface: {}", iface.name);
        if let Ok(stats) = iface.stats() {
            println!("  RX: {} bytes, TX: {} bytes", stats.rx_bytes.0, stats.tx_bytes.0);
        }
    }

    Ok(())
}
```

## Live Monitoring / Async Delta-Streaming

Most `/proc` counters (network bytes, disk I/O, CPU jiffies) are cumulative — what you usually want is a *rate*, not a raw snapshot. procfs2 provides this as a first-class primitive instead of leaving it to every caller to reimplement "read twice, subtract, divide by elapsed time."

```rust, ignore
use futures_util::StreamExt;
use procfs2::async_helpers::{self, read_to_string};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let uptime = async_helpers::watch(Duration::from_secs(1), || async {
        read_to_string("/proc/uptime").await
    });
    tokio::pin!(uptime);

    while let Some(sample) = uptime.next().await {
        match sample {
            Ok(snapshot) => print!("uptime: {snapshot}"),
            Err(error) => {
                eprintln!("failed to read /proc/uptime: {error}");
                break;
            }
        }
    }
}
```

`async_helpers::watch()` is a generic polling combinator — it accepts any `async fn() -> Result<T>` and turns it into a `Stream<Item = Result<T>>` on a fixed interval. Layer `Sampler<T>` on top for automatic, wraparound-safe delta computation between consecutive snapshots (see `watch_net_dev` for a full example with `/proc/net/dev`).

This requires the `async` feature. procfs has no async or streaming API at all — everything above is a pattern procfs2 supports natively rather than something you'd hand-roll on top of a sync-only crate.

> **Note on inotify:** Linux's `inotify` does not reliably fire on most `/proc` entries — the kernel allocates a fresh inode on most procfs opens, which inotify can't attach persistent watches to. procfs2 includes an inotify `Watcher` (under the `watch` feature) for watching *regular* files, but `/proc` monitoring should go through `async_helpers::watch()` above, not inotify.

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
| `async` | Async read variants, generic `watch()` polling combinator, delta-streaming (`Sampler`, `NetDeltaSample`) | `tokio` |
| `watch` | `inotify`-based `Watcher` API for regular files | `libc` |
| `serde` | `Serialize`/`Deserialize` on all structs | `serde` |
| `macros` | `#[derive(ProcKeyValue)]` proc-macro | `procfs2-macros` |

## Module Overview

### `/proc` Module

#### System-wide
- `proc::uptime()` — `Uptime { total, idle }` from `/proc/uptime`
- `proc::loadavg()` — `LoadAvg { one, five, fifteen, runnable, total }` from `/proc/loadavg`
- `proc::meminfo()` — `MemInfo { total, free, available, ... }` from `/proc/meminfo`
- `proc::stat()` — `Stat { cpu_total, per_cpu, ctxt, btime, ... }` from `/proc/stat`
- `proc::cpuinfo()` — `Vec<CpuCore>` from `/proc/cpuinfo`
- `proc::version()` — `KernelVersion` from `/proc/version`
- `proc::mounts()` — `Vec<Mount>` from `/proc/mounts`
- `proc::cgroups()` — `Vec<Cgroup>` from `/proc/cgroups`

#### Per-Process (`Process`)
```rust
use procfs2::proc::Process;

fn main() -> procfs2::Result<()> {
    // Open the current process
    let me = Process::current()?;
    // Iterate over all PIDs on the system
    let _all_procs = Process::all();

    // Methods
    me.stat()?;       // /proc/PID/stat
    me.status()?;     // /proc/PID/status
    me.cmdline()?;    // /proc/PID/cmdline
    me.environ()?;    // /proc/PID/environ
    me.exe()?;         // /proc/PID/exe
    me.cwd()?;         // /proc/PID/cwd
    me.maps()?;        // /proc/PID/maps
    me.smaps()?;      // /proc/PID/smaps
    me.fds()?;         // /proc/PID/fd/
    me.io()?;          // /proc/PID/io
    me.limits()?;      // /proc/PID/limits
    me.mountinfo()?;   // /proc/PID/mountinfo
    me.cgroup()?;      // /proc/PID/cgroup
    me.namespaces()?;  // /proc/PID/ns/
    for _ in me.threads() {} // /proc/PID/task/

    Ok(())
}
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
use procfs2::proc;

fn main() -> procfs2::Result<()> {
    use procfs2::util::Kibibytes;
    let mem = proc::meminfo()?;
    let kb: Kibibytes = mem.total;           // e.g., 14175680 KiB
    use procfs2::util::Bytes;
    let bytes: Bytes = Bytes::from(kb);      // convert to bytes
    println!("{} MiB", bytes.as_mib());     // convert to MiB

    // Jiffies (clock ticks, typically 10ms each)
    let stat = proc::stat()?;
    let user_secs = stat.cpu_total.user.0 as f64 / 100.0; // convert to seconds

    Ok(())
}
```

## Error Handling

All operations return `procfs2::Result<T>`, which is `Result<T, procfs2::Error>`. Errors are structured by cause rather than collapsed into a single generic I/O failure — a caller can distinguish, for example, "the process exited mid-read" from "this kernel doesn't support this file" from "malformed data on line N," and handle each differently:

```rust
pub enum Error {
    /// Wrapped I/O failure. `path` is the file involved, when there is one
    /// (some operations, like inotify fd errors, have no single associated path).
    Io { path: Option<std::path::PathBuf>, error: std::io::Error },

    /// A parsing failure at a specific line of a specific file.
    Parse { path: std::path::PathBuf, line: usize, msg: &'static str },

    /// The target process exited between discovering its PID and opening its `/proc/PID` entry.
    ProcessGone(u32),

    /// Insufficient permissions to read a `/proc` or `/sys` path.
    PermissionDenied(std::path::PathBuf),

    /// The running kernel doesn't meet the minimum version required for a feature.
    UnsupportedKernel { required: procfs2::KernelVersion, found: procfs2::KernelVersion },
}
```

Where kernel formats evolve incrementally (e.g. `/proc/diskstats` gaining trailing fields on newer kernels), procfs2 prefers detecting the file's actual shape over hard version-gating — so a file with fewer fields on an older kernel returns partial data (`Option<T>` on the newer fields) instead of failing outright.

## Minimum Supported Rust Version

Rust 1.86 — verified in CI by building with the 1.86.0 toolchain.

## License

Licensed under the [MIT License](https://github.com/lucop1911/procfs2/blob/master/LICENSE).

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

### Running the Demo

A small CLI that dumps system info is included as an example:

```bash
cargo run --example demo
```

Async delta-streaming and inotify examples are also included:

```bash
cargo run --example async_watch --features async
cargo run --example watch --features watch
```

### Running Benchmarks

```bash
cargo bench
```

### Proc-Macro (Optional)

The `#[derive(ProcKeyValue)]` macro is in the `procfs2-macros` crate.
Enable with the `macros` feature:

```rust,ignore
use procfs2::macros::ProcKeyValue;

#[derive(ProcKeyValue)]
pub struct MemInfo {
    #[proc_key = "MemTotal"]
    pub total: Kibibytes,
    #[proc_key = "MemFree"]
    pub free: Kibibytes,
}
```