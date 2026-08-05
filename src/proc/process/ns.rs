use crate::error::{Error, Result};

/// Namespace inode numbers for a process.
///
/// Each namespace is exposed as a symlink under `/proc/PID/ns/`
/// whose target is of the form `type:[<inode>]`. Reading the
/// symlinks gives the inode number, which can be used to compare
/// whether two processes share the same namespace.
///
/// Fields are `Option<u64>` because some namespaces (e.g. `time`)
/// only exist on newer kernels and may be absent.
#[derive(Debug)]
pub struct Namespaces {
    /// Cgroup namespace.
    pub cgroup: Option<u64>,
    /// IPC namespace (System V IPC, POSIX message queues).
    pub ipc: Option<u64>,
    /// Mount namespace.
    pub mnt: Option<u64>,
    /// Network namespace.
    pub net: Option<u64>,
    /// PID namespace.
    pub pid: Option<u64>,
    /// Time namespace (kernel 5.6+).
    pub time: Option<u64>,
    /// Time namespace for children (kernel 5.6+).
    pub time_for_children: Option<u64>,
    /// User namespace.
    pub user: Option<u64>,
    /// UTS namespace (hostname, NIS domain name).
    pub uts: Option<u64>,
}

impl Namespaces {
    /// Reads all symlinks under `/proc/PID/ns/` and extracts inode
    /// numbers for each recognized namespace.
    ///
    /// Unknown symlinks (e.g. future kernel additions) are silently
    /// ignored. Missing namespaces remain as `None`.
    pub fn from_dir(path: &str) -> Result<Self> {
        let mut cgroup = None;
        let mut ipc = None;
        let mut mnt = None;
        let mut net = None;
        let mut pid = None;
        let mut time = None;
        let mut time_for_children = None;
        let mut user = None;
        let mut uts = None;

        let entries = std::fs::read_dir(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                Error::PermissionDenied(std::path::PathBuf::from(path))
            } else {
                Error::Io(e)
            }
        })?;

        for entry in entries {
            let entry = entry.map_err(Error::Io)?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            let target = std::fs::read_link(entry.path()).map_err(Error::Io)?;
            let target_str = target.to_string_lossy();

            if let Some(inode) = parse_ns_inode(&target_str) {
                match name_str.as_ref() {
                    "cgroup" => cgroup = Some(inode),
                    "ipc" => ipc = Some(inode),
                    "mnt" => mnt = Some(inode),
                    "net" => net = Some(inode),
                    "pid" => pid = Some(inode),
                    "time" => time = Some(inode),
                    "time_for_children" => time_for_children = Some(inode),
                    "user" => user = Some(inode),
                    "uts" => uts = Some(inode),
                    _ => {}
                }
            }
        }

        Ok(Namespaces {
            cgroup,
            ipc,
            mnt,
            net,
            pid,
            time,
            time_for_children,
            user,
            uts,
        })
    }
}

/// Extracts the inode number from a namespace symlink target.
///
/// Namespace targets look like `mnt:[4026531840]`. Returns `None`
/// if the format doesn't match.
fn parse_ns_inode(target: &str) -> Option<u64> {
    let s = target.strip_prefix('[')?;
    let s = s.strip_suffix(']')?;
    s.parse::<u64>().ok()
}
