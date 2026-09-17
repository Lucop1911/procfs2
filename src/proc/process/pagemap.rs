use std::path::PathBuf;

use crate::error::{Error, Result};

/// A single entry from `/proc/PID/pagemap`.
///
/// Each entry is a 64-bit value describing one virtual page. The
/// kernel's `Documentation/filesystems/proc.rst` defines the bit
/// layout; only the fields understood by this crate are exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMapEntry {
    /// Page frame number (bits 0-54). Present only if the page is
    /// resident or swapped out.
    pub pfn: Option<u64>,
    /// Bit 56: page is present in RAM.
    pub present: bool,
    /// Bit 57: page is swapped out.
    pub swapped: bool,
    /// Bit 55: the PTE is a swap entry. Valid only when the page is
    /// swapped out, in which case `pfn` holds the swap offset.
    pub swap_pte: bool,
    /// Bit 58: page has been written to since the soft-dirty bit was
    /// last cleared.
    pub soft_dirty: bool,
    /// Bit 59: page is mapped in only one process (kernel 3.11+).
    pub exclusive: bool,
    /// Bit 60: page is backed by a file rather than anonymous memory
    /// (kernel 5.8+; always false on older kernels).
    pub file_page: bool,
}

impl PageMapEntry {
    /// Splits a raw 64-bit pagemap value into its fields.
    #[inline]
    pub fn from_raw(raw: u64) -> Self {
        const MASK_PFN: u64 = (1 << 55) - 1;
        const BIT_SWAP_PTE: u64 = 1 << 55;
        const BIT_PRESENT: u64 = 1 << 56;
        const BIT_SWAPPED: u64 = 1 << 57;
        const BIT_SOFT_DIRTY: u64 = 1 << 58;
        const BIT_EXCLUSIVE: u64 = 1 << 59;
        const BIT_FILE: u64 = 1 << 60;

        let present = raw & BIT_PRESENT != 0;
        let swapped = raw & BIT_SWAPPED != 0;

        PageMapEntry {
            pfn: (present || swapped).then_some(raw & MASK_PFN),
            present,
            swapped,
            swap_pte: raw & BIT_SWAP_PTE != 0,
            soft_dirty: raw & BIT_SOFT_DIRTY != 0,
            exclusive: raw & BIT_EXCLUSIVE != 0,
            file_page: raw & BIT_FILE != 0,
        }
    }

    /// Returns whether the page is either resident or swapped out.
    #[inline]
    pub fn is_mapped(&self) -> bool {
        self.present || self.swapped
    }
}

/// Parses a buffer of `/proc/PID/pagemap` entries.
///
/// The file is a flat array of native-endian `u64` values, one per
/// virtual page. Callers reading from a file should seek to the page
/// they want first; entry index `i` corresponds to the page at
/// address `i * page_size`. Unmapped holes read back as zero.
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<PageMapEntry>> {
    if bytes.len() % 8 != 0 {
        return Err(Error::Parse {
            path: PathBuf::from("<pagemap>"),
            line: 0,
            msg: "size is not a multiple of 8 bytes",
        });
    }

    Ok(bytes
        .chunks_exact(8)
        .map(|c| {
            let raw = u64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
            PageMapEntry::from_raw(raw)
        })
        .collect())
}
