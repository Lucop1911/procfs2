use crate::error::{Error, Result};
use crate::util::{parse, Kibibytes};

/// System-wide memory statistics.
///
/// Sourced from `/proc/meminfo`. All memory values are in kibibytes
/// (1024 bytes), matching the kernel's native reporting unit.
///
/// Fields that are absent on a given kernel default to zero rather
/// than returning an error, since `/proc/meminfo` has grown many
/// optional entries over time.
#[derive(Debug)]
pub struct MemInfo {
    /// Total usable RAM (excluding swap).
    pub total: Kibibytes,
    /// Completely unused RAM.
    pub free: Kibibytes,
    /// RAM available for starting new applications without swapping.
    ///
    /// This accounts for reclaimable caches and is the value most
    /// tools report as "available memory".
    pub available: Kibibytes,
    /// Buffer cache (block device metadata).
    pub buffers: Kibibytes,
    /// Page cache (file content cache).
    pub cached: Kibibytes,
    /// Total swap space.
    pub swap_total: Kibibytes,
    /// Unused swap space.
    pub swap_free: Kibibytes,
    /// Reclaimable slab cache (shrinkable under memory pressure).
    pub slab_reclaimable: Kibibytes,
    /// Non-reclaimable slab cache.
    pub slab_unreclaimable: Kibibytes,
    /// Shared memory (tmpfs, shmem).
    pub shmem: Kibibytes,
    /// Active pages (recently used, unlikely to be reclaimed).
    pub active: Kibibytes,
    /// Inactive pages (not recently used, reclaim candidates).
    pub inactive: Kibibytes,
    /// File-backed pages pending writeback to storage.
    pub dirty: Kibibytes,
    /// Pages actively being written to storage.
    pub writeback: Kibibytes,
    /// Anonymous pages (not file-backed, e.g. heap, stack).
    pub anon_pages: Kibibytes,
    /// Memory-mapped files.
    pub mapped: Kibibytes,
    /// Page table memory.
    pub page_tables: Kibibytes,
    /// Total huge pages allocated.
    pub hugepages_total: u64,
    /// Huge pages currently free.
    pub hugepages_free: u64,
}

impl MemInfo {
    /// Reads `/proc/meminfo` and returns [`MemInfo`].
    pub fn current() -> Result<Self> {
        Self::from_path("/proc/meminfo")
    }

    /// Parses `/proc/meminfo` from an arbitrary path (used in tests).
    pub(super) fn from_path(path: &str) -> Result<Self> {
        let bytes = parse::read_file(std::path::Path::new(path))?;

        let mut total = None;
        let mut free = None;
        let mut available = None;
        let mut buffers = None;
        let mut cached = None;
        let mut swap_total = None;
        let mut swap_free = None;
        let mut slab_reclaimable = None;
        let mut slab_unreclaimable = None;
        let mut shmem = None;
        let mut active = None;
        let mut inactive = None;
        let mut dirty = None;
        let mut writeback = None;
        let mut anon_pages = None;
        let mut mapped = None;
        let mut page_tables = None;
        let mut hugepages_total = None;
        let mut hugepages_free = None;

        for (line_num, line) in bytes.split(|&b| b == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }

            let (key, value) = match parse::parse_key_value_line(line) {
                Some(kv) => kv,
                None => continue,
            };

            let value_trimmed = parse::trim_start(value);
            let num_bytes = value_trimmed
                .split(|&b| b == b' ')
                .next()
                .unwrap_or(value_trimmed);

            let val = parse::parse_dec_u64(num_bytes).map_err(|_| Error::Parse {
                path: std::path::PathBuf::from(path),
                line: line_num + 1,
                msg: "invalid number",
            })?;

            match key {
                b"MemTotal" => total = Some(Kibibytes(val)),
                b"MemFree" => free = Some(Kibibytes(val)),
                b"MemAvailable" => available = Some(Kibibytes(val)),
                b"Buffers" => buffers = Some(Kibibytes(val)),
                b"Cached" => cached = Some(Kibibytes(val)),
                b"SwapTotal" => swap_total = Some(Kibibytes(val)),
                b"SwapFree" => swap_free = Some(Kibibytes(val)),
                b"SlabReclaimable" => slab_reclaimable = Some(Kibibytes(val)),
                b"SlabUnreclaimable" => slab_unreclaimable = Some(Kibibytes(val)),
                b"Shmem" => shmem = Some(Kibibytes(val)),
                b"Active" => active = Some(Kibibytes(val)),
                b"Inactive" => inactive = Some(Kibibytes(val)),
                b"Dirty" => dirty = Some(Kibibytes(val)),
                b"Writeback" => writeback = Some(Kibibytes(val)),
                b"AnonPages" => anon_pages = Some(Kibibytes(val)),
                b"Mapped" => mapped = Some(Kibibytes(val)),
                b"PageTables" => page_tables = Some(Kibibytes(val)),
                b"HugePages_Total" => hugepages_total = Some(val),
                b"HugePages_Free" => hugepages_free = Some(val),
                _ => {}
            }
        }

        Ok(MemInfo {
            total: total.unwrap_or(Kibibytes(0)),
            free: free.unwrap_or(Kibibytes(0)),
            available: available.unwrap_or(Kibibytes(0)),
            buffers: buffers.unwrap_or(Kibibytes(0)),
            cached: cached.unwrap_or(Kibibytes(0)),
            swap_total: swap_total.unwrap_or(Kibibytes(0)),
            swap_free: swap_free.unwrap_or(Kibibytes(0)),
            slab_reclaimable: slab_reclaimable.unwrap_or(Kibibytes(0)),
            slab_unreclaimable: slab_unreclaimable.unwrap_or(Kibibytes(0)),
            shmem: shmem.unwrap_or(Kibibytes(0)),
            active: active.unwrap_or(Kibibytes(0)),
            inactive: inactive.unwrap_or(Kibibytes(0)),
            dirty: dirty.unwrap_or(Kibibytes(0)),
            writeback: writeback.unwrap_or(Kibibytes(0)),
            anon_pages: anon_pages.unwrap_or(Kibibytes(0)),
            mapped: mapped.unwrap_or(Kibibytes(0)),
            page_tables: page_tables.unwrap_or(Kibibytes(0)),
            hugepages_total: hugepages_total.unwrap_or(0),
            hugepages_free: hugepages_free.unwrap_or(0),
        })
    }
}
