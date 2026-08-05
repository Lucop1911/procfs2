#![allow(unsafe_code)]

pub mod inotify;

pub use inotify::{WatchEvent, WatchHandle, Watcher};
