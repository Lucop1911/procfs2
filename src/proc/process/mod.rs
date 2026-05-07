mod cgroup;
mod fd;
mod io;
mod limits;
mod maps;
mod mountinfo;
mod ns;
pub mod stat;
mod status;
mod threads;

pub use cgroup::CgroupEntry;
pub use fd::{Fd, FdTarget};
pub use io::ProcessIo;
pub use limits::{Limit, LimitUnit, ProcessLimits};
pub use maps::{MapPathname, MapPermissions, MemoryMap, MemoryMapDetail};
pub use mountinfo::MountInfo;
pub use ns::Namespaces;
pub use stat::ProcessStat;
pub use status::{Gids, ProcessState, ProcessStatus, Uids};

use crate::error::{Error, Result};
use std::os::unix::ffi::OsStrExt;

/// A handle to a running process, identified by its PID.
///
/// This struct is the entry point for all per-process information
/// available under `/proc/PID/`. It does not hold any open file
/// descriptors — each method performs a fresh read.
///
/// # Process lifetime
///
/// A `Process` may become stale between creation and use if the
/// target process exits. Methods return [`Error::ProcessGone`] in
/// that case.
#[derive(Debug)]
pub struct Process {
    pub pid: u32,
}

impl Process {
    /// Creates a `Process` handle for the given PID.
    ///
    /// Verifies that `/proc/<pid>` exists as a directory. Returns
    /// [`Error::ProcessGone`] if the directory is absent, which
    /// means either the PID is invalid or the process has already
    /// terminated.
    pub fn new(pid: u32) -> Result<Self> {
        let path = format!("/proc/{}", pid);
        if std::path::Path::new(&path).is_dir() {
            Ok(Process { pid })
        } else {
            Err(Error::ProcessGone(pid))
        }
    }

    /// Returns a `Process` handle for the calling process.
    ///
    /// Resolves `/proc/self` to determine the caller's PID. This is
    /// preferred over `Process::new(std::process::id())` because it
    /// works correctly inside containers and PID namespaces where
    /// the kernel-visible PID may differ from the host PID.
    pub fn current() -> Result<Self> {
        let path = "/proc/self";
        let target = std::fs::read_link(path).map_err(Error::Io)?;
        let pid_str = target
            .file_name()
            .ok_or_else(|| Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "invalid /proc/self symlink",
            })?
            .to_str()
            .ok_or_else(|| Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "non-utf8 pid in /proc/self",
            })?;

        let pid = pid_str.parse::<u32>().map_err(|_| Error::Parse {
            path: std::path::PathBuf::from(path),
            line: 0,
            msg: "invalid pid in /proc/self",
        })?;

        Ok(Process { pid })
    }

    /// Iterates over all currently visible processes.
    ///
    /// Scans `/proc` for numeric directory entries. The snapshot is
    /// taken at call time; processes spawned or terminated after the
    /// call are not reflected. Returns an error for any directory
    /// that cannot be read, but continues iterating over the rest.
    pub fn all() -> impl Iterator<Item = Result<Self>> {
        let entries = match std::fs::read_dir("/proc") {
            Ok(iter) => iter,
            Err(e) => {
                return vec![Err(Error::Io(e))].into_iter();
            }
        };

        entries
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.chars().all(|c| c.is_ascii_digit()) {
                        let pid = name_str.parse::<u32>().ok()?;
                        Some(Ok(Process { pid }))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(Error::Io(e))),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Reads `/proc/PID/stat` and returns [`ProcessStat`].
    ///
    /// Contains scheduling information, memory usage, and timing
    /// data. The `comm` field is limited to 15 characters by the
    /// kernel and may be truncated.
    pub fn stat(&self) -> Result<ProcessStat> {
        let path = format!("/proc/{}/stat", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        ProcessStat::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/status` and returns [`ProcessStatus`].
    ///
    /// A human-readable key-value format with more detail than
    /// `/proc/PID/stat`, including UIDs, GIDs, voluntary context
    /// switches, and optional memory peaks.
    pub fn status(&self) -> Result<ProcessStatus> {
        let path = format!("/proc/{}/status", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        ProcessStatus::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/cmdline` and returns the argument vector.
    ///
    /// Arguments are null-delimited in the kernel file. Returns an
    /// empty vector for kernel threads, which have no cmdline.
    pub fn cmdline(&self) -> Result<Vec<std::ffi::OsString>> {
        let path = format!("/proc/{}/cmdline", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;

        if bytes.is_empty() {
            return Ok(Vec::new());
        }

        let args = bytes
            .split(|&b| b == b'\0')
            .filter(|s| !s.is_empty())
            .map(|s| std::ffi::OsStr::from_bytes(s).to_os_string())
            .collect();

        Ok(args)
    }

    /// Reads `/proc/PID/environ` and returns the environment map.
    ///
    /// Entries are null-delimited `KEY=VALUE` pairs. Returns an
    /// empty map for kernel threads. Reading another user's
    /// environment typically yields [`Error::PermissionDenied`].
    pub fn environ(
        &self,
    ) -> Result<std::collections::HashMap<std::ffi::OsString, std::ffi::OsString>> {
        let path = format!("/proc/{}/environ", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;

        let mut map = std::collections::HashMap::new();

        for entry in bytes.split(|&b| b == b'\0').filter(|s| !s.is_empty()) {
            if let Some(eq_pos) = entry.iter().position(|&b| b == b'=') {
                let key = std::ffi::OsStr::from_bytes(&entry[..eq_pos]).to_os_string();
                let value = std::ffi::OsStr::from_bytes(&entry[eq_pos + 1..]).to_os_string();
                map.insert(key, value);
            }
        }

        Ok(map)
    }

    /// Reads the `/proc/PID/exe` symlink to get the executable path.
    ///
    /// Returns [`Error::PermissionDenied`] if the process is owned
    /// by another user and the caller lacks `CAP_SYS_PTRACE`.
    pub fn exe(&self) -> Result<std::path::PathBuf> {
        let path = format!("/proc/{}/exe", self.pid);
        std::fs::read_link(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(std::path::PathBuf::from(&path))
            } else {
                Error::Io(e)
            }
        })
    }

    /// Reads the `/proc/PID/cwd` symlink to get the working directory.
    ///
    /// Returns [`Error::PermissionDenied`] under the same conditions
    /// as [`Process::exe`].
    pub fn cwd(&self) -> Result<std::path::PathBuf> {
        let path = format!("/proc/{}/cwd", self.pid);
        std::fs::read_link(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(std::path::PathBuf::from(&path))
            } else {
                Error::Io(e)
            }
        })
    }

    /// Reads `/proc/PID/maps` and returns the memory map entries.
    ///
    /// Each entry describes a virtual memory region: address range,
    /// permissions (read/write/exec/shared/private), offset, device,
    /// inode, and optional backing pathname.
    pub fn maps(&self) -> Result<Vec<MemoryMap>> {
        let path = format!("/proc/{}/maps", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        MemoryMap::parse_all(&bytes)
    }

    /// Reads `/proc/PID/smaps` and returns detailed memory stats.
    ///
    /// Extends `/proc/PID/maps` with per-region RSS, PSS, shared/
    /// private clean/dirty, referenced, anonymous, and swap counts.
    /// This is significantly larger than `maps` and slower to parse.
    pub fn smaps(&self) -> Result<Vec<MemoryMapDetail>> {
        let path = format!("/proc/{}/smaps", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        MemoryMapDetail::parse_all(&bytes)
    }

    /// Reads `/proc/PID/fd/` and returns open file descriptors.
    ///
    /// Iterates the directory, reads each symlink, and classifies
    /// the target as a file, socket, pipe, anon-inode, or other.
    pub fn fds(&self) -> Result<Vec<Fd>> {
        let path = format!("/proc/{}/fd", self.pid);
        let entries = std::fs::read_dir(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(std::path::PathBuf::from(&path))
            } else {
                Error::Io(e)
            }
        })?;

        let mut fds = Vec::new();

        for entry in entries {
            let entry = entry.map_err(Error::Io)?;
            let fd_num = entry.file_name().to_string_lossy().parse::<i32>().ok();
            if let Some(num) = fd_num {
                let target = std::fs::read_link(entry.path()).map_err(Error::Io)?;
                let target_str = target.to_string_lossy();
                let fd_target = FdTarget::parse(&target_str);
                fds.push(Fd {
                    number: num,
                    target: fd_target,
                });
            }
        }

        Ok(fds)
    }

    /// Reads `/proc/PID/io` and returns I/O counters.
    ///
    /// Reports bytes read/written at the syscall level (`rchar`/
    /// `wchar`) and at the storage layer (`read_bytes`/
    /// `write_bytes`). The difference between the two pairs reveals
    /// page cache activity.
    pub fn io(&self) -> Result<ProcessIo> {
        let path = format!("/proc/{}/io", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        ProcessIo::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/limits` and returns resource limits.
    ///
    /// Parses the table-format file into typed [`Limit`] structs
    /// with soft/hard values and units. `unlimited` is represented
    /// as `None`.
    pub fn limits(&self) -> Result<ProcessLimits> {
        let path = format!("/proc/{}/limits", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        ProcessLimits::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/mountinfo` and returns mount entries.
    ///
    /// A richer format than `/proc/mounts` with mount IDs, parent
    /// IDs, optional fields, and separate superblock options.
    pub fn mountinfo(&self) -> Result<Vec<MountInfo>> {
        let path = format!("/proc/{}/mountinfo", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        MountInfo::parse_all(&bytes)
    }

    /// Reads `/proc/PID/cgroup` and returns cgroup memberships.
    ///
    /// Each entry lists the hierarchy ID, controller list, and
    /// cgroup path. Empty controller lists indicate the process is
    /// in the root cgroup for that hierarchy.
    pub fn cgroup(&self) -> Result<Vec<CgroupEntry>> {
        let path = format!("/proc/{}/cgroup", self.pid);
        let bytes = crate::util::parse::read_file(std::path::Path::new(&path))?;
        CgroupEntry::parse_all(&bytes)
    }

    /// Reads `/proc/PID/ns/` and returns namespace inode numbers.
    ///
    /// Each namespace symlink (`mnt`, `pid`, `net`, etc.) is read
    /// and its target inode is extracted. Missing namespaces (e.g.
    /// `time` on older kernels) are returned as `None`.
    pub fn namespaces(&self) -> Result<Namespaces> {
        let path = format!("/proc/{}/ns", self.pid);
        Namespaces::from_dir(&path)
    }

    /// Iterates over the threads of this process.
    ///
    /// Scans `/proc/PID/task/` for numeric entries. Each thread is
    /// returned as a `Process` with its TID as the `pid` field,
    /// allowing all `Process` methods to be called on individual
    /// threads.
    pub fn threads(&self) -> impl Iterator<Item = Result<Self>> {
        let task_path = format!("/proc/{}/task", self.pid);
        let entries = match std::fs::read_dir(&task_path) {
            Ok(iter) => iter,
            Err(e) => {
                return vec![Err(Error::Io(e))].into_iter();
            }
        };

        entries
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.chars().all(|c| c.is_ascii_digit()) {
                        let tid = name_str.parse::<u32>().ok()?;
                        Some(Ok(Process { pid: tid }))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(Error::Io(e))),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}
