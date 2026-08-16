pub mod cgroups;
pub mod cmdline;
pub mod cpuinfo;
pub mod devices;
pub mod diskstats;
pub mod filesystems;
pub mod loadavg;
pub mod meminfo;
pub mod mounts;
pub mod net;
pub mod partitions;
pub mod process;
pub mod stat;
pub mod swaps;
pub mod uptime;
pub mod version;

pub use cgroups::{CgroupStat, cgroups};
pub use cmdline::cmdline;
pub use cpuinfo::{CpuCore, CpuFlag, cpuinfo};
pub use devices::{Device, DeviceKind, devices};
pub use diskstats::{DiskStat, diskstats};
pub use filesystems::{FileSystem, filesystems};
pub use loadavg::{LoadAvg, loadavg};
pub use meminfo::MemInfo;
pub use mounts::{MountEntry, mounts};
pub use partitions::{Partition, partitions};
pub use process::Process;
pub use stat::{CpuTime, PerCpuTime, SystemStat, stat};
pub use swaps::{Swap, SwapType, swaps};
pub use uptime::{Uptime, uptime};
pub use version::version;

/// Reads `/proc/meminfo` and returns system-wide memory statistics.
///
/// This is a convenience wrapper around [`MemInfo::current`].
pub fn meminfo() -> crate::error::Result<MemInfo> {
    MemInfo::current()
}
