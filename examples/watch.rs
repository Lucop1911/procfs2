//! Register a synchronous inotify watch on a regular file.
//!
//! inotify does not work reliably on `/proc` entries (the kernel
//! allocates a fresh inode per open for most procfs files, so change
//! notifications never fire — see e.g. LKML discussions on watching
//! /proc/mounts or /proc/<pid>). For monitoring `/proc` data, use
//! `async_helpers::watch()` instead (see `async_watch.rs`).
//!
//! Run with: `cargo run --example watch --features watch`

use procfs2::watch::Watcher;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("procfs2_watch_example.txt");
    std::fs::write(&path, "initial")?;

    let mut watcher = Watcher::new()?;
    let handle = watcher.watch(&path)?;
    println!("watching {:?} (descriptor {})", path, handle.watch_descriptor());

    // trigger a real event
    std::fs::OpenOptions::new().append(true).open(&path)?.write_all(b" updated")?;

    let event = watcher.next_event()?;
    println!("event: {event:?}");

    std::fs::remove_file(&path).ok();
    Ok(())
}