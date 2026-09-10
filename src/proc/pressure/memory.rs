use std::path::Path;

use super::{PressureEntry, parse_pressure_line};
use crate::error::{Error, Result};
use crate::util::parse;

/// Memory pressure stall information from `/proc/pressure/memory`.
///
/// The `some` line measures the percentage of wall-clock time during
/// which at least one task was stalled due to memory pressure (e.g.
/// direct reclaim, compaction, swapping). The `full` line measures the
/// percentage of wall-clock time during which *all* tasks were stalled.
#[derive(Debug)]
pub struct MemoryPressure {
    /// At least one task stalled on memory.
    pub some: PressureEntry,
    /// All tasks stalled on memory.
    pub full: PressureEntry,
}

/// Reads `/proc/pressure/memory` and returns [`MemoryPressure`].
pub fn memory_pressure() -> Result<MemoryPressure> {
    let path = Path::new("/proc/pressure/memory");
    let bytes = parse::read_file(path)?;

    let mut some = None;
    let mut full = None;

    for line in bytes.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let (label, entry) = parse_pressure_line(line)?;
        match label {
            b"some" => some = Some(entry),
            b"full" => full = Some(entry),
            _ => {}
        }
    }
    Ok(MemoryPressure {
        some: some.ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            line: 0,
            msg: "missing some line",
        })?,
        full: full.ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            line: 0,
            msg: "missing full line",
        })?,
    })
}
