#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#[cfg(not(target_os = "linux"))]
compile_error!(
    "procfs2 only supports Linux. Use `#[cfg(target_os = \"linux\")]` to conditionally depend on this crate."
);

/// Error types and the [`KernelVersion`] helper.
pub mod error;
/// Byte-level parsing toolkit and unit newtypes.
pub mod util;

#[cfg(feature = "macros")]
/// Proc-macro derive for [`ParseFromBytes`](util::parse::ParseFromBytes).
pub mod macros;

#[cfg(feature = "async")]
/// Tokio-backed async file reads and a generic polling stream combinator.
pub mod async_helpers;

/// Typed parsers for system-wide and per-process `/proc` entries.
#[path = "proc/mod.rs"]
pub mod r#proc;

/// Typed parsers for `/sys` entries (block devices, CPUs, network, power).
pub mod sys;

#[cfg(feature = "watch")]
/// Inotify-based file watcher and a snapshot delta helper.
pub mod watch;

pub use error::{Error, KernelVersion, Result};
pub use util::{Bytes, Jiffies, Kibibytes, Milliseconds, Pages};

#[cfg(feature = "async")]
pub use tokio;

#[cfg(feature = "async")]
pub use futures_core;

#[cfg(feature = "serde")]
pub use serde;
