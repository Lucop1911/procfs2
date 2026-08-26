use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A misc character device from `/proc/misc`.
///
/// Each line maps a minor number to a device name (e.g. `fuse`,
/// `device-mapper`, `kvm`). Misc devices are a catch-all category
/// for character devices that don't fit into a major device class.
#[derive(Debug)]
pub struct Misc {
    /// Minor number (0–255, kernel limit for misc devices).
    pub minor: u8,
    /// Device name (e.g. `fuse`, `kvm`).
    pub name: Box<str>,
}

/// Reads `/proc/misc` and returns the registered misc character
/// devices.
///
/// The file is a flat table of `<minor>  <name>` lines with no
/// header. Minor numbers are assigned by the kernel at device
/// registration time.
pub fn misc() -> Result<Vec<Misc>> {
    let path = Path::new("/proc/misc");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<Misc> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<2>::new(line);

        let minor = parse::parse_dec_u32(fields[0])
            .map(|n| n as u8)
            .map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid minor",
            })?;

        let name = std::str::from_utf8(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        out.push(Misc {
            minor,
            name: name.into(),
        });
    }

    Ok(out)
}
