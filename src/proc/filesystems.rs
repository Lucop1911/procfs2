use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A filesystem type registered with the kernel, from `/proc/filesystems`.
#[derive(Debug, Clone)]
pub struct FileSystem {
    /// Filesystem type name (e.g. `ext4`, `tmpfs`, `proc`).
    pub name: Box<str>,
    /// Whether mounting this filesystem requires a block device.
    ///
    /// Device-backed filesystems (e.g. `ext4`, `vfat`) print no `nodev`
    /// flag, so `dev` is `true`. Pseudo-filesystems (e.g. `proc`,
    /// `tmpfs`) are prefixed with `nodev`, so `dev` is `false`.
    pub dev: bool,
}

/// Reads `/proc/filesystems` and returns all filesystem types supported
/// by the running kernel.
///
/// The kernel emits one line per filesystem, e.g. `nodev` + tab + `proc`
/// for pseudo-filesystems, or a bare tab + `ext4` for device-backed ones.
/// The flag field is either the literal `nodev` (no block device needed)
/// or empty (device-backed); the name always follows a tab.
pub fn filesystems() -> Result<Vec<FileSystem>> {
    let path = Path::new("/proc/filesystems");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        // Split at the always-present tab: `flag<tab>name`. The flag is
        // empty for device-backed filesystems, so `SplitFields` (which
        // skips empty fields) cannot be used here.
        let (flag, name) = parse::split_at_byte(line, b'\t');
        if name.is_empty() {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<flag><TAB><name>'",
            });
        }

        let name = std::str::from_utf8(name).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        out.push(FileSystem {
            name: name.into(),
            dev: flag.is_empty(),
        });
    }

    Ok(out)
}
