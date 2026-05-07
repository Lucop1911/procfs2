use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::Jiffies;

/// Aggregate CPU time counters for a single logical CPU or the
/// system-wide total.
///
/// All values are in jiffies (clock ticks). On Linux `/proc` the
/// tick rate is `HZ = 100` (1 jiffy = 10 ms).
#[derive(Debug)]
pub struct CpuTime {
    /// Time spent in user mode.
    pub user: Jiffies,
    /// Time spent in user mode with low priority (nice).
    pub nice: Jiffies,
    /// Time spent in kernel mode.
    pub system: Jiffies,
    /// Time spent idle (excluding iowait).
    pub idle: Jiffies,
    /// Time spent waiting for I/O to complete.
    pub iowait: Jiffies,
    /// Time spent servicing hardware interrupts.
    pub irq: Jiffies,
    /// Time spent servicing software interrupts.
    pub softirq: Jiffies,
    /// Time stolen by the hypervisor in a virtualized environment.
    pub steal: Jiffies,
    /// Time spent running a virtual CPU for a guest OS (counted in
    /// user time as well, so usually equal to `user` on the guest).
    pub guest: Jiffies,
    /// Time spent running a guest with low priority (nice).
    pub guest_nice: Jiffies,
}

/// Per-CPU time counters.
#[derive(Debug)]
pub struct PerCpuTime {
    /// CPU index (0-based).
    pub id: u32,
    pub times: CpuTime,
}

/// System-wide statistics from `/proc/stat`.
///
/// Includes aggregate and per-CPU time counters, context switch
/// count, boot time, and process statistics.
#[derive(Debug)]
pub struct SystemStat {
    /// Aggregate CPU time across all cores.
    pub cpu_total: CpuTime,
    /// Per-CPU time counters, one entry per logical CPU.
    pub per_cpu: Vec<PerCpuTime>,
    /// Total number of context switches since boot.
    pub ctxt: u64,
    /// Boot time as a Unix timestamp (seconds since epoch).
    pub btime: u64,
    /// Total number of forks since boot.
    pub processes: u64,
    /// Number of processes currently in the running state.
    pub procs_running: u64,
    /// Number of processes currently blocked on I/O.
    pub procs_blocked: u64,
}

fn parse_cpu_times(fields: &[&[u8]]) -> Result<CpuTime> {
    let get = |idx: usize| -> Result<Jiffies> {
        if idx < fields.len() {
            parse::parse_dec_u64(fields[idx]).map(Jiffies)
        } else {
            Ok(Jiffies(0))
        }
    };

    Ok(CpuTime {
        user: get(0)?,
        nice: get(1)?,
        system: get(2)?,
        idle: get(3)?,
        iowait: get(4)?,
        irq: get(5)?,
        softirq: get(6)?,
        steal: get(7)?,
        guest: get(8)?,
        guest_nice: get(9)?,
    })
}

/// Reads `/proc/stat` and returns [`SystemStat`].
///
/// Parses the `cpu` aggregate line, all `cpuN` per-CPU lines, and
/// the `ctxt`, `btime`, `processes`, `procs_running`, and
/// `procs_blocked` entries. Other `/proc/stat` lines (interrupts,
/// softirqs, etc.) are intentionally ignored — they may be added in
/// future modules.
pub fn stat() -> Result<SystemStat> {
    let bytes = parse::read_file(std::path::Path::new("/proc/stat"))?;
    let path = std::path::PathBuf::from("/proc/stat");

    let mut cpu_total = None;
    let mut per_cpu = Vec::new();
    let mut ctxt = None;
    let mut btime = None;
    let mut processes = None;
    let mut procs_running = None;
    let mut procs_blocked = None;

    for (line_num, line) in bytes.split(|&b| b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }

        let fields = parse::split_spaces(line);
        if fields.is_empty() {
            continue;
        }

        let key = fields[0];

        if key == b"cpu" {
            cpu_total = Some(parse_cpu_times(&fields[1..])?);
        } else if key.starts_with(b"cpu") {
            let id_str = &key[3..];
            let id = parse::parse_dec_u32(id_str).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid cpu id",
            })?;
            let times = parse_cpu_times(&fields[1..])?;
            per_cpu.push(PerCpuTime { id, times });
        } else if key == b"ctxt" {
            ctxt = Some(parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid ctxt",
            })?);
        } else if key == b"btime" {
            btime = Some(parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid btime",
            })?);
        } else if key == b"processes" {
            processes = Some(parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid processes",
            })?);
        } else if key == b"procs_running" {
            procs_running = Some(parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid procs_running",
            })?);
        } else if key == b"procs_blocked" {
            procs_blocked = Some(parse::parse_dec_u64(fields[1]).map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid procs_blocked",
            })?);
        }
    }

    Ok(SystemStat {
        cpu_total: cpu_total.unwrap_or(CpuTime {
            user: Jiffies(0),
            nice: Jiffies(0),
            system: Jiffies(0),
            idle: Jiffies(0),
            iowait: Jiffies(0),
            irq: Jiffies(0),
            softirq: Jiffies(0),
            steal: Jiffies(0),
            guest: Jiffies(0),
            guest_nice: Jiffies(0),
        }),
        per_cpu,
        ctxt: ctxt.unwrap_or(0),
        btime: btime.unwrap_or(0),
        processes: processes.unwrap_or(0),
        procs_running: procs_running.unwrap_or(0),
        procs_blocked: procs_blocked.unwrap_or(0),
    })
}
