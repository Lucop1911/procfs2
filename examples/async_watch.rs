//! Poll a `/proc` entry with the generic async watch helper.
//!
//! Run with: `cargo run --example async_watch --features async`

use std::time::Duration;

use futures_util::StreamExt;
use procfs2::async_helpers::{self, read_to_string};

#[tokio::main]
async fn main() {
    let uptime = async_helpers::watch(Duration::from_secs(1), || async {
        read_to_string("/proc/uptime").await
    });
    tokio::pin!(uptime);

    for _ in 0..3 {
        match uptime.next().await {
            Some(Ok(snapshot)) => print!("uptime: {snapshot}"),
            Some(Err(error)) => {
                eprintln!("failed to read /proc/uptime: {error}");
                break;
            }
            None => break,
        }
    }
}
