use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Per-CPU software interrupt (`softirq`) counters from `/proc/softirqs`.
///
/// Values are cumulative since boot; sample twice and subtract to get
/// rates.
#[derive(Debug)]
pub struct Softirqs {
    /// CPU ids from the header line, ascending.
    ///
    /// Each row's counts align with this vector by position, not by id.
    pub cpus: Vec<u32>,
    /// One entry per softirq type, in file order.
    pub rows: Vec<SoftirqRow>,
}

/// Counts for one softirq type across all CPUs.
#[derive(Debug)]
pub struct SoftirqRow {
    /// Softirq name, without the trailing `:`.
    ///
    /// The kernel fixes the set (`HI`, `TIMER`, `NET_TX`, `NET_RX`,
    /// `BLOCK`, `IRQ_POLL`, `TASKLET`, `SCHED`, `HRTIMER`, `RCU`);
    /// names from future kernels are kept as-is.
    pub name: Box<str>,
    /// Times this softirq has run on each CPU, aligned with
    /// [`Softirqs::cpus`] by position.
    pub per_cpu: Vec<u64>,
}

/// Reads `/proc/softirqs` and returns per-CPU softirq counters.
///
/// The first line names the online CPUs (`CPU0`, `CPU1`, ...); every
/// following line holds one softirq type and its per-CPU counts.
/// Counts are read column by column, so a row with fewer columns than
/// the header (a CPU going offline mid-read) yields a shorter
/// [`SoftirqRow::per_cpu`] instead of an error.
pub fn softirqs() -> Result<Softirqs> {
    let path = Path::new("/proc/softirqs");
    let bytes = parse::read_file(path)?;

    let mut cpus: Vec<u32> = Vec::new();
    let mut rows: Vec<SoftirqRow> = Vec::new();

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
        } else {
            if fields.len() < 2 {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<name>: <per-cpu counts>'",
                });
            }

            let (name, _) = parse::split_at_byte(fields[0], b':');
            let name = std::str::from_utf8(name).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid softirq name",
            })?;

            let mut per_cpu = Vec::with_capacity(fields.len() - 1);
            for &field in &fields[1..] {
                per_cpu.push(parse::parse_dec_u64(field).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid counter",
                })?);
            }
            rows.push(SoftirqRow {
                name: name.into(),
                per_cpu,
            });
        }
    }

    Ok(Softirqs { cpus, rows })
}
