pub mod cgroups;
pub mod cmdline;
pub mod cpuinfo;
pub mod devices;
pub mod filesystems;
pub mod loadavg;
pub mod meminfo;
pub mod mounts;
pub mod net;
pub mod process;
pub mod stat;
pub mod uptime;
pub mod version;

pub use cgroups::{CgroupStat, cgroups};
pub use cmdline::cmdline;
pub use cpuinfo::{CpuCore, CpuFlag, cpuinfo};
pub use devices::{Device, DeviceKind, devices};
pub use filesystems::{FileSystem, filesystems};
pub use loadavg::{LoadAvg, loadavg};
pub use meminfo::MemInfo;
pub use mounts::{MountEntry, mounts};
pub use process::Process;
pub use stat::{CpuTime, PerCpuTime, SystemStat, stat};
pub use uptime::{Uptime, uptime};
pub use version::version;

/// Reads `/proc/meminfo` and returns system-wide memory statistics.
///
/// This is a convenience wrapper around [`MemInfo::current`].
pub fn meminfo() -> crate::error::Result<MemInfo> {
    MemInfo::current()
}
