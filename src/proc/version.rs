use crate::error::{KernelVersion, Result};

/// Reads the running kernel version from `/proc/version`.
///
/// Delegates to [`KernelVersion::current`] which parses the version
/// string embedded in `/proc/version`.
pub fn version() -> Result<KernelVersion> {
    KernelVersion::current()
}
