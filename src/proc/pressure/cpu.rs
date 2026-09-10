use std::path::Path;

use super::{PressureEntry, parse_pressure_line};
use crate::error::{Error, Result};
use crate::util::parse;

/// CPU pressure stall information from `/proc/pressure/cpu`.
///
/// The `some` line measures the percentage of wall-clock time during
/// which at least one runnable task was waiting for CPU time. The
/// `full` line measures the percentage of wall-clock time during which
/// *all* runnable tasks were stalled — this is always zero on
/// non-containerized systems because the scheduler always picks one.
#[derive(Debug)]
pub struct CpuPressure {
    /// At least one task stalled waiting for CPU.
    pub some: PressureEntry,
    /// All tasks stalled waiting for CPU.
    pub full: PressureEntry,
}

/// Reads `/proc/pressure/cpu` and returns [`CpuPressure`].
pub fn cpu_pressure() -> Result<CpuPressure> {
    let path = Path::new("/proc/pressure/cpu");
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
    Ok(CpuPressure {
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
