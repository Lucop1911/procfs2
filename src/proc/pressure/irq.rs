use std::path::Path;

use super::{PressureEntry, parse_pressure_line};
use crate::error::{Error, Result};
use crate::util::parse;

/// IRQ pressure stall information from `/proc/pressure/irq`.
///
/// Only the `full` line is reported. It measures the percentage of
/// wall-clock time during which *all* CPUs were blocked servicing
/// hardware interrupts. The `some` line is omitted by the kernel
/// because at least one CPU is always available to handle interrupts
/// on a non-degenerate system.
#[derive(Debug)]
pub struct IrqPressure {
    /// All CPUs blocked on IRQ handling.
    pub full: PressureEntry,
}

/// Reads `/proc/pressure/irq` and returns [`IrqPressure`].
pub fn irq_pressure() -> Result<IrqPressure> {
    let path = Path::new("/proc/pressure/irq");
    let bytes = parse::read_file(path)?;

    let mut full = None;

    for line in bytes.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let (label, entry) = parse_pressure_line(line)?;

        if label == b"full" {
            full = Some(entry);
        }
    }
    Ok(IrqPressure {
        full: full.ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            line: 0,
            msg: "missing full line",
        })?,
    })
}
