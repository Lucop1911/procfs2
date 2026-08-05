/// Target of a file descriptor symlink in `/proc/PID/fd/`.
///
/// The kernel represents each fd as a symlink whose target encodes
/// the fd type:
/// - Regular files → absolute path
/// - Sockets → `socket:[<inode>]`
/// - Pipes → `pipe:[<inode>]`
/// - Anonymous inodes → `anon_inode:<kind>`
/// - memfd → `/memfd:<name>`
#[derive(Debug)]
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
