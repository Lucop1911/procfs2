#![allow(unsafe_code)]
#![allow(clippy::unwrap_used)]

pub mod inotify;
pub mod sample;

pub use inotify::{WatchEvent, WatchHandle, Watcher};
pub use sample::Sampler;
