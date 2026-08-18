use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Free list sizes for one node/zone from `/proc/buddyinfo`.
///
/// A chunk of order `i` covers `2^i` consecutive pages.
#[derive(Debug)]
pub struct BuddyInfo {
    /// NUMA node ID.
    pub node: u32,
    /// Memory zone name.
    pub zone: Box<str>,
    /// Free chunks per buddy order.
    pub free_lists: Vec<u64>,
}

/// Reads `/proc/buddyinfo` and returns the per-zone free list sizes.
///
/// Lines are of the form `Node <id>, zone <name> <order 0> <order 1> ...`.
/// The number of order columns equals the kernel's `MAX_ORDER`, which
/// varies across kernels, so each line's counts are stored in a [`Vec`].
/// Only populated zones are listed.
pub fn buddyinfo() -> Result<Vec<BuddyInfo>> {
    let path = Path::new("/proc/buddyinfo");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<BuddyInfo> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<32>::new(line);

        if fields.len() < 5 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<Node> <id> <zone> <order counts>'",
            });
        }

        let (node, _) = parse::split_at_byte(fields[1], b',');

        let node = parse::parse_dec_u32(node).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid node",
        })?;

        let zone = std::str::from_utf8(fields[3]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid zone",
        })?;

        let mut free_lists = Vec::with_capacity(fields.len() - 4);
        for field in &fields[4..] {
            free_lists.push(parse::parse_dec_u64(field).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid free list count",
            })?);
        }

        out.push(BuddyInfo {
            node,
            zone: zone.into(),
            free_lists,
        });
    }

    Ok(out)
}
