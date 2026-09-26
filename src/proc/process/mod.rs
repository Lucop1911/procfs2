mod auxv;
mod cgroup;
mod coredump_filter;
mod fd;
mod io;
mod limits;
mod maps;
mod mountinfo;
mod ns;
mod pagemap;
mod stat;
mod statm;
mod status;
mod syscall;
mod threads;

pub use auxv::{Auxv, AuxvEntry, auxv_type};
pub use cgroup::CgroupEntry;
pub use coredump_filter::CoreDumpFilter;
pub use fd::{Fd, FdTarget};
pub use io::ProcessIo;
pub use limits::{Limit, LimitUnit, ProcessLimits};
pub use maps::{MapPathname, MapPermissions, MemoryMap, MemoryMapDetail, SmapsRollup};
pub use mountinfo::MountInfo;
pub use ns::Namespaces;
pub use pagemap::PageMapEntry;
pub use stat::ProcessStat;
pub use statm::Statm;
pub use status::{Gids, ProcessState, ProcessStatus, Uids};
pub use syscall::{ProcessSyscall, SyscallState};

use crate::{
    error::{Error, KernelVersion, Result},
    util::parse,
};
use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

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
    /// Process ID.
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
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, pid, "");

        if Path::new(path).is_dir() {
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
        let target = std::fs::read_link(path).map_err(|e| Error::Io {
            path: Some(PathBuf::from(path)),
            error: e,
        })?;
        let pid_str = target
            .file_name()
            .ok_or_else(|| Error::Parse {
                path: PathBuf::from(path),
                line: 0,
                msg: "invalid /proc/self symlink",
            })?
            .to_str()
            .ok_or_else(|| Error::Parse {
                path: PathBuf::from(path),
                line: 0,
                msg: "non-utf8 pid in /proc/self",
            })?;

        let pid = pid_str.parse::<u32>().map_err(|_| Error::Parse {
            path: PathBuf::from(path),
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
                return vec![Err(Error::Io {
                    path: Some(PathBuf::from("/proc")),
                    error: e,
                })]
                .into_iter();
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
                Err(e) => Some(Err(Error::Io {
                    path: Some(PathBuf::from("/proc")),
                    error: e,
                })),
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
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/stat");
        let bytes = parse::read_file(Path::new(&path))?;
        ProcessStat::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/status` and returns [`ProcessStatus`].
    ///
    /// A human-readable key-value format with more detail than
    /// `/proc/PID/stat`, including UIDs, GIDs, voluntary context
    /// switches, and optional memory peaks.
    pub fn status(&self) -> Result<ProcessStatus> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/status");
        let bytes = parse::read_file(Path::new(&path))?;
        ProcessStatus::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/cmdline` and returns the argument vector.
    ///
    /// Arguments are null-delimited in the kernel file. Returns an
    /// empty vector for kernel threads, which have no cmdline.
    pub fn cmdline(&self) -> Result<Vec<std::ffi::OsString>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/cmdline");
        let bytes = parse::read_file(Path::new(&path))?;

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
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/environ");
        let bytes = parse::read_file(Path::new(&path))?;

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
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/exe");
        std::fs::read_link(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(PathBuf::from(&path))
            } else {
                Error::Io {
                    path: Some(PathBuf::from(&path)),
                    error: e,
                }
            }
        })
    }

    /// Reads the `/proc/PID/cwd` symlink to get the working directory.
    ///
    /// Returns [`Error::PermissionDenied`] under the same conditions
    /// as [`Process::exe`].
    pub fn cwd(&self) -> Result<std::path::PathBuf> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/cwd");
        std::fs::read_link(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(PathBuf::from(&path))
            } else {
                Error::Io {
                    path: Some(PathBuf::from(&path)),
                    error: e,
                }
            }
        })
    }

    /// Reads `/proc/PID/maps` and returns the memory map entries.
    ///
    /// Each entry describes a virtual memory region: address range,
    /// permissions (read/write/exec/shared/private), offset, device,
    /// inode, and optional backing pathname.
    pub fn maps(&self) -> Result<Vec<MemoryMap>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/maps");
        let bytes = parse::read_file(Path::new(&path))?;
        MemoryMap::parse_all(&bytes)
    }

    /// Reads `/proc/PID/smaps` and returns detailed memory stats.
    ///
    /// Extends `/proc/PID/maps` with per-region RSS, PSS, shared/
    /// private clean/dirty, referenced, anonymous, and swap counts.
    /// This is significantly larger than `maps` and slower to parse.
    pub fn smaps(&self) -> Result<Vec<MemoryMapDetail>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/smaps");
        let bytes = parse::read_file(Path::new(&path))?;
        MemoryMapDetail::parse_all(&bytes)
    }

    /// Reads `/proc/PID/smaps_rollup` and returns a [`SmapsRollup`].
    ///
    /// A single aggregate memory summary (RSS, PSS, shared/private,
    /// swap, ...). Much smaller and faster to parse than the full
    /// `/proc/PID/smaps`, at the cost of losing per-region details.
    ///
    /// Requires kernel 4.14 or newer; on older kernels this returns
    /// [`Error::UnsupportedKernel`].
    pub fn smaps_rollup(&self) -> Result<SmapsRollup> {
        KernelVersion::current()?.require(4, 14)?;
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/smaps_rollup");
        let bytes = parse::read_file(Path::new(&path))?;
        SmapsRollup::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/fd/` and returns open file descriptors.
    ///
    /// Iterates the directory, reads each symlink, and classifies
    /// the target as a file, socket, pipe, anon-inode, or other.
    pub fn fds(&self) -> Result<Vec<Fd>> {
        fd::read_fds(self.pid)
    }

    /// Reads `/proc/PID/io` and returns I/O counters.
    ///
    /// Reports bytes read/written at the syscall level (`rchar`/
    /// `wchar`) and at the storage layer (`read_bytes`/
    /// `write_bytes`). The difference between the two pairs reveals
    /// page cache activity.
    pub fn io(&self) -> Result<ProcessIo> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/io");
        let bytes = parse::read_file(Path::new(&path))?;
        ProcessIo::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/limits` and returns resource limits.
    ///
    /// Parses the table-format file into typed [`Limit`] structs
    /// with soft/hard values and units. `unlimited` is represented
    /// as `None`.
    pub fn limits(&self) -> Result<ProcessLimits> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/limits");
        let bytes = parse::read_file(Path::new(&path))?;
        ProcessLimits::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/syscall` and returns the current system call.
    ///
    /// Exposes the system call number and argument registers for the
    /// system call currently being executed by the process, followed
    /// by the stack pointer and instruction pointer.
    ///
    /// Returns [`SyscallState::Running`] if the process is not blocked.
    /// Returns [`SyscallState::BlockedNotInSyscall`] if blocked but
    /// not in a syscall (syscall number -1). Returns
    /// [`SyscallState::InSyscall`] with the syscall number and
    /// arguments if currently in a syscall.
    ///
    /// Requires kernel 2.6.27+ with `CONFIG_HAVE_ARCH_TRACEHOOK`.
    /// Returns [`Error::PermissionDenied`] if the caller lacks
    /// `PTRACE_MODE_ATTACH_FSCREDS` permission.
    pub fn syscall(&self) -> Result<ProcessSyscall> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/syscall");
        let bytes = parse::read_file(Path::new(&path))?;
        ProcessSyscall::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/mountinfo` and returns mount entries.
    ///
    /// A richer format than `/proc/mounts` with mount IDs, parent
    /// IDs, optional fields, and separate superblock options.
    pub fn mountinfo(&self) -> Result<Vec<MountInfo>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/mountinfo");
        let bytes = parse::read_file(Path::new(&path))?;
        MountInfo::parse_all(&bytes)
    }

    /// Reads `/proc/PID/cgroup` and returns cgroup memberships.
    ///
    /// Each entry lists the hierarchy ID, controller list, and
    /// cgroup path. Empty controller lists indicate the process is
    /// in the root cgroup for that hierarchy.
    pub fn cgroup(&self) -> Result<Vec<CgroupEntry>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/cgroup");
        let bytes = parse::read_file(Path::new(&path))?;
        CgroupEntry::parse_all(&bytes)
    }

    /// Reads `/proc/PID/ns/` and returns namespace inode numbers.
    ///
    /// Each namespace symlink (`mnt`, `pid`, `net`, etc.) is read
    /// and its target inode is extracted. Missing namespaces (e.g.
    /// `time` on older kernels) are returned as `None`.
    pub fn namespaces(&self) -> Result<Namespaces> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/ns");
        Namespaces::from_dir(path)
    }

    /// Iterates over the threads of this process.
    ///
    /// Scans `/proc/PID/task/` for numeric entries. Each thread is
    /// returned as a `Process` with its TID as the `pid` field,
    /// allowing all `Process` methods to be called on individual
    /// threads.
    pub fn threads(&self) -> impl Iterator<Item = Result<Self>> {
        threads::read_threads(self.pid)
    }

    /// Reads `/proc/PID/auxv` and returns the auxiliary vector.
    ///
    /// The auxiliary vector carries startup information the kernel
    /// passes to the dynamic linker, including the page size, ELF
    /// header pointers, and random bytes.
    pub fn auxv(&self) -> Result<Auxv> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/auxv");
        let bytes = parse::read_file(Path::new(&path))?;
        Auxv::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/pagemap` for a virtual address range.
    ///
    /// Returns one [`PageMapEntry`] per page intersecting `start..end`
    /// (start inclusive, end exclusive). The pagemap covers the whole
    /// address space, including unmapped holes, so it is read a range
    /// at a time; use [`Process::maps`] to find the regions worth
    /// querying.
    ///
    /// The page size comes from the process's auxiliary vector
    /// (`AT_PAGESZ`), so this also reads `/proc/PID/auxv`.
    ///
    /// Reading another process's pagemap requires `CAP_SYS_PTRACE`.
    /// Without it the page frame numbers read back as zero, though
    /// the present and soft-dirty bits remain visible.
    ///
    /// Ranges that extend past the end of the address space (such as
    /// the `[vsyscall]` mapping on x86_64) yield only the entries the
    /// kernel can report.
    pub fn pagemap(&self, start: u64, end: u64) -> Result<Vec<PageMapEntry>> {
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        let path = proc_path(&mut buf1, self.pid, "/pagemap");
        let auxv_path = proc_path(&mut buf2, self.pid, "/auxv");
        pagemap::read(path, auxv_path, start, end)
    }

    /// Reads `/proc/PID/statm` and returns the memory summary.
    ///
    /// Seven page counts covering total size, resident set, shared
    /// (file-backed), text, libraries, data, and dirty pages. All
    /// values are in pages, not bytes.
    pub fn statm(&self) -> Result<Statm> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/statm");
        let bytes = parse::read_file(Path::new(&path))?;
        Statm::from_bytes(&bytes)
    }

    /// Reads `/proc/PID/loginuid` and returns the login uid.
    ///
    /// The login uid is set by `pam_loginuid` at login and identifies
    /// the user a process was logged in as, surviving `setuid` and
    /// namespace transitions. It is `u32::MAX` when no login uid has
    /// been assigned (e.g. system services started before login).
    ///
    /// The `/proc/PID/loginuid` file only exists when auditing is enabled
    /// in the kernel (`CONFIG_AUDITSYSCALL`); kernels built without it
    /// yield `None` rather than an error.
    pub fn loginuid(&self) -> Result<Option<u32>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/loginuid");
        let bytes = match parse::read_file(Path::new(&path)) {
            Ok(bytes) => bytes,
            Err(Error::Io { error, .. }) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };

        Ok(Some(parse::parse_dec_u32(&bytes)?))
    }

    /// Reads `/proc/PID/sessionid` and returns the audit session id.
    ///
    /// The audit session id identifies the login session a process
    /// belongs to. It is inherited by children and survives `setuid`
    /// and namespace transitions, tying together all processes started
    /// from one login. Kernel threads and processes started before any
    /// login report `u32::MAX`.
    ///
    /// The `/proc/PID/sessionid` file only exists when auditing is enabled
    /// in the kernel (`CONFIG_AUDITSYSCALL`); kernels built without it
    /// yield `None` rather than an error.
    pub fn sessionid(&self) -> Result<Option<u32>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/sessionid");
        let bytes = match parse::read_file(Path::new(&path)) {
            Ok(bytes) => bytes,
            Err(Error::Io { error, .. }) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };

        Ok(Some(parse::parse_dec_u32(&bytes)?))
    }

    /// Reads `/proc/PID/coredump_filter` and returns the core dump filter.
    ///
    /// A bitmask of the memory mapping types included when the process
    /// dumps core. The value is inherited by `fork` and preserved across
    /// `execve`.
    ///
    /// The `/proc/PID/coredump_filter` file only exists when the kernel is
    /// built with `CONFIG_ELF_CORE`; kernels built without it yield
    /// `None` rather than an error. Kernel threads have no address space
    /// and report an empty filter.
    pub fn coredump_filter(&self) -> Result<Option<CoreDumpFilter>> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/coredump_filter");
        let bytes = match parse::read_file(Path::new(path)) {
            Ok(bytes) => bytes,
            Err(Error::Io { error, .. }) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(e) => return Err(e),
        };

        Ok(Some(CoreDumpFilter::from_bytes(&bytes)?))
    }

    /// Reads `/proc/PID/oom_score` and returns the OOM score.
    ///
    /// The kernel's estimate of how likely this process is to be picked
    /// as the next OOM victim when memory runs out. Higher is worse:
    /// the score grows with resident memory and is adjusted by the
    /// `oom_score_adj` value, which can push it into the low thousands.
    /// It fits comfortably in a `u16`.
    pub fn oom_score(&self) -> Result<u16> {
        let mut buf = [0u8; 32];
        let path = proc_path(&mut buf, self.pid, "/oom_score");
        let bytes = parse::read_file(Path::new(path))?;

        parse::parse_dec_u32(&bytes).map(|v| v as u16)
    }
}

fn write_u32(mut n: u32, buf: &mut [u8]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 10];
    let mut i = 10;
    while n > 0 {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    let len = 10 - i;
    buf[..len].copy_from_slice(&tmp[i..]);
    len
}

/// Writes "/proc/{pid}{suffix}" into `buf` and returns it as a `&str`.
/// `suffix` should include its own leading slash, e.g. "/stat", or be "" for the bare pid dir.
pub(crate) fn proc_path<'a>(buf: &'a mut [u8; 32], pid: u32, suffix: &str) -> &'a OsStr {
    let mut len = 0;
    buf[..6].copy_from_slice(b"/proc/");
    len += 6;
    len += write_u32(pid, &mut buf[len..]);
    let s = suffix.as_bytes();
    buf[len..len + s.len()].copy_from_slice(s);
    len += s.len();
    OsStr::from_bytes(&buf[..len])
}
