/// CPU pressure from `/proc/pressure/cpu`.
pub mod cpu;
/// I/O pressure from `/proc/pressure/io`.
pub mod io;
/// IRQ pressure from `/proc/pressure/irq`.
pub mod irq;
/// Memory pressure from `/proc/pressure/memory`.
pub mod memory;

pub use cpu::{CpuPressure, cpu_pressure};
pub use io::{IoPressure, io_pressure};
pub use irq::{IrqPressure, irq_pressure};
pub use memory::{MemoryPressure, memory_pressure};

use crate::error::Result;
use crate::util::parse::{parse_dec_f64, parse_dec_u64, split_at_byte};

/// A single pressure stall information (PSI) line.
///
/// Each line in `/proc/pressure/*` carries four values: three
/// exponentially-decaying averages (10 s, 60 s, 300 s) representing the
/// percentage of wall-clock time that at least one task (or all tasks,
/// for the `full` line) was stalled on the given resource, plus a
/// cumulative `total` in microseconds.
#[derive(Debug)]
pub struct PressureEntry {
    /// 10-second average stall percentage.
    pub avg10: f64,
    /// 60-second average stall percentage.
    pub avg60: f64,
    /// 300-second average stall percentage.
    pub avg300: f64,
    /// Cumulative stall time in microseconds since boot.
    pub total: u64,
}

/// Parses a single PSI line of the form `TYPE avg10=X avg60=Y avg300=Z total=N`.
///
/// Returns the label (`some` or `full`) and the parsed values.
pub(crate) fn parse_pressure_line(line: &[u8]) -> Result<(&[u8], PressureEntry)> {
    let (label, rest) = split_at_byte(line, b' ');

    let mut avg10 = 0.0;
    let mut avg60 = 0.0;
    let mut avg300 = 0.0;
    let mut total = 0u64;

    for token in rest.split(|&b| b == b' ') {
        if token.is_empty() {
            continue;
        }
        let (key, val) = split_at_byte(token, b'=');
        match key {
            b"avg10" => avg10 = parse_dec_f64(val)?,
            b"avg60" => avg60 = parse_dec_f64(val)?,
            b"avg300" => avg300 = parse_dec_f64(val)?,
            b"total" => total = parse_dec_u64(val)?,
            _ => {}
        }
    }
    Ok((
        label,
        PressureEntry {
            avg10,
            avg60,
            avg300,
            total,
        },
    ))
}
