#![allow(unsafe_code)]

pub mod inotify;

pub use inotify::{Watcher, WatchHandle, WatchEvent};
