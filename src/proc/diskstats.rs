use std::path::Path;

use crate::error::{Error, Result};
use crate::util::Milliseconds;
use crate::util::parse::{self, parse_dec_u32_fast, parse_dec_u64_fast};

#[derive(Debug)]
/// A single disk's I/O statistics from `/proc/diskstats`.
pub struct DiskStat {
    /// Device major number.
    pub major: u32,
    /// Device minor number.
    pub minor: u32,
    /// Device name (e.g. `sda1`, `nvme0n1`).
    pub name: Box<str>,
    /// Read I/Os completed.
    pub reads_completed: u64,
    /// Read I/Os merged with adjacent requests.
    pub reads_merged: u64,
    /// 512-byte sectors read.
    pub sectors_read: u64,
    /// Time spent reading (ms).
    pub time_reading: Milliseconds,
    /// Write I/Os completed.
    pub writes_completed: u64,
    /// Write I/Os merged with adjacent requests.
    pub writes_merged: u64,
    /// 512-byte sectors written.
    pub sectors_written: u64,
    /// Time spent writing (ms).
    pub time_writing: Milliseconds,
    /// I/Os currently in flight.
    pub io_in_progress: u64,
    /// Time spent doing I/O (ms).
    pub time_io: Milliseconds,
    /// Weighted time doing I/O (ms).
    pub weighted_time_io: Milliseconds,
    /// Discard I/Os completed.
    pub discards_completed: Option<u64>,
    /// Discard I/Os merged.
    pub discards_merged: Option<u64>,
    /// 512-byte sectors discarded.
    pub sectors_discarded: Option<u64>,
    /// Time spent discarding (ms).
    pub time_discarding: Option<Milliseconds>,
    /// Flush requests completed.
    pub flush_completed: Option<u64>,
    /// Time spent flushing (ms).
    pub time_flushing: Option<Milliseconds>,
}

/// Reads `/proc/diskstats` and returns per-device I/O statistics.
///
/// The file has no header: one whitespace-aligned line per block
/// device and partition, holding `major minor name` followed by at
/// least 11 numeric statistics. Discard and flush statistics are
/// optional because they were added by later kernel versions. It lists
/// the whole device (`sda`, `nvme0n1`)
/// as well as each of its partitions (`sda1`, `nvme0n1p2`).
pub fn diskstats() -> Result<Vec<DiskStat>> {
    let path = Path::new("/proc/diskstats");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<20>::new(line);

        if fields.len() < 14 || fields.len() > 20 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<major> <minor> <name> <11-17 stat fields>'",
            });
        }

        let major = parse_dec_u32_fast(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid major number",
        })?;

        let minor = parse_dec_u32_fast(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid minor number",
        })?;

        let name = std::str::from_utf8(fields[2]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        let reads_completed = parse_dec_u64_fast(fields[3]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid reads completed",
        })?;

        let reads_merged = parse_dec_u64_fast(fields[4]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid reads merged",
        })?;

        let sectors_read = parse_dec_u64_fast(fields[5]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid sectors read",
        })?;

        let time_reading =
            Milliseconds(parse_dec_u64_fast(fields[6]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid time reading",
            })?);

        let writes_completed = parse_dec_u64_fast(fields[7]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid writes completed",
        })?;

        let writes_merged = parse_dec_u64_fast(fields[8]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid writes merged",
        })?;

        let sectors_written = parse_dec_u64_fast(fields[9]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid sectors written",
        })?;

        let time_writing =
            Milliseconds(parse_dec_u64_fast(fields[10]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid time writing",
            })?);

        let io_in_progress = parse_dec_u64_fast(fields[11]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid io in progress",
        })?;

        let time_io = Milliseconds(parse_dec_u64_fast(fields[12]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid time io",
        })?);

        let weighted_time_io =
            Milliseconds(parse_dec_u64_fast(fields[13]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid weighted time io",
            })?);

        let parse_optional = |index, msg| {
            if index < fields.len() {
                Ok(Some(parse_dec_u64_fast(fields[index]).map_err(|_| {
                    Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg,
                    }
                })?))
            } else {
                Ok(None)
            }
        };
        let discards_completed = parse_optional(14, "invalid discards completed")?;
        let discards_merged = parse_optional(15, "invalid discards merged")?;
        let sectors_discarded = parse_optional(16, "invalid sectors discarded")?;
        let time_discarding = parse_optional(17, "invalid time discarding")?.map(Milliseconds);
        let flush_completed = parse_optional(18, "invalid flush completed")?;
        let time_flushing = parse_optional(19, "invalid time flushing")?.map(Milliseconds);

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
