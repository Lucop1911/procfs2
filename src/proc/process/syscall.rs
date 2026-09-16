use crate::error::{Error, Result};
use crate::util::parse::{SplitFields, parse_dec_i64, parse_hex_u64};

/// Current system call state of a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallState {
    /// The process is not blocked in a system call.
    Running,
    /// The process is blocked but not in a system call (e.g., waiting on I/O).
    BlockedNotInSyscall,
    /// The process is currently executing a system call.
    InSyscall,
}

/// Information about the system call currently being executed by a process.
///
/// Parsed from `/proc/PID/syscall`. The file exposes the system call number
/// and argument registers, followed by the stack pointer and instruction pointer.
///
/// If the process is not blocked, the file contains just "running".
/// If blocked but not in a syscall, it shows -1 followed by SP and IP.
/// If in a syscall, it shows the syscall number, 6 arguments, SP, and IP.
#[derive(Debug)]
pub struct ProcessSyscall {
    /// The system call state.
    pub state: SyscallState,
    /// System call number (valid when `state == InSyscall`).
    pub nr: Option<i64>,
    /// System call arguments (valid when `state == InSyscall`).
    ///
    /// Up to 6 arguments are exposed, always stored as 64-bit values
    /// regardless of architecture.
    pub args: [Option<u64>; 6],
    /// Stack pointer register value.
    pub sp: u64,
    /// Instruction pointer (program counter) register value.
    pub ip: u64,
}

impl ProcessSyscall {
    /// Parses a `/proc/PID/syscall` line from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let trimmed = crate::util::parse::trim(bytes);

        if trimmed == b"running" {
            return Ok(ProcessSyscall {
                state: SyscallState::Running,
                nr: None,
                args: [None; 6],
                sp: 0,
                ip: 0,
            });
        }

        let fields = SplitFields::<10>::new(trimmed);

        if fields.len() < 3 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<syscall>"),
                line: 1,
                msg: "too few fields in syscall entry",
            });
        }

        let nr = parse_dec_i64(fields[0])?;

        if nr < 0 {
            if fields.len() != 3 {
                return Err(Error::Parse {
                    path: std::path::PathBuf::from("<syscall>"),
                    line: 1,
                    msg: "blocked not in syscall expects 3 fields",
                });
            }
            let sp = parse_hex_u64(fields[1])?;
            let ip = parse_hex_u64(fields[2])?;
            return Ok(ProcessSyscall {
                state: SyscallState::BlockedNotInSyscall,
                nr: Some(nr),
                args: [None; 6],
                sp,
                ip,
            });
        }

        if fields.len() != 9 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<syscall>"),
                line: 1,
                msg: "in-syscall entry expects 9 fields",
            });
        }

        let mut args: [Option<u64>; 6] = [None, None, None, None, None, None];
        for i in 0..6 {
            args[i] = Some(parse_hex_u64(fields[1 + i])?);
        }
        let sp = parse_hex_u64(fields[7])?;
        let ip = parse_hex_u64(fields[8])?;

        Ok(ProcessSyscall {
            state: SyscallState::InSyscall,
            nr: Some(nr),
            args,
            sp,
            ip,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_running() -> Result<()> {
        let result = ProcessSyscall::from_bytes(b"running\n")?;
        assert_eq!(result.state, SyscallState::Running);
        assert_eq!(result.nr, None);
        Ok(())
    }

    #[test]
    fn test_parse_blocked_not_in_syscall() -> Result<()> {
        let result = ProcessSyscall::from_bytes(b"-1 0x7fff1234 0x400000\n")?;
        assert_eq!(result.state, SyscallState::BlockedNotInSyscall);
        assert_eq!(result.nr, Some(-1));
        assert_eq!(result.sp, 0x7fff1234);
        assert_eq!(result.ip, 0x400000);
        Ok(())
    }

    #[test]
    fn test_parse_in_syscall() -> Result<()> {
        let data = b"278 0x1 0x7fff1234 0x100 0x0 0x0 0x0 0x7fff1234 0x400000\n";
        let result = ProcessSyscall::from_bytes(data)?;
        assert_eq!(result.state, SyscallState::InSyscall);
        assert_eq!(result.nr, Some(278));
        assert_eq!(result.args[0], Some(0x1));
        assert_eq!(result.args[1], Some(0x7fff1234));
        assert_eq!(result.args[2], Some(0x100));
        assert_eq!(result.args[3], Some(0x0));
        assert_eq!(result.args[4], Some(0x0));
        assert_eq!(result.args[5], Some(0x0));
        assert_eq!(result.sp, 0x7fff1234);
        assert_eq!(result.ip, 0x400000);
        Ok(())
    }
}
