use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// The device class of an entry in `/proc/devices`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// A character device.
    Character,
    /// A block device.
    Block,
}

/// A single device driver entry from `/proc/devices`.
///
/// A major number may appear multiple times with different names
/// (e.g. `195 nvidia`, `195 nvidiactl`), so each line is a distinct
/// [`Device`].
#[derive(Debug, Clone)]
pub struct Device {
    /// The device major number.
    pub major: u64,
    /// The driver name (may contain `/`, `-`, `.`; never spaces).
    pub name: Box<str>,
    /// Whether this is a character or block device.
    pub kind: DeviceKind,
}

/// Reads `/proc/devices` and returns all registered device drivers.
///
/// The file contains two sections (`Character devices:` and
/// `Block devices:`), each with one `major name` line per registered
/// driver. A major number can be listed multiple times.
pub fn devices() -> Result<Vec<Device>> {
    let path = Path::new("/proc/devices");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();
    let mut kind = None;

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        match line {
            b"Character devices:" => {
                kind = Some(DeviceKind::Character);
                continue;
            }
            b"Block devices:" => {
                kind = Some(DeviceKind::Block);
                continue;
            }
            _ => {}
        }

        let Some(ref cur_kind) = kind else {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "entry before any section header",
            });
        };

        let fields = parse::SplitFields::<2>::new(line);
        if fields.len() != 2 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<major> <name>'",
            });
        }

        let major = parse::parse_dec_u64(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid major number",
        })?;

        let name = std::str::from_utf8(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        out.push(Device {
            major,
            name: name.into(),
            kind: *cur_kind,
        });
    }

    Ok(out)
}
