//! Derives a key-value parser from a struct using the `ProcKeyValue` proc-macro.
//!
//! The `#[proc_key = "..."]` attribute maps each field to a line key in the
//! source file. Rather than hand-writing a `parse_from_bytes` impl, the derive
//! inspects the field types and generates the parsing for you.
//!
//! Run with: `cargo run --example macros --features macros`

use procfs2::macros::ProcKeyValue;
use procfs2::util::Kibibytes;
use procfs2::util::parse::ParseFromBytes;

#[derive(ProcKeyValue)]
struct MemInfo {
    #[proc_key = "MemTotal"]
    total: Kibibytes,
    #[proc_key = "MemFree"]
    free: Kibibytes,
    #[proc_key = "MemAvailable"]
    available: Option<Kibibytes>,
    #[proc_key = "Buffers"]
    buffers: Kibibytes,
    #[proc_key = "SwapTotal"]
    swap_total: Kibibytes,
    #[proc_key = "SwapFree"]
    swap_free: Kibibytes,
}

fn main() {
    let bytes = procfs2::util::parse::read_file(std::path::Path::new("/proc/meminfo"))
        .expect("Failed to read /proc/meminfo");
    let mem = MemInfo::parse_from_bytes(&bytes).expect("Failed to parse /proc/meminfo");

    println!("=== /proc/meminfo via ProcKeyValue derive ===");
    println!("Total:    {} kB", mem.total.0);
    println!("Free:     {} kB", mem.free.0);
    println!("Buffers:  {} kB", mem.buffers.0);
    println!("Available: {} kB", mem.available.map(|a| a.0).unwrap_or(0));
    println!(
        "Swap:     {} kB used / {} kB total",
        mem.swap_total.0 - mem.swap_free.0,
        mem.swap_total.0
    );
}
