use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Per-CPU hardware interrupt (`irq`) counters from `/proc/interrupts`.
///
/// The file contains numbered IRQ lines (device and architecture
/// interrupts) and named aggregate counters (`NMI`, `LOC`, `CAL`, ...).
/// All values are cumulative since boot; sample twice and subtract to
/// get rates.
#[derive(Debug)]
pub struct Interrupts {
    /// CPU ids from the header line, ascending.
    ///
    /// Each row's counts align with this vector by position, not by id.
    pub cpus: Vec<u32>,
    /// One entry per numbered IRQ line, in file order.
    pub rows: Vec<InterruptRow>,
    /// Named aggregate counters from the bottom of the file (`NMI`,
    /// `LOC`, `CAL`, ...), in file order.
    pub counts: Vec<InterruptCount>,
}

/// Counts for one numbered IRQ line across all CPUs.
#[derive(Debug)]
pub struct InterruptRow {
    /// The IRQ number preceding the `:` (`0` is the timer on x86).
    pub irq: u64,
    /// Times this IRQ has fired on each CPU, aligned with
    /// [`Interrupts::cpus`] by position.
    pub per_cpu: Vec<u64>,
    /// Everything after the per-CPU counts, verbatim (tokens re-joined
    /// with single spaces): controller, trigger mode, device name,
    /// e.g. `IR-IO-APIC 2-edge timer`. The column layout differs
    /// between architectures and kernel versions.
    pub info: Box<str>,
}

/// One named aggregate line (`NMI`, `LOC`, `TLB`, ...) below the IRQ
/// table.
///
/// Most lines carry one counter per CPU; a few (`ERR`, `MIS`) carry a
/// single system-wide total instead.
#[derive(Debug)]
pub struct InterruptCount {
    /// Counter name, without the trailing `:`.
    ///
    /// The set is architecture-specific and grows across kernel
    /// versions; names are kept as-is.
    pub name: Box<str>,
    /// One value per CPU aligned with [`Interrupts::cpus`], or a lone
    /// total for system-wide counters like `ERR` and `MIS`.
    pub values: Vec<u64>,
    /// Trailing description, e.g. `TLB shootdowns`. Empty if the line
    /// has none.
    pub description: Box<str>,
}

/// Reads `/proc/interrupts` and returns per-CPU interrupt counters.
///
/// The first line names the online CPUs (`CPU0`, `CPU1`, ...); every
/// following line is either a numbered IRQ or a named aggregate
/// counter. Per-CPU count columns are split with heap
/// [`parse::split_spaces`] rather than the fixed-size
/// [`parse::SplitFields`] buffer because column count grows with CPU
/// count. A row with fewer columns than the header (a CPU going
/// offline mid-read) yields a shorter count vector instead of an error.
pub fn interrupts() -> Result<Interrupts> {
    let path = Path::new("/proc/interrupts");
    let bytes = parse::read_file(path)?;

    let mut cpus: Vec<u32> = Vec::new();
    let mut rows: Vec<InterruptRow> = Vec::new();
    let mut counts: Vec<InterruptCount> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::split_spaces(line);
        if fields.is_empty() {
            continue;
        }

        if fields[0].starts_with(b"CPU") {
            cpus.clear();
            for &field in &fields {
                let id = field
                    .strip_prefix(&b"CPU"[..])
                    .ok_or_else(|| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid CPU column",
                    })?;
                cpus.push(parse::parse_dec_u32(id).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid CPU id",
                })?);
            }
            continue;
        }

        let (label, _) = parse::split_at_byte(fields[0], b':');
        if label.len() == fields[0].len() {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected ':' after label",
            });
        }

        if label.iter().all(|b| b.is_ascii_digit()) {
            let irq = parse::parse_dec_u64(label).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid irq number",
            })?;

            let mut per_cpu = Vec::new();
            let mut tail_start = 1usize;

            for &field in &fields[1..] {
                match parse::parse_dec_u64(field) {
                    Ok(v) => {
                        per_cpu.push(v);
                        tail_start += 1;
                    }
                    Err(_) => break,
                }
            }

            if per_cpu.is_empty() {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected per-cpu counts",
                });
            }

            let info = join_tail(&fields[tail_start..])?;

            rows.push(InterruptRow {
                irq,
                per_cpu,
                info: info.into(),
            });
        } else {
            let name = std::str::from_utf8(label).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid counter name",
            })?;

            let mut values = Vec::new();
            let mut tail_start = 1usize;

            for &field in &fields[1..] {
                match parse::parse_dec_u64(field) {
                    Ok(v) => {
                        values.push(v);
                        tail_start += 1;
                    }
                    Err(_) => break,
                }
            }

            if values.is_empty() {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected counter values",
                });
            }

            let description = join_tail(&fields[tail_start..])?;

            counts.push(InterruptCount {
                name: name.into(),
                values,
                description: description.into(),
            });
        }
    }

    Ok(Interrupts { cpus, rows, counts })
}

fn join_tail(fields: &[&[u8]]) -> Result<String> {
    let mut tokens = Vec::with_capacity(fields.len());
    for &f in fields {
        let s = std::str::from_utf8(f).map_err(|_| Error::Parse {
            path: Path::new("/proc/interrupts").to_path_buf(),
            line: 0,
            msg: "non-UTF8 info field",
        })?;
        tokens.push(s);
    }
    Ok(tokens.join(" "))
}
