use std::{
    ffi::OsStr,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

use crate::error::{Error, Result};

/// One pagemap entry: a native-endian `u64` per virtual page.
const ENTRY_SIZE: usize = std::mem::size_of::<u64>();
// `u64::from_ne_bytes` below reads exactly eight bytes.
const _: () = assert!(ENTRY_SIZE == 8);

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

/// Parses the pagemap of a process for a virtual address range.
///
/// Returns one [`PageMapEntry`] per page intersecting `start..end`.
/// `page_size` comes from the process's `AT_PAGESZ` entry, which the
/// caller is expected to have already read. This is the I/O
/// counterpart to [`parse`], which handles the bytes once they are in
/// memory.
pub(super) fn read(
    path: &OsStr,
    page_size: u64,
    start: u64,
    end: u64,
) -> Result<Vec<PageMapEntry>> {
    let first = start / page_size;
    let count = end.div_ceil(page_size).saturating_sub(first);

    if count == 0 {
        return Ok(Vec::new());
    }

    let mut file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            Error::PermissionDenied(PathBuf::from(&path))
        } else {
            Error::Io {
                path: Some(PathBuf::from(&path)),
                error: e,
            }
        }
    })?;

    file.seek(SeekFrom::Start(first * ENTRY_SIZE as u64))
        .map_err(|e| Error::Io {
            path: Some(PathBuf::from(&path)),
            error: e,
        })?;

    // Read up to the end of the address space. The kernel stops at
    // the last page it can map, so an early EOF is not an error.
    let mut buf = vec![0u8; (count * ENTRY_SIZE as u64) as usize];
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => {
                return Err(Error::Io {
                    path: Some(PathBuf::from(&path)),
                    error: e,
                });
            }
        }
    }
    buf.truncate(filled);

    parse(&buf)
}

/// Parses a buffer of `/proc/PID/pagemap` entries.
///
/// The file is a flat array of native-endian `u64` values, one per
/// virtual page. Callers reading from a file should seek to the page
/// they want first; entry index `i` corresponds to the page at
/// address `i * page_size`. Unmapped holes read back as zero.
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<PageMapEntry>> {
    if bytes.len() % ENTRY_SIZE != 0 {
        return Err(Error::Parse {
            path: PathBuf::from("<pagemap>"),
            line: 0,
            msg: "size is not a multiple of the pagemap entry size",
        });
    }

    Ok(bytes
        .chunks_exact(ENTRY_SIZE)
        .map(|c| {
            let raw = u64::from_ne_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
            PageMapEntry::from_raw(raw)
        })
        .collect())
}
