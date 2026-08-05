/// Raw byte count.
///
/// Used for fields that report sizes in bytes, such as `/proc/PID/io`
/// read/write counters or `/sys/block/*/size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Bytes(pub u64);

/// Kibibytes (1024 bytes).
///
/// The unit used by most numeric fields in `/proc/meminfo` and
/// `/proc/PID/status` (e.g. `VmRSS`, `VmSize`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Kibibytes(pub u64);

/// Memory pages.
///
/// The kernel's native page-granularity unit. Used for RSS in
/// `/proc/PID/stat`. Convert to bytes with [`Pages::to_bytes`],
/// passing the system page size (typically 4096).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Pages(pub u64);

/// Kernel jiffies (clock ticks).
///
/// One jiffy equals one scheduler tick. On Linux `/proc` the tick
/// rate is `HZ = 100`, so 1 jiffy = 10 milliseconds. Used for CPU
/// time fields in `/proc/stat` and `/proc/PID/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Jiffies(pub u64);

/// Milliseconds.
///
/// Used for timing fields in `/sys/block/*/stat` (e.g. time spent
/// reading/writing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Milliseconds(pub u64);

impl From<Kibibytes> for Bytes {
    fn from(kb: Kibibytes) -> Self {
        Bytes(kb.0 * 1024)
    }
}

impl Bytes {
    /// Converts to kibibytes, truncating toward zero.
    pub fn as_kib(&self) -> Kibibytes {
        Kibibytes(self.0 / 1024)
    }

    /// Converts to mebibytes, truncating toward zero.
    pub fn as_mib(&self) -> u64 {
        self.0 / 1024 / 1024
    }

    /// Converts to gibibytes, truncating toward zero.
    pub fn as_gib(&self) -> u64 {
        self.0 / 1024 / 1024 / 1024
    }
}

impl Kibibytes {
    /// Converts to bytes.
    pub fn as_bytes(&self) -> Bytes {
        Bytes::from(*self)
    }
}

impl Pages {
    /// Converts pages to bytes given the system page size.
    ///
    /// On x86_64 Linux the page size is 4096. Use
    /// `sysconf(_SC_PAGESIZE)` or `std::sync::OnceLock` to
    /// obtain it at runtime.
    pub const fn to_bytes(self, page_size: u64) -> Bytes {
        Bytes(self.0 * page_size)
    }
}

impl Milliseconds {
    /// Converts to a [`std::time::Duration`].
    pub fn as_duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.0)
    }
}
