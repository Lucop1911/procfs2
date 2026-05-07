#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#[cfg(not(target_os = "linux"))]
compile_error!("procfs2 only supports Linux. Use `#[cfg(target_os = \"linux\")]` to conditionally depend on this crate.");

pub mod error;
pub mod util;

#[cfg(feature = "macros")]
pub mod macros;

#[path = "proc/mod.rs"]
pub mod r#proc;

pub mod sys;
pub mod watch;

pub use error::{Error, KernelVersion, Result};
pub use util::{Bytes, Jiffies, Kibibytes, Milliseconds, Pages};

#[cfg(feature = "async")]
pub use tokio;

#[cfg(feature = "serde")]
pub use serde;
