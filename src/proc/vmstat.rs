use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Reads `/proc/vmstat` and returns virtual memory statistics.
///
/// The file holds one `key value` line per counter (e.g.
/// `nr_free_pages 1974792`). Since the exact set of statistics varies
/// from kernel to kernel, and because most of them are not well
/// documented, this returns a `HashMap` instead of a fixed struct.
/// Values are `u64` (kernel counters are `unsigned long`).
pub fn vmstat() -> Result<HashMap<String, u64>> {
    let path = Path::new("/proc/vmstat");
    let bytes = parse::read_file(path)?;

    let mut out: HashMap<String, u64> = HashMap::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<2>::new(line);

        if fields.len() != 2 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected '<key> <value>'",
            });
        }

        let key = String::from(std::str::from_utf8(fields[0]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid key",
        })?);

        let value = parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid value",
        })?;

        out.insert(key, value);
    }

    Ok(out)
}
