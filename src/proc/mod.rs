pub mod cgroups;
pub mod cpuinfo;
pub mod loadavg;
pub mod meminfo;
pub mod mounts;
pub mod net;
pub mod process;
pub mod stat;
pub mod uptime;
pub mod version;

pub use cgroups::{cgroups, CgroupStat};
pub use cpuinfo::{cpuinfo, CpuCore, CpuFlag};
pub use loadavg::{loadavg, LoadAvg};
pub use meminfo::MemInfo;
pub use mounts::{mounts, MountEntry};
pub use process::Process;
pub use stat::{stat, CpuTime, PerCpuTime, SystemStat};
pub use uptime::{uptime, Uptime};
pub use version::version;

/// Reads `/proc/meminfo` and returns system-wide memory statistics.
///
/// This is a convenience wrapper around [`MemInfo::current`].
pub fn meminfo() -> crate::error::Result<MemInfo> {
    MemInfo::current()
}
