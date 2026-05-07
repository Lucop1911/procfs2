use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::{Bytes, Milliseconds};

/// Statistics for a block device from `/sys/block/<name>/stat`.
///
/// Mirrors the format of `/proc/diskstats` — 11 space-separated
/// decimal values. Time fields are in milliseconds.
#[derive(Debug)]
pub struct BlockStat {
    /// Number of read I/Os issued.
    pub reads_completed: u64,
    /// Number of read I/Os merged with adjacent requests.
    pub reads_merged: u64,
    /// Number of 512-byte sectors read.
    pub sectors_read: u64,
    /// Total time spent reading (ms).
    pub time_reading: Milliseconds,
    /// Number of write I/Os issued.
    pub writes_completed: u64,
    /// Number of write I/Os merged with adjacent requests.
    pub writes_merged: u64,
    /// Number of 512-byte sectors written.
    pub sectors_written: u64,
    /// Total time spent writing (ms).
    pub time_writing: Milliseconds,
    /// Number of I/Os currently in flight.
    pub io_in_progress: u64,
    /// Total time spent doing I/O (ms).
    pub time_io: Milliseconds,
}

/// Queue parameters from `/sys/block/<name>/queue/`.
///
/// These control how the block layer schedules and dispatches I/O
/// requests to the device.
#[derive(Debug)]
pub struct QueueParams {
    /// I/O scheduler name (e.g. `mq-deadline`, `kyber`, `bfq`).
    ///
    /// `None` if the device has no configurable scheduler (e.g.
    /// NVMe devices with no-op scheduling).
    pub scheduler: Option<Box<str>>,
    /// Maximum number of requests the block layer will allocate.
    pub nr_requests: u32,
    /// `true` if the device is rotational (HDD), `false` for SSD/NVMe.
    pub rotational: bool,
    /// Read-ahead window size in kibibytes.
    pub read_ahead_kb: u32,
}

/// A block device exposed under `/sys/block/<name>/`.
///
/// Each method reads a different set of files under the device's
/// sysfs directory.
pub struct BlockDevice {
    pub name: Box<str>,
    base: PathBuf,
}

impl BlockDevice {
    /// Iterates over all block devices visible in `/sys/block/`.
    ///
    /// Each entry is a symbolic link to the actual device under
    /// `/sys/devices/`. Only the basename (e.g. `sda`, `nvme0n1`)
    /// is used as the device name.
    pub fn all() -> impl Iterator<Item = Result<Self>> {
        let entries = match std::fs::read_dir("/sys/block") {
            Ok(iter) => iter,
            Err(e) => return vec![Err(Error::Io(e))].into_iter(),
        };

        entries
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.is_empty() {
                        Some(Ok(BlockDevice {
                            name: name_str.into_owned().into_boxed_str(),
                            base: e.path(),
                        }))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(Error::Io(e))),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Reads `/sys/block/<name>/stat` and returns [`BlockStat`].
    pub fn stat(&self) -> Result<BlockStat> {
        let path = self.base.join("stat");
        let bytes = parse::read_file(&path)?;
        let fields: Vec<&[u8]> = parse::split_spaces(&bytes);

        if fields.len() < 11 {
            return Err(Error::Parse {
                path,
                line: 0,
                msg: "not enough fields in stat",
            });
        }

        let get = |i: usize| parse::parse_dec_u64(fields[i]).unwrap_or(0);

        Ok(BlockStat {
            reads_completed: get(0),
            reads_merged: get(1),
            sectors_read: get(2),
            time_reading: Milliseconds(get(3)),
            writes_completed: get(4),
            writes_merged: get(5),
            sectors_written: get(6),
            time_writing: Milliseconds(get(7)),
            io_in_progress: get(8),
            time_io: Milliseconds(get(9)),
        })
    }

    /// Reads `/sys/block/<name>/size` and returns the device size in bytes.
    ///
    /// The kernel reports size in 512-byte sectors.
    pub fn size(&self) -> Result<Bytes> {
        let path = self.base.join("size");
        let bytes = parse::read_file(&path)?;
        let sectors = parse::parse_dec_u64(&bytes)?;
        Ok(Bytes(sectors * 512))
    }

    /// Reads queue parameters from `/sys/block/<name>/queue/`.
    pub fn queue_params(&self) -> Result<QueueParams> {
        let queue = self.base.join("queue");

        let scheduler = {
            let path = queue.join("scheduler");
            match parse::read_file(&path) {
                Ok(bytes) => {
                    let s = std::str::from_utf8(&bytes).unwrap_or("").trim();
                    // The active scheduler is wrapped in brackets:
                    // `mq-deadline [none] kyber`
                    if let Some(start) = s.find('[') {
                        s.find(']').map(|end| s[start + 1..end].to_string().into_boxed_str())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        };

        let nr_requests = {
            let path = queue.join("nr_requests");
            match parse::read_file(&path) {
                Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(128),
                Err(_) => 128,
            }
        };

        let rotational = {
            let path = queue.join("rotational");
            match parse::read_file(&path) {
                Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(0) != 0,
                Err(_) => false,
            }
        };

        let read_ahead_kb = {
            let path = queue.join("read_ahead_kb");
            match parse::read_file(&path) {
                Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(256),
                Err(_) => 256,
            }
        };

        Ok(QueueParams {
            scheduler,
            nr_requests,
            rotational,
            read_ahead_kb,
        })
    }
}
