use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// The type of a file lock from `/proc/locks`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// A BSD file lock created with `flock(2)`.
    Flock,
    /// An open file description (OFD) lock created with `fcntl(2)`.
    Ofdlck,
    /// A POSIX byte-range lock created with `fcntl(2)`.
    Posix,
}

/// Whether a lock is advisory or mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockClass {
    /// An advisory lock (cooperative, requires the holder to check).
    Advisory,
    /// A mandatory lock (enforced by the kernel).
    Mandatory,
}

/// The access mode of a file lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockAccess {
    /// A shared / read lock (POSIX or OFD) or a BSD shared lock.
    Read,
    /// An exclusive / write lock (POSIX or OFD) or a BSD exclusive lock.
    Write,
}

/// A single file lock or lease from `/proc/locks`.
///
/// Each line represents one active `flock(2)`, `fcntl(2)`, or
/// lease lock held by a process. OFD locks show a PID of `-1`
/// because they are not owned by a single process.
#[derive(Debug)]
pub struct Lock {
    /// Lock type: BSD (`flock`), POSIX, or OFD.
    pub lock_type: LockType,
    /// Advisory or mandatory.
    pub class: LockClass,
    /// Read (shared) or write (exclusive).
    pub access: LockAccess,
    /// PID of the lock owner, or `-1` for OFD locks.
    pub pid: i32,
    /// Major device number of the filesystem.
    pub major: u32,
    /// Minor device number of the filesystem.
    pub minor: u32,
    /// Inode number of the locked file.
    pub inode: u64,
    /// Byte offset of the first byte of the lock.
    pub start: u64,
    /// Byte offset of the last byte of the lock, or `None` if the
    /// lock extends to end of file (`EOF`).
    pub end: Option<u64>,
}

/// Reads `/proc/locks` and returns all active file locks and leases.
///
/// Each line starts with an ordinal (`N:`) followed by seven
/// whitespace-separated fields: type, class, access, pid,
/// `major:minor:inode`, start, and end (or `EOF`).
pub fn locks() -> Result<Vec<Lock>> {
    let path = Path::new("/proc/locks");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<Lock> = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<8>::new(line);

        if fields.len() < 8 {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "expected 'N: TYPE CLASS ACCESS PID DEV:INO START END'",
            });
        }

        // skipping fields[0] as its the ordinal (e.g. "1:").

        let lock_type = match fields[1] {
            b"FLOCK" => LockType::Flock,
            b"OFDLCK" => LockType::Ofdlck,
            b"POSIX" => LockType::Posix,
            _ => {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid lock type",
                });
            }
        };

        let class = match fields[2] {
            b"ADVISORY" => LockClass::Advisory,
            b"MANDATORY" => LockClass::Mandatory,
            _ => {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid class",
                });
            }
        };

        let access = match fields[3] {
            b"READ" => LockAccess::Read,
            b"WRITE" => LockAccess::Write,
            _ => {
                return Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid access",
                });
            }
        };

        let pid = parse::parse_dec_i64(fields[4]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid pid",
        })? as i32;

        let (major_b, rest) = parse::split_at_byte(fields[5], b':');
        let (minor_b, inode_b) = parse::split_at_byte(rest, b':');

        let major = parse::parse_dec_u32(major_b).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid major",
        })?;

        let minor = parse::parse_dec_u32(minor_b).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid minor",
        })?;

        let inode = parse::parse_dec_u64(inode_b).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid inode",
        })?;

        let start = parse::parse_dec_u64(fields[6]).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "invalid start",
        })?;

        let end = match fields[7] {
            b"EOF" => None,
            _ => Some(parse::parse_dec_u64(fields[7]).map_err(|_| Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "invalid end",
            })?),
        };

        out.push(Lock {
            lock_type,
            class,
            access,
            pid,
            major,
            minor,
            inode,
            start,
            end,
        });
    }

    Ok(out)
}
