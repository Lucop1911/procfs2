use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A single I/O port region from `/proc/ioports`.
///
/// Each line maps a port address range to a description (e.g.
/// `dma1`, `timer0`, `PCI Bus 0000:00`). Indentation encodes
/// the nesting depth: root entries are unindented, child entries
/// are indented by two spaces per level.
#[derive(Debug)]
pub struct IoPort {
    /// Start port number (inclusive, hex-decoded).
    pub start: u32,
    /// End port number (inclusive, hex-decoded).
    pub end: u32,
    /// Human-readable description (e.g. `timer0`, `PCI Bus 0000:00`).
    pub name: Box<str>,
    /// Nesting depth (0 = root, +1 per indentation level).
    pub depth: usize,
}

/// Reads `/proc/ioports` and returns a flat list of I/O port
/// regions in file order.
///
/// The file uses indentation (two spaces per level) to express a
/// tree of port regions. The result is flat with a [`depth`](IoPort::depth)
/// field so callers can reconstruct the hierarchy or just iterate
/// linearly. Addresses are hex without the `0x` prefix.
pub fn ioports() -> Result<Vec<IoPort>> {
    let path = Path::new("/proc/ioports");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<IoPort> = Vec::new();

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

        let start = parse::parse_hex_u64(start_bytes)
            .map(|v| v as u32)
            .map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid start",
            })?;

        let end = parse::parse_hex_u64(end_bytes)
            .map(|v| v as u32)
            .map_err(|_| Error::Parse {
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

        out.push(IoPort {
            start,
            end,
            name: name.into(),
            depth,
        });
    }

    Ok(out)
}
