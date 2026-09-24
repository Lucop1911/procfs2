use bitflags::bitflags;

use crate::error::Result;
use crate::util::parse;

bitflags! {
    /// Which memory mapping types are written to a core dump.
    ///
    /// Parsed from the hexadecimal bitmask in `/proc/PID/coredump_filter`.
    /// When a bit is set, mappings of that type are dumped; when clear,
    /// they are skipped. The kernel default is anonymous private and
    /// shared mappings, ELF headers, and private huge pages (`0x33`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CoreDumpFilter: u32 {
        /// Anonymous private mappings (heap, stack).
        const ANON_PRIVATE    = 1 << 0;
        /// Anonymous shared mappings.
        const ANON_SHARED     = 1 << 1;
        /// File-backed private mappings.
        const MAPPED_PRIVATE  = 1 << 2;
        /// File-backed shared mappings.
        const MAPPED_SHARED   = 1 << 3;
        /// ELF headers (since Linux 2.6.24).
        const ELF_HEADERS     = 1 << 4;
        /// Private huge pages (since Linux 2.6.28).
        const HUGETLB_PRIVATE = 1 << 5;
        /// Shared huge pages (since Linux 2.6.33).
        const HUGETLB_SHARED  = 1 << 6;
        /// Private device-dependent (DAX) huge pages.
        const DAX_PRIVATE     = 1 << 7;
        /// Shared device-dependent (DAX) huge pages.
        const DAX_SHARED      = 1 << 8;
    }
}

impl CoreDumpFilter {
    /// Parses a `/proc/PID/coredump_filter` value from raw bytes.
    ///
    /// The kernel writes the mask zero-padded to eight hex digits with
    /// no `0x` prefix, e.g. `00000033`. Bits above the nine defined ones
    /// cannot be stored by the kernel and are dropped.
    ///
    /// Kernel threads have no address space, so the file reads empty and
    /// yields a filter with no bits set.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let v = parse::parse_hex_u64(bytes)?;
        Ok(Self::from_bits_retain(v as u32 & 0x1ff))
    }
}
