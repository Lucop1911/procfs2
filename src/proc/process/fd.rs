use crate::{error::{Error, Result}, proc::process::proc_path};

/// Target of a file descriptor symlink in `/proc/PID/fd/`.
///
/// The kernel represents each fd as a symlink whose target encodes
/// the fd type:
/// - Regular files → absolute path
/// - Sockets → `socket:[<inode>]`
/// - Pipes → `pipe:[<inode>]`
/// - Anonymous inodes → `anon_inode:<kind>`
/// - memfd → `/memfd:<name>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdTarget {
    /// A regular file or device node.
    File(std::path::PathBuf),
    /// A network socket with the given kernel inode number.
    ///
    /// The inode can be cross-referenced with `/proc/net/tcp`,
    /// `/proc/net/udp`, etc. to find the actual socket.
    Socket(u64),
    /// A pipe with the given kernel inode number.
    Pipe(u64),
    /// An anonymous inode (epoll, signalfd, timerfd, etc.).
    ///
    /// The inner string identifies the kind, e.g. `[eventpoll]`,
    /// `[signalfd]`, `[timerfd]`.
    AnonInode(Box<str>),
    /// A memfd (memory-backed file descriptor).
    MemFD(Box<str>),
    /// Any target that doesn't match the above patterns.
    Other(Box<str>),
}

impl FdTarget {
    /// Classifies a fd symlink target string into a typed variant.
    ///
    /// The classification is purely string-based, matching the
    /// patterns the kernel uses when creating the symlinks.
    pub fn parse(target: &str) -> Self {
        if let Some(inode) = target.strip_prefix("socket:[") {
            if let Some(inode) = inode.strip_suffix(']') {
                return FdTarget::Socket(inode.parse().unwrap_or(0));
            }
        }
        if let Some(inode) = target.strip_prefix("pipe:[") {
            if let Some(inode) = inode.strip_suffix(']') {
                return FdTarget::Pipe(inode.parse().unwrap_or(0));
            }
        }
        if let Some(kind) = target.strip_prefix("anon_inode:") {
            return FdTarget::AnonInode(kind.to_string().into_boxed_str());
        }
        if let Some(name) = target.strip_prefix("/memfd:") {
            return FdTarget::MemFD(name.to_string().into_boxed_str());
        }
        if target.starts_with('/') {
            return FdTarget::File(std::path::PathBuf::from(target));
        }
        FdTarget::Other(target.to_string().into_boxed_str())
    }
}

/// A single open file descriptor.
#[derive(Debug)]
pub struct Fd {
    /// The file descriptor number (e.g. 0 for stdin).
    pub number: i32,
    /// What this fd points to.
    pub target: FdTarget,
}

/// Reads `/proc/PID/fd/` and returns open file descriptors.
///
/// Iterates the directory, reads each symlink, and classifies
/// the target as a file, socket, pipe, anon-inode, or other.
pub fn read_fds(pid: u32) -> Result<Vec<Fd>> {
    let mut buf = [0u8; 32];
    let path = proc_path(&mut buf, pid, "/fd");
    
    let entries = std::fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            Error::PermissionDenied(std::path::PathBuf::from(path))
        } else {
            Error::Io {
                path: Some(std::path::PathBuf::from(path)),
                error: e,
            }
        }
    })?;

    let mut fds = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| Error::Io {
            path: Some(std::path::PathBuf::from(path)),
            error: e,
        })?;
        let fd_num = entry.file_name().to_string_lossy().parse::<i32>().ok();
        if let Some(num) = fd_num {
            let target = std::fs::read_link(entry.path()).map_err(|e| Error::Io {
                path: Some(entry.path().to_path_buf()),
                error: e,
            })?;
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
