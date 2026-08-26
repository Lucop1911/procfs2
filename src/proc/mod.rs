pub mod buddyinfo;
pub mod cgroups;
pub mod cmdline;
pub mod cpuinfo;
pub mod devices;
pub mod diskstats;
pub mod filesystems;
pub mod interrupts;
pub mod iomem;
pub mod ioports;
pub mod loadavg;
pub mod meminfo;
pub mod misc;
pub mod modules;
pub mod mounts;
pub mod net;
pub mod pagetypeinfo;
pub mod partitions;
pub mod process;
pub mod softirqs;
pub mod stat;
pub mod swaps;
pub mod uptime;
pub mod version;
pub mod vmstat;
pub mod zoneinfo;

pub use buddyinfo::{BuddyInfo, buddyinfo};
pub use cgroups::{Cgroup, cgroups};
pub use cmdline::cmdline;
pub use cpuinfo::{CpuCore, CpuFlag, cpuinfo};
pub use devices::{Device, DeviceKind, devices};
pub use diskstats::{DiskStat, diskstats};
pub use filesystems::{FileSystem, filesystems};
pub use interrupts::{InterruptCount, InterruptRow, Interrupts, interrupts};
pub use iomem::{IoMem, iomem};
pub use ioports::{IoPort, ioports};
pub use loadavg::{LoadAvg, loadavg};
pub use meminfo::MemInfo;
pub use misc::{Misc, misc};
pub use modules::{Module, modules};
pub use mounts::{Mount, mounts};
pub use pagetypeinfo::{PageTypeBlock, PageTypeFree, PageTypeInfo, pagetypeinfo};
pub use partitions::{Partition, partitions};
pub use process::Process;
pub use softirqs::{SoftirqRow, Softirqs, softirqs};
pub use stat::{CpuTime, PerCpuTime, Stat, stat};
pub use swaps::{Swap, SwapType, swaps};
pub use uptime::{Uptime, uptime};
pub use version::version;
pub use vmstat::vmstat;
pub use zoneinfo::{Pcp, ZoneInfo, ZonePages, zoneinfo};

/// Reads `/proc/meminfo` and returns system-wide memory statistics.
///
/// This is a convenience wrapper around [`MemInfo::current`].
pub fn meminfo() -> crate::error::Result<MemInfo> {
    MemInfo::current()
}
