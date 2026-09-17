/// Buddy info for each memory zone from `/proc/buddyinfo`.
pub mod buddyinfo;
/// Control groups from `/proc/cgroups`.
pub mod cgroups;
/// Kernel command line from `/proc/cmdline`.
pub mod cmdline;
/// Console devices and their flags from `/proc/consoles`.
pub mod consoles;
/// Per-CPU information from `/proc/cpuinfo`.
pub mod cpuinfo;
/// Crypto algorithms registered in the kernel from `/proc/crypto`.
pub mod crypto;
/// Character and block device drivers from `/proc/devices`.
pub mod devices;
/// Disk I/O statistics from `/proc/diskstats`.
pub mod diskstats;
/// Filesystem types supported by the kernel from `/proc/filesystems`.
pub mod filesystems;
/// Hardware and software interrupt counters from `/proc/interrupts`.
pub mod interrupts;
/// Physical memory layout from `/proc/iomem`.
pub mod iomem;
/// I/O port regions from `/proc/ioports`.
pub mod ioports;
/// System load averages from `/proc/loadavg`.
pub mod loadavg;
/// Active file locks from `/proc/locks`.
pub mod locks;
/// System memory usage from `/proc/meminfo`.
pub mod meminfo;
/// Miscellaneous character devices from `/proc/misc`.
pub mod misc;
/// Loaded kernel modules from `/proc/modules`.
pub mod modules;
/// Mounted filesystems from `/proc/mounts`.
pub mod mounts;
/// Network protocol statistics from `/proc/net`.
pub mod net;
/// Per-page-type migration counters from `/proc/pagetypeinfo`.
pub mod pagetypeinfo;
/// Block device partition table from `/proc/partitions`.
pub mod partitions;
/// Pressure stall information (PSI) from `/proc/pressure`.
pub mod pressure;
/// Per-process information under `/proc/PID`.
pub mod process;
/// Software interrupt counters from `/proc/softirqs`.
pub mod softirqs;
/// System-wide CPU time counters from `/proc/stat`.
pub mod stat;
/// Swap devices from `/proc/swaps`.
pub mod swaps;
/// System uptime and idle time from `/proc/uptime`.
pub mod uptime;
/// Running kernel version from `/proc/version`.
pub mod version;
/// Virtual memory statistics from `/proc/vmstat`.
pub mod vmstat;
/// NUMA memory zone statistics from `/proc/zoneinfo`.
pub mod zoneinfo;

pub use buddyinfo::{BuddyInfo, buddyinfo};
pub use cgroups::{Cgroup, cgroups};
pub use cmdline::cmdline;
pub use consoles::{Console, ConsoleFlags, consoles};
pub use cpuinfo::{CpuCore, CpuFlag, cpuinfo};
pub use crypto::{Crypto, crypto};
pub use devices::{Device, DeviceKind, devices};
pub use diskstats::{DiskStat, diskstats};
pub use filesystems::{FileSystem, filesystems};
pub use interrupts::{InterruptCount, InterruptRow, Interrupts, interrupts};
pub use iomem::{IoMem, iomem};
pub use ioports::{IoPort, ioports};
pub use loadavg::{LoadAvg, loadavg};
pub use locks::{Lock, LockAccess, LockClass, LockType, locks};
pub use meminfo::MemInfo;
pub use misc::{Misc, misc};
pub use modules::{Module, modules};
pub use mounts::{Mount, mounts};
pub use pagetypeinfo::{PageTypeBlock, PageTypeFree, PageTypeInfo, pagetypeinfo};
pub use partitions::{Partition, partitions};
pub use pressure::{CpuPressure, IoPressure, IrqPressure, MemoryPressure, PressureEntry};
pub use pressure::{cpu_pressure, io_pressure, irq_pressure, memory_pressure};
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
