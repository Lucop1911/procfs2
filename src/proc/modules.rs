use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A loaded kernel module from `/proc/modules`.
///
/// Each line lists one currently loaded module with its size,
/// reference count, dependencies, state, and kernel address.
#[derive(Debug)]
pub struct Module {
    /// Module name (e.g. `ext4`, `nvidia`).
    pub name: Box<str>,
    /// Module size in bytes.
    pub size: u64,
    /// Number of references held to this module.
    pub ref_count: u64,
    /// Comma-separated dependency list, or `"-"` if none.
    pub deps: Box<str>,
    /// Module state: `"Live"`, `"Loading"`, or `"Unloading"`.
    pub state: Box<str>,
    /// Kernel address of the module code (hex with `0x` prefix).
    pub address: u64,
}

/// Reads `/proc/modules` and returns all loaded kernel modules.
///
/// Each line is whitespace-separated: `name size refcount deps
/// state address`. Dependencies are a trailing-comma-separated
/// list of module names, or `"-"` for none.
pub fn modules() -> Result<Vec<Module>> {
    let path = Path::new("/proc/modules");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<Module> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<6>::new(line);

        let name = std::str::from_utf8(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid name",
        })?;

        let size = parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid size",
        })?;

        let ref_count = parse::parse_dec_u64(fields[2]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid reference count",
        })?;

        let deps = std::str::from_utf8(fields[3]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid deps",
        })?;

        let state = std::str::from_utf8(fields[4]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid state",
        })?;

        let address = parse::parse_hex_u64(fields[5]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid address",
        })?;

        out.push(Module {
            name: name.into(),
            size,
            ref_count,
            deps: deps.into(),
            state: state.into(),
            address,
        });
    }
    Ok(out)
}
