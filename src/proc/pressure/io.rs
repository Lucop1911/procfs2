use std::path::Path;

use super::{PressureEntry, parse_pressure_line};
use crate::error::{Error, Result};
use crate::util::parse;

/// I/O pressure stall information from `/proc/pressure/io`.
///
/// The `some` line measures the percentage of wall-clock time during
/// which at least one task was stalled waiting for I/O to complete. The
/// `full` line measures the percentage of wall-clock time during which
/// *all* tasks were stalled on I/O.
#[derive(Debug)]
pub struct IoPressure {
    /// At least one task stalled on I/O.
    pub some: PressureEntry,
    /// All tasks stalled on I/O.
    pub full: PressureEntry,
}

/// Reads `/proc/pressure/io` and returns [`IoPressure`].
pub fn io_pressure() -> Result<IoPressure> {
    let path = Path::new("/proc/pressure/io");
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
    Ok(IoPressure {
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
