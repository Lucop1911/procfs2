use crate::error::{Error, Result};
use crate::util::parse;

/// Scheduling and state information for a process.
///
/// Parsed from `/proc/PID/stat`, a single line of space-separated
/// values. The `comm` field (process name) is enclosed in
/// parentheses and may itself contain spaces or closing parens,
/// so parsing must find the *last* `)` to locate the end of the
/// command name.
///
/// Time fields (`utime`, `stime`, etc.) are in jiffies. Convert
/// to seconds by dividing by the system clock tick rate
/// (`sysconf(_SC_CLK_TCK)`, typically 100).
#[derive(Debug)]
pub struct ProcessStat {
    /// Process ID.
    pub pid: u32,
    /// Filename of the executable, enclosed in parentheses.
    ///
    /// Truncated to 15 characters by the kernel. May not be
    /// unique and can be changed by the process itself.
    pub comm: Box<str>,
    /// Single-character process state code.
    ///
    /// `R` = running, `S` = sleeping, `D` = disk sleep,
    /// `Z` = zombie, `T` = stopped, `t` = tracing stop,
    /// `X` = dead, `I` = idle.
    pub state: char,
    /// Parent process ID.
    pub ppid: u32,
    /// Process group ID.
    pub pgrp: u32,
    /// Session ID.
    pub session: u32,
    /// Controlling terminal device number (major/minor packed).
    pub tty_nr: i32,
    /// Foreground process group ID of the controlling terminal.
    ///
    /// `-1` if the process has no controlling terminal.
    pub tpgid: u32,
    /// Kernel flags word (e.g. `PF_KTHREAD` for kernel threads).
    pub flags: u32,
    /// Minor page faults (no disk I/O required).
    pub minflt: u64,
    /// Minor page faults by waited-for children.
    pub cminflt: u64,
    /// Major page faults (required disk I/O).
    pub majflt: u64,
    /// Major page faults by waited-for children.
    pub cmajflt: u64,
    /// User-mode CPU time in jiffies.
    pub utime: u64,
    /// Kernel-mode CPU time in jiffies.
    pub stime: u64,
    /// User-mode CPU time of waited-for children.
    pub cutime: i64,
    /// Kernel-mode CPU time of waited-for children.
    pub cstime: i64,
    /// Real-time scheduling priority (higher = more priority).
    pub priority: i64,
    /// Nice value (higher = less priority).
    pub nice: i64,
    /// Number of threads in this process.
    pub num_threads: i64,
    /// Real-time timer signal delivery time (always 0 since 2.6.17).
    pub itrealvalue: i64,
    /// Time the process started, measured in jiffies since boot.
    pub starttime: u64,
    /// Virtual memory size in bytes.
    pub vsize: u64,
    /// Resident set size in pages.
    pub rss: i64,
}

impl ProcessStat {
    /// Parses a `/proc/PID/stat` line from raw bytes.
    ///
    /// The parsing strategy:
    /// 1. Find the last `)` to delimit the `comm` field (which may
    ///    contain spaces or parens).
    /// 2. Everything before the first space after the opening `(`
    ///    is the PID.
    /// 3. Everything after the last `)` is space-separated numeric
    ///    fields starting at index 3 (state).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let last_paren = bytes
            .iter()
            .rposition(|&b| b == b')')
            .ok_or_else(|| Error::Parse {
                path: std::path::PathBuf::from("<stat>"),
                line: 1,
                msg: "missing closing paren",
            })?;

        let space_after_pid =
            bytes
                .iter()
                .position(|&b| b == b' ')
                .ok_or_else(|| Error::Parse {
                    path: std::path::PathBuf::from("<stat>"),
                    line: 1,
                    msg: "missing space after pid",
                })?;

        let comm_start = space_after_pid + 2;
        let comm = &bytes[comm_start..last_paren];
        let comm_str = std::str::from_utf8(comm).map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<stat>"),
            line: 1,
            msg: "invalid utf8 in comm",
        })?;

        let rest = &bytes[last_paren + 2..];
        let fields: Vec<&[u8]> = rest
            .split(|&b| b == b' ')
            .filter(|f| !f.is_empty())
            .collect();

        if fields.len() < 22 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<stat>"),
                line: 1,
                msg: "not enough fields in stat",
            });
        }

        Ok(ProcessStat {
            pid: std::str::from_utf8(&bytes[..space_after_pid])
                .map_err(|_| Error::Parse {
                    path: std::path::PathBuf::from("<stat>"),
                    line: 1,
                    msg: "invalid pid",
                })?
                .parse()
                .map_err(|_| Error::Parse {
                    path: std::path::PathBuf::from("<stat>"),
                    line: 1,
                    msg: "invalid pid",
                })?,
            comm: comm_str.into(),
            state: *fields[0].first().ok_or_else(|| Error::Parse {
                path: std::path::PathBuf::from("<stat>"),
                line: 1,
                msg: "invalid state",
            })? as char,
            ppid: parse::parse_dec_u32(fields[1])?,
            pgrp: parse::parse_dec_u32(fields[2])?,
            session: parse::parse_dec_u32(fields[3])?,
            tty_nr: parse::parse_dec_i64(fields[4])? as i32,
            tpgid: parse::parse_dec_i64(fields[5])? as u32,
            flags: parse::parse_dec_u32(fields[6])?,
            minflt: parse::parse_dec_u64(fields[7])?,
            cminflt: parse::parse_dec_u64(fields[8])?,
            majflt: parse::parse_dec_u64(fields[9])?,
            cmajflt: parse::parse_dec_u64(fields[10])?,
            utime: parse::parse_dec_u64(fields[11])?,
            stime: parse::parse_dec_u64(fields[12])?,
            cutime: parse::parse_dec_i64(fields[13])?,
            cstime: parse::parse_dec_i64(fields[14])?,
            priority: parse::parse_dec_i64(fields[15])?,
            nice: parse::parse_dec_i64(fields[16])?,
            num_threads: parse::parse_dec_i64(fields[17])?,
            itrealvalue: parse::parse_dec_i64(fields[18])?,
            starttime: parse::parse_dec_u64(fields[19])?,
            vsize: parse::parse_dec_u64(fields[20])?,
            rss: parse::parse_dec_i64(fields[21])?,
        })
    }
}
