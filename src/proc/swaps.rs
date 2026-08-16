use std::path::Path;

use crate::error::{Error, Result};
use crate::util::{Kibibytes, parse};

/// The type of a swap area, from the `Type` column of `/proc/swaps`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapType {
    /// A swap partition (e.g. `/dev/zram0`).
    Partition,
    /// A swap file.
    File,
}

/// A single swap area from `/proc/swaps`.
#[derive(Debug, Clone)]
pub struct Swap {
    /// Path to the backing device or file.
    pub filename: Box<str>,
    /// Whether this is a partition or a file.
    pub kind: SwapType,
    /// Total size of the swap area in kibibytes.
    pub size: Kibibytes,
    /// Amount of swap space currently in use, in kibibytes.
    pub used: Kibibytes,
    /// Swap priority. May be negative; the kernel default is `-1`.
    pub priority: i32,
}

/// Reads `/proc/swaps` and returns all configured swap areas.
///
/// The file has a `Filename Type Size Used Priority` header followed by
/// one tab-aligned line per swap area. Columns are padded with spaces
/// and tabs, so splitting on whitespace yields the five fields cleanly.
/// Systems with no swap configured produce an empty list.
pub fn swaps() -> Result<Vec<Swap>> {
    let path = Path::new("/proc/swaps");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<5>::new(line);

        // Skip the header line rather than assuming it is line zero.
        if fields.first() == Some(&&b"Filename"[..]) {
            continue;
        }
        if fields.len() != 5 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<filename> <type> <size> <used> <priority>'",
            });
        }

        let filename = std::str::from_utf8(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid filename",
        })?;

        let kind = match fields[1] {
            b"partition" => SwapType::Partition,
            b"file" => SwapType::File,
            _ => {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "unknown swap type",
                });
            }
        };

        let size = parse::parse_dec_u64(fields[2])
            .map(Kibibytes)
            .map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid size",
            })?;
        let used = parse::parse_dec_u64(fields[3])
            .map(Kibibytes)
            .map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid used",
            })?;
        let priority = parse::parse_dec_i64(fields[4]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid priority",
        })? as i32;

        out.push(Swap {
            filename: filename.into(),
            kind,
            size,
            used,
            priority,
        });
    }

    Ok(out)
}
