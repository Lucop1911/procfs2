#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]

pub mod inotify;

pub use inotify::{WatchEvent, WatchHandle, Watcher};
