//! Uses the async file helpers to read `/proc` entries without blocking.
//!
//! The `async` feature pulls in tokio's filesystem support. These helpers
//! mirror `std::fs`, so use them anywhere you would otherwise do a synchronous
//! read inside an async context.
//!
//! Run with: `cargo run --example async --features async`

use procfs2::async_helpers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== async_helpers::read_file ===");
    let bytes = async_helpers::read_file("/proc/meminfo").await?;
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines().take(5) {
        println!("{line}");
    }

    println!("\n=== async_helpers::read_to_string ===");
    let uptime = async_helpers::read_to_string("/proc/uptime").await?;
    println!("{uptime}");

    Ok(())
}
