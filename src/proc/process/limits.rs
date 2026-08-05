use crate::error::Result;
use crate::util::parse;

/// A single resource limit entry.
///
/// Sourced from the `/proc/PID/limits` table. Soft and hard limits
/// are `None` when the limit is set to `unlimited`.
#[derive(Debug)]
pub struct Limit {
    /// Soft limit — the value enforced by the kernel.
    ///
    /// A process may lower its soft limit up to the hard limit
    /// via `setrlimit()`.
    pub soft: Option<u64>,
    /// Hard limit — the ceiling for the soft limit.
    ///
    /// Only a privileged process (with `CAP_SYS_RESOURCE`) may
    /// raise the hard limit.
    pub hard: Option<u64>,
    /// The unit of measurement for this limit.
    pub unit: LimitUnit,
}

/// The unit of measurement for a resource limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitUnit {
    /// Number of files.
    Files,
    /// Number of bytes.
    Bytes,
    /// Number of processes/threads.
    Processes,
    /// Number of seconds.
    Seconds,
    /// Number of microseconds (used for realtime timeout).
    Microseconds,
}

/// Resource limits for a process.
///
/// Parsed from `/proc/PID/limits`, which uses a fixed-width table
/// format:
/// ```text
/// Limit                     Soft Limit           Hard Limit           Units
/// Max cpu time              unlimited            unlimited            seconds
/// Max file size             unlimited            unlimited            bytes
/// ...
/// ```
#[derive(Debug)]
pub struct ProcessLimits {
    /// Maximum CPU time per process (`RLIMIT_CPU`).
    pub max_cpu_time: Limit,
    /// Maximum file size a process may create (`RLIMIT_FSIZE`).
    pub max_fsize: Limit,
    /// Maximum size of the process's data segment (`RLIMIT_DATA`).
    pub max_data: Limit,
    /// Maximum size of the stack (`RLIMIT_STACK`).
    pub max_stack: Limit,
    /// Maximum size of a core dump file (`RLIMIT_CORE`).
    pub max_core: Limit,
    /// Maximum resident set size (`RLIMIT_RSS`).
    pub max_rss: Limit,
    /// Maximum amount of memory that may be locked (`RLIMIT_MEMLOCK`).
    pub max_locked: Limit,
    /// Maximum size of the process's virtual address space (`RLIMIT_AS`).
    pub max_addr_space: Limit,
    /// Maximum number of open file descriptors (`RLIMIT_NOFILE`).
    pub max_open_files: Limit,
    /// Maximum number of processes for this real user ID (`RLIMIT_NPROC`).
    pub max_processes: Limit,
    /// Maximum nice priority value (`RLIMIT_NICE`).
    pub max_nice: Limit,
    /// Maximum realtime scheduling priority (`RLIMIT_RTPRIO`).
    pub max_rt_priority: Limit,
    /// Maximum realtime timeout in microseconds (`RLIMIT_RTTIME`).
    pub max_rt_time: Limit,
}

impl ProcessLimits {
    /// Parses `/proc/PID/limits` from raw bytes.
    ///
    /// The first line is a header and is skipped. Each subsequent
    /// line has a 25-character fixed-width limit name followed by
    /// three whitespace-separated fields: soft limit, hard limit,
    /// and unit.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut limits = ProcessLimits {
            max_cpu_time: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Seconds,
            },
            max_fsize: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_data: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_stack: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_core: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_rss: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_locked: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_addr_space: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Bytes,
            },
            max_open_files: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Files,
            },
            max_processes: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Processes,
            },
            max_nice: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Files,
            },
            max_rt_priority: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Files,
            },
            max_rt_time: Limit {
                soft: None,
                hard: None,
                unit: LimitUnit::Microseconds,
            },
        };

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            let trimmed = parse::trim_start(line);

            // Skip the header row.
            if trimmed.starts_with(b"Limit") {
                continue;
            }

            // The limit name occupies the first 25 characters.
            let name_end = 25.min(trimmed.len());
            let name = parse::trim_end(&trimmed[..name_end]);
            let rest = parse::trim_start(&trimmed[name_end..]);

            let fields: Vec<&[u8]> = parse::split_spaces(rest);
            if fields.len() < 3 {
                continue;
            }

            let soft = parse_limit_value(fields[0]);
            let hard = parse_limit_value(fields[1]);
            let unit = parse_limit_unit(fields[2]);

            let limit = Limit { soft, hard, unit };

            match name {
                b"Max cpu time" => limits.max_cpu_time = limit,
                b"Max file size" => limits.max_fsize = limit,
                b"Max data size" => limits.max_data = limit,
                b"Max stack size" => limits.max_stack = limit,
                b"Max core file size" => limits.max_core = limit,
                b"Max resident set" => limits.max_rss = limit,
                b"Max locked memory" => limits.max_locked = limit,
                b"Max address space" => limits.max_addr_space = limit,
                b"Max open files" => limits.max_open_files = limit,
                b"Max processes" => limits.max_processes = limit,
                b"Max nice priority" => limits.max_nice = limit,
                b"Max realtime priority" => limits.max_rt_priority = limit,
                b"Max realtime timeout" => limits.max_rt_time = limit,
                _ => {}
            }
        }

        Ok(limits)
    }
}

/// Parses a limit value, returning `None` for `unlimited`.
fn parse_limit_value(s: &[u8]) -> Option<u64> {
    if s == b"unlimited" {
        None
    } else {
        parse::parse_dec_u64(s).ok()
    }
}

/// Maps a unit string to the corresponding [`LimitUnit`].
fn parse_limit_unit(s: &[u8]) -> LimitUnit {
    match s {
        b"bytes" => LimitUnit::Bytes,
        b"files" => LimitUnit::Files,
        b"processes" => LimitUnit::Processes,
        b"seconds" => LimitUnit::Seconds,
        _ => LimitUnit::Files,
    }
}
