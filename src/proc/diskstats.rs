use std::path::Path;

use crate::error::{Error, KernelVersion, Result};
use crate::util::Milliseconds;
use crate::util::parse::{self, parse_dec_u32, parse_dec_u64};

#[derive(Debug)]
pub struct DiskStat {
    /// Device major number.
    pub major: u32,
    /// Device minor number.
    pub minor: u32,
    /// Device name (e.g. `sda1`, `nvme0n1`).
    pub name: Box<str>,
    /// Number of read I/Os completed.
    pub reads_completed: u64,
    /// Number of read I/Os merged with adjacent requests.
    pub reads_merged: u64,
    /// Number of 512-byte sectors read.
    pub sectors_read: u64,
    /// Total time spent reading (ms).
    pub time_reading: Milliseconds,
    /// Number of write I/Os completed.
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
    /// Weighted time spent doing I/O (ms).
    pub weighted_time_io: Milliseconds,
    /// Number of discard I/Os completed.
    pub discards_completed: u64,
    /// Number of discard I/Os merged.
    pub discards_merged: u64,
    /// Number of 512-byte sectors discarded.
    pub sectors_discarded: u64,
    /// Total time spent discarding (ms).
    pub time_discarding: Milliseconds,
    /// Number of flush requests completed.
    pub flush_completed: u64,
    /// Total time spent flushing (ms).
    pub time_flushing: Milliseconds,
}

/// Reads `/proc/diskstats` and returns per-device I/O statistics.
///
/// The file has no header: one whitespace-aligned line per block
/// device and partition, holding `major minor name` followed by 17
/// numeric statistics. It lists the whole device (`sda`, `nvme0n1`)
/// as well as each of its partitions (`sda1`, `nvme0n1p2`).
///
/// Requires kernel 5.5 or newer, which added the flush request
/// fields at the end of each line. On older kernels this returns
/// [`Error::UnsupportedKernel`].
pub fn diskstats() -> Result<Vec<DiskStat>> {
    KernelVersion::current()?.require(5, 5)?;

    let path = Path::new("/proc/diskstats");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<20>::new(line);

        if fields.len() != 20 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<major> <minor> <name> <17 stat fields>'",
            });
        }

        let major = parse_dec_u32(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid major number",
        })?;

        let minor = parse_dec_u32(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid minor number",
        })?;

        let name = std::str::from_utf8(fields[2]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        let reads_completed = parse_dec_u64(fields[3]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid reads completed",
        })?;

        let reads_merged = parse_dec_u64(fields[4]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid reads merged",
        })?;

        let sectors_read = parse_dec_u64(fields[5]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid sectors read",
        })?;

        let time_reading = Milliseconds(parse_dec_u64(fields[6]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid time reading",
        })?);

        let writes_completed = parse_dec_u64(fields[7]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid writes completed",
        })?;

        let writes_merged = parse_dec_u64(fields[8]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid writes merged",
        })?;

        let sectors_written = parse_dec_u64(fields[9]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid sectors written",
        })?;

        let time_writing = Milliseconds(parse_dec_u64(fields[10]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid time writing",
        })?);

        let io_in_progress = parse_dec_u64(fields[11]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid io in progress",
        })?;

        let time_io = Milliseconds(parse_dec_u64(fields[12]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid time io",
        })?);

        let weighted_time_io =
            Milliseconds(parse_dec_u64(fields[13]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid weighted time io",
            })?);

        let discards_completed = parse_dec_u64(fields[14]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid discards completed",
        })?;

        let discards_merged = parse_dec_u64(fields[15]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid discards merged",
        })?;

        let sectors_discarded = parse_dec_u64(fields[16]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid sectors discarded",
        })?;

        let time_discarding =
            Milliseconds(parse_dec_u64(fields[17]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid time discarding",
            })?);

        let flush_completed = parse_dec_u64(fields[18]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid flush completed",
        })?;

        let time_flushing = Milliseconds(parse_dec_u64(fields[19]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid time flushing",
        })?);

        out.push(DiskStat {
            major,
            minor,
            name: name.into(),
            reads_completed,
            reads_merged,
            sectors_read,
            time_reading,
            writes_completed,
            writes_merged,
            sectors_written,
            time_writing,
            io_in_progress,
            time_io,
            weighted_time_io,
            discards_completed,
            discards_merged,
            sectors_discarded,
            time_discarding,
            flush_completed,
            time_flushing,
        });
    }

    Ok(out)
}
