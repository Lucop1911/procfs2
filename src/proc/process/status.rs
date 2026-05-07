use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::Kibibytes;

/// Process state as reported in `/proc/PID/status`.
///
/// Maps the single-character `State:` field to a typed enum.
/// The kernel uses these codes:
/// - `R` — running or runnable
/// - `S` — interruptible sleep (waiting for an event)
/// - `D` — uninterruptible sleep (usually I/O)
/// - `Z` — zombie (terminated but not reaped)
/// - `T` — stopped (by job control signal)
/// - `t` — stopped (by debugger during tracing)
/// - `X` — dead (should never be seen in `/proc`)
/// - `I` — idle kernel thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Zombie,
    Stopped,
    Dead,
    Idle,
    WakeKill,
    Waking,
    Parked,
}

impl ProcessState {
    fn from_char(c: char) -> Self {
        match c {
            'R' => ProcessState::Running,
            'S' => ProcessState::Sleeping,
            'D' => ProcessState::Waiting,
            'Z' => ProcessState::Zombie,
            'T' | 't' => ProcessState::Stopped,
            'X' | 'x' => ProcessState::Dead,
            'I' => ProcessState::Idle,
            'K' => ProcessState::WakeKill,
            'W' => ProcessState::Waking,
            'P' => ProcessState::Parked,
            _ => ProcessState::Dead,
        }
    }
}

/// User IDs associated with a process.
///
/// All four IDs come from the `Uid:` line in `/proc/PID/status`,
/// which reports them as tab-separated decimal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Uids {
    /// Real UID — the user that launched the process.
    pub real: u32,
    /// Effective UID — used for permission checks.
    ///
    /// Differs from `real` when the process has setuid or has
    /// called `seteuid()`.
    pub effective: u32,
    /// Saved set-UID — preserved across `seteuid()` calls so the
    /// process can regain privileges.
    pub saved: u32,
    /// Filesystem UID — Linux-specific, used for file access
    /// checks. Normally equals the effective UID.
    pub filesystem: u32,
}

/// Group IDs associated with a process.
///
/// Same layout as [`Uids`], sourced from the `Gid:` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gids {
    pub real: u32,
    pub effective: u32,
    pub saved: u32,
    pub filesystem: u32,
}

/// Detailed process status from `/proc/PID/status`.
///
/// A key-value format file with one `Key:\tValue` pair per line.
/// This is the most human-readable per-process source in `/proc`,
/// but also the largest. Only a subset of fields is exposed here;
/// the full file contains many more entries (capabilities,
/// seccomp, speculation store, etc.).
#[derive(Debug)]
pub struct ProcessStatus {
    /// Process name (same as `comm` in `/proc/PID/stat`).
    pub name: Box<str>,
    /// Current process state.
    pub state: ProcessState,
    /// Thread group ID (equals PID for single-threaded processes).
    pub pid: u32,
    /// Parent process ID.
    pub ppid: u32,
    /// User IDs.
    pub uid: Uids,
    /// Group IDs.
    pub gid: Gids,
    /// Peak virtual memory size, if available.
    ///
    /// Not present on all kernels; `None` means the field was
    /// absent from the status file.
    pub vm_peak: Option<Kibibytes>,
    /// Current resident set size (physical memory in use).
    pub vm_rss: Kibibytes,
    /// Total virtual memory size.
    pub vm_size: Kibibytes,
    /// Number of threads in this process.
    pub threads: u32,
    /// Voluntary context switches (process yielded the CPU).
    pub voluntary_ctxt_switches: u64,
    /// Involuntary context switches (scheduler preempted the process).
    pub nonvoluntary_ctxt_switches: u64,
}

impl ProcessStatus {
    /// Parses `/proc/PID/status` from raw bytes.
    ///
    /// Iterates each `Key:\tValue` line and dispatches on the key.
    /// Unknown keys are silently ignored. Missing optional fields
    /// default to zero or `None`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut name = Box::from("");
        let mut state = ProcessState::Dead;
        let mut pid = 0u32;
        let mut ppid = 0u32;
        let mut uid = Uids {
            real: 0,
            effective: 0,
            saved: 0,
            filesystem: 0,
        };
        let mut gid = Gids {
            real: 0,
            effective: 0,
            saved: 0,
            filesystem: 0,
        };
        let mut vm_peak = None;
        let mut vm_rss = None;
        let mut vm_size = None;
        let mut threads = 0u32;
        let mut voluntary_ctxt_switches = 0u64;
        let mut nonvoluntary_ctxt_switches = 0u64;

        for line in bytes.split(|&b| b == b'\n') {
            if line.is_empty() {
                continue;
            }

            let (key, value) = match parse::parse_key_value_line(line) {
                Some(kv) => kv,
                None => continue,
            };

            match key {
                b"Name" => {
                    name = std::str::from_utf8(parse::trim_start(value))
                        .map_err(|_| Error::Parse {
                            path: std::path::PathBuf::from("<status>"),
                            line: 0,
                            msg: "invalid utf8 in Name",
                        })?
                        .to_string()
                        .into_boxed_str();
                }
                b"State" => {
                    let trimmed = parse::trim_start(value);
                    if let Some(&c) = trimmed.first() {
                        state = ProcessState::from_char(c as char);
                    }
                }
                b"Tgid" => {
                    pid = parse::parse_dec_u32(parse::trim_start(value)).unwrap_or(0);
                }
                b"PPid" => {
                    ppid = parse::parse_dec_u32(parse::trim_start(value)).unwrap_or(0);
                }
                b"Uid" => {
                    uid = parse_uid_line(value);
                }
                b"Gid" => {
                    gid = parse_gid_line(value);
                }
                b"VmPeak" => {
                    vm_peak = Some(parse_kib(value));
                }
                b"VmRSS" => {
                    vm_rss = Some(parse_kib(value));
                }
                b"VmSize" => {
                    vm_size = Some(parse_kib(value));
                }
                b"Threads" => {
                    threads = parse::parse_dec_u32(parse::trim_start(value)).unwrap_or(0);
                }
                b"voluntary_ctxt_switches" => {
                    voluntary_ctxt_switches =
                        parse::parse_dec_u64(parse::trim_start(value)).unwrap_or(0);
                }
                b"nonvoluntary_ctxt_switches" => {
                    nonvoluntary_ctxt_switches =
                        parse::parse_dec_u64(parse::trim_start(value)).unwrap_or(0);
                }
                _ => {}
            }
        }

        Ok(ProcessStatus {
            name,
            state,
            pid,
            ppid,
            uid,
            gid,
            vm_peak,
            vm_rss: vm_rss.unwrap_or(Kibibytes(0)),
            vm_size: vm_size.unwrap_or(Kibibytes(0)),
            threads,
            voluntary_ctxt_switches,
            nonvoluntary_ctxt_switches,
        })
    }
}

/// Parses the four space-separated UID values from a `Uid:` line.
fn parse_uid_line(value: &[u8]) -> Uids {
    let fields: Vec<&[u8]> = parse::split_spaces(parse::trim_start(value));
    let get = |i: usize| -> u32 {
        if i < fields.len() {
            parse::parse_dec_u32(fields[i]).unwrap_or(0)
        } else {
            0
        }
    };
    Uids {
        real: get(0),
        effective: get(1),
        saved: get(2),
        filesystem: get(3),
    }
}

/// Parses the four space-separated GID values from a `Gid:` line.
fn parse_gid_line(value: &[u8]) -> Gids {
    let fields: Vec<&[u8]> = parse::split_spaces(parse::trim_start(value));
    let get = |i: usize| -> u32 {
        if i < fields.len() {
            parse::parse_dec_u32(fields[i]).unwrap_or(0)
        } else {
            0
        }
    };
    Gids {
        real: get(0),
        effective: get(1),
        saved: get(2),
        filesystem: get(3),
    }
}

/// Extracts a `Kibibytes` value from a `/proc/PID/status` memory field.
///
/// The value is always followed by ` kB` (with a leading space),
/// so we take the first whitespace-delimited token and parse it
/// as a decimal integer.
fn parse_kib(value: &[u8]) -> Kibibytes {
    let trimmed = parse::trim_start(value);
    let num = trimmed.split(|&b| b == b' ').next().unwrap_or(trimmed);
    Kibibytes(parse::parse_dec_u64(num).unwrap_or(0))
}
