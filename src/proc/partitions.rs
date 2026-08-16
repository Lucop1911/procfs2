use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse::{parse_dec_u32, parse_dec_u64};
use crate::util::{Kibibytes, parse};

/// A single block device or partition from `/proc/partitions`.
#[derive(Debug, Clone)]
pub struct Partition {
    /// Device major number.
    pub major: u32,
    /// Device minor number.
    pub minor: u32,
    /// Size of the device or partition in kibibytes.
    pub blocks: Kibibytes,
    /// Device name (e.g. `sda1`).
    pub name: Box<str>,
}

/// Reads `/proc/partitions` and returns the partition table.
///
/// The file has a `major minor #blocks name` header followed by one
/// whitespace-aligned line per block device and partition. Splitting
/// on whitespace yields the four fields cleanly.
pub fn partitions() -> Result<Vec<Partition>> {
    let path = Path::new("/proc/partitions");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<Partition> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<4>::new(line);

        if fields.first() == Some(&&b"major"[..]) {
            continue;
        }
        if fields.len() != 4 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<major> <minor> <blocks> <name>'",
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

        let blocks = parse_dec_u64(fields[2])
            .map(Kibibytes)
            .map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid size",
            })?;

        let name = std::str::from_utf8(fields[3]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        out.push(Partition {
            major,
            minor,
            blocks,
            name: name.into(),
        })
    }

    Ok(out)
}
