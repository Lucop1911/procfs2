use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A single physical memory region from `/proc/iomem`.
///
/// Each line in the file maps a physical address range to a
/// description (e.g. `System RAM`, `PCI Bus 0000:00`, `nvidia`).
/// Indentation encodes the nesting depth: root entries are
/// unindented, child entries are indented by two spaces per level.
#[derive(Debug)]
pub struct IoMem {
    /// Start address of the region (inclusive, physical).
    pub start: u64,
    /// End address of the region (inclusive, physical).
    pub end: u64,
    /// Human-readable description (e.g. `System RAM`, `PCI Bus 0000:00`).
    pub name: Box<str>,
    /// Nesting depth (0 = root, +1 per indentation level).
    pub depth: usize,
}

/// Reads `/proc/iomem` and returns a flat list of physical memory
/// regions in file order.
///
/// The file uses indentation (two spaces per level) to express a
/// tree of memory regions. The result is flat with a [`depth`](IoMemEntry::depth)
/// field so callers can reconstruct the hierarchy or just iterate
/// linearly. Addresses are parsed as hex without the `0x` prefix.
pub fn iomem() -> Result<Vec<IoMem>> {
    let path = Path::new("/proc/iomem");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<IoMem> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let trimmed = parse::trim_start(line);
        let depth = (line.len() - trimmed.len()) / 2;

        let (range, name_bytes) = parse::split_at_byte(trimmed, b':');
        if name_bytes.is_empty() {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid name bytes",
            });
        }

        let (start_bytes, end_bytes) = parse::split_at_byte(range, b'-');

        let start = parse::parse_hex_u64(start_bytes).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid start",
        })?;

        let end = parse::parse_hex_u64(end_bytes).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid end",
        })?;

        let name =
            std::str::from_utf8(parse::trim_start(name_bytes)).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid name",
            })?;

        out.push(IoMem {
            start,
            end,
            name: name.into(),
            depth,
        });
    }

    Ok(out)
}
