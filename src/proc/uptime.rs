use std::time::Duration;

use crate::error::{Error, Result};
use crate::util::parse;

/// System uptime and idle time.
///
/// Sourced from `/proc/uptime`, which reports two space-separated
/// decimal values: total seconds since boot and cumulative idle
/// seconds across all CPUs.
#[derive(Debug)]
pub struct Uptime {
    /// Wall-clock time since boot.
    pub total: Duration,
    /// Sum of idle time across all CPUs since boot.
    ///
    /// This value can exceed `total` on multi-core systems because
    /// each core accumulates idle time independently.
    pub idle: Duration,
}

/// Reads `/proc/uptime` and returns [`Uptime`].
pub fn uptime() -> Result<Uptime> {
    let bytes = parse::read_file(std::path::Path::new("/proc/uptime"))?;

    let space_idx = parse::memchr(b' ', &bytes).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("/proc/uptime"),
        line: 1,
        msg: "missing space separator",
    })?;

    let total_secs = parse::parse_dec_f64(&bytes[..space_idx])?;
    let idle_bytes = parse::trim_end(&bytes[space_idx + 1..]);
    let idle_secs = parse::parse_dec_f64(idle_bytes)?;

    Ok(Uptime {
        total: Duration::from_secs_f64(total_secs),
        idle: Duration::from_secs_f64(idle_secs),
    })
}
