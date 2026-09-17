#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]

/// Inotify-based file watcher.
pub mod inotify;
/// Snapshot delta helper for computing per-interval changes.
pub mod sample;

pub use inotify::{WatchEvent, WatchHandle, Watcher};
pub use sample::Sampler;
