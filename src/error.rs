use std::fmt;
use std::path::PathBuf;

/// Errors that can occur when reading or parsing `/proc` and `/sys` entries.
///
/// All variants are designed to be cloneable and comparable for testing purposes.
/// IO errors from the underlying filesystem are wrapped rather than propagated raw
/// to maintain a single error surface.
#[derive(Debug)]
pub enum Error {
    /// A wrapped [`std::io::Error`] from filesystem operations.
    ///
    /// `path` is the `/proc` or `/sys` file that was being accessed when
    /// the operation failed, or `None` when there is no single file (e.g.
    /// an inotify fd operation or a translated `last_os_error`).
    Io {
        path: Option<PathBuf>,
        error: std::io::Error,
    },

    /// A parsing failure at a specific location within a file.
    ///
    /// `line` is 1-indexed. `msg` is a static string describing what went wrong.
    Parse {
        path: PathBuf,
        line: usize,
        msg: &'static str,
    },

    /// The target process exited between the time its PID was discovered
    /// and the time its `/proc/PID` entry was opened.
    ProcessGone(u32),

    /// Insufficient permissions to read a `/proc` or `/sys` path.
    ///
    /// Common for `/proc/PID/exe`, `/proc/PID/environ`, and other
    /// restricted entries of processes owned by other users.
    PermissionDenied(PathBuf),

    /// The running kernel does not meet the minimum version required
    /// for a particular feature.
    UnsupportedKernel {
        required: KernelVersion,
        found: KernelVersion,
    },
}

impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            // Rebuild the OS error preserving its raw error code rather than
            // wrapping the Display text, which would drop the code and turn
            // the message into a plain custom string.
            Error::Io { path, error: e } => Error::Io {
                path: path.clone(),
                error: match e.raw_os_error() {
                    Some(code) => std::io::Error::from_raw_os_error(code),
                    None => std::io::Error::new(e.kind(), "io error"),
                },
            },
            Error::Parse { path, line, msg } => Error::Parse {
                path: path.clone(),
                line: *line,
                msg,
            },
            Error::ProcessGone(pid) => Error::ProcessGone(*pid),
            Error::PermissionDenied(path) => Error::PermissionDenied(path.clone()),
            Error::UnsupportedKernel { required, found } => Error::UnsupportedKernel {
                required: required.clone(),
                found: found.clone(),
            },
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Error::Io {
                    path: p1,
                    error: e1,
                },
                Error::Io {
                    path: p2,
                    error: e2,
                },
            ) => {
                p1 == p2
                    && e1.kind() == e2.kind()
                    && match (e1.raw_os_error(), e2.raw_os_error()) {
                        (Some(c1), Some(c2)) => c1 == c2,
                        (None, None) => true,
                        _ => false,
                    }
            }
            (
                Error::Parse {
                    path: p1,
                    line: l1,
                    msg: m1,
                },
                Error::Parse {
                    path: p2,
                    line: l2,
                    msg: m2,
                },
            ) => p1 == p2 && l1 == l2 && m1 == m2,
            (Error::ProcessGone(p1), Error::ProcessGone(p2)) => p1 == p2,
            (Error::PermissionDenied(p1), Error::PermissionDenied(p2)) => p1 == p2,
            (
                Error::UnsupportedKernel {
                    required: r1,
                    found: f1,
                },
                Error::UnsupportedKernel {
                    required: r2,
                    found: f2,
                },
            ) => r1 == r2 && f1 == f2,
            _ => false,
        }
    }
}

impl Eq for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io {
                path: Some(path),
                error: e,
            } => {
                write!(f, "IO error reading {:?}: {e}", path.display())
            }
            Error::Io {
                path: None,
                error: e,
            } => write!(f, "IO error: {e}"),
            Error::Parse { path, line, msg } => {
                write!(f, "Parse error at {:?}:{line}: {msg}", path.display())
            }
            Error::ProcessGone(pid) => write!(f, "Process {pid} no longer exists"),
            Error::PermissionDenied(path) => write!(f, "Permission denied: {:?}", path.display()),
            Error::UnsupportedKernel { required, found } => {
                write!(
                    f,
                    "Kernel version {}.{}.{} required, but found {}.{}.{}",
                    required.major,
                    required.minor,
                    required.patch,
                    found.major,
                    found.minor,
                    found.patch,
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// Represents a Linux kernel version as three numeric components.
///
/// Parsed from `/proc/version`. Used for runtime feature guards where
/// certain `/proc` or `/sys` entries only exist on newer kernels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl KernelVersion {
    /// Reads and parses the running kernel version from `/proc/version`.
    ///
    /// The version string in `/proc/version` looks like:
    /// `Linux version 5.15.0-91-generic (buildd@...) (gcc ...) #101-Ubuntu ...`
    /// This function locates the `version ` prefix and extracts the first
    /// three dot-separated numeric components.
    pub fn current() -> Result<Self> {
        let path = PathBuf::from("/proc/version");
        let bytes = std::fs::read(&path).map_err(|e| Error::Io {
            path: Some(path.clone()),
            error: e,
        })?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::Parse {
            path: PathBuf::from("/proc/version"),
            line: 0,
            msg: "invalid utf8 in /proc/version",
        })?;

        let version_start = text
            .find("version ")
            .map(|i| i + 8)
            .ok_or_else(|| Error::Parse {
                path: PathBuf::from("/proc/version"),
                line: 0,
                msg: "missing 'version' keyword",
            })?;

        let version_end = text[version_start..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| version_start + i)
            .unwrap_or(text.len());

        let version_str = &text[version_start..version_end];
        let parts: Vec<&str> = version_str.split('.').collect();

        if parts.len() < 3 {
            return Err(Error::Parse {
                path: PathBuf::from("/proc/version"),
                line: 0,
                msg: "invalid version format",
            });
        }

        let major = parts[0].parse::<u32>().map_err(|_| Error::Parse {
            path: PathBuf::from("/proc/version"),
            line: 0,
            msg: "invalid major version",
        })?;
        let minor = parts[1].parse::<u32>().map_err(|_| Error::Parse {
            path: PathBuf::from("/proc/version"),
            line: 0,
            msg: "invalid minor version",
        })?;
        let patch = parts[2].parse::<u32>().map_err(|_| Error::Parse {
            path: PathBuf::from("/proc/version"),
            line: 0,
            msg: "invalid patch version",
        })?;

        Ok(KernelVersion {
            major,
            minor,
            patch,
        })
    }

    /// Returns `true` if this kernel version is at least `major.minor`.
    ///
    /// Patch level is intentionally ignored for feature gating — if a
    /// feature landed in 5.10, it's assumed available on all 5.10.x.
    pub fn at_least(&self, major: u32, minor: u32) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Fails with [`Error::UnsupportedKernel`] unless the running
    /// kernel is at least `major.minor`.
    ///
    /// Patch level is intentionally ignored, matching [`Self::at_least`].
    pub fn require(&self, major: u32, minor: u32) -> Result<()> {
        if self.at_least(major, minor) {
            Ok(())
        } else {
            Err(Error::UnsupportedKernel {
                required: KernelVersion {
                    major,
                    minor,
                    patch: 0,
                },
                found: self.clone(),
            })
        }
    }
}

/// Alias for `std::result::Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::{Error, KernelVersion};

    fn err_noent() -> Error {
        Error::Io {
            path: Some(std::path::PathBuf::from("/proc/nonexistent")),
            error: std::io::Error::from_raw_os_error(2), // ENOENT
        }
    }

    #[test]
    fn io_clone_preserves_code_and_path() {
        let err = err_noent();
        let cloned = err.clone();
        match (&err, &cloned) {
            (
                Error::Io {
                    path: p1,
                    error: e1,
                },
                Error::Io {
                    path: p2,
                    error: e2,
                },
            ) => {
                assert_eq!(p1, p2);
                assert_eq!(e1.raw_os_error(), Some(2));
                assert_eq!(e2.raw_os_error(), Some(2));
            }
            _ => panic!("expected Io variants"),
        }
    }

    #[test]
    fn io_clone_eq_self() {
        let err = err_noent();
        assert_eq!(err.clone(), err);
    }

    #[test]
    fn io_eq_distinguishes_path() {
        let a = Error::Io {
            path: Some(std::path::PathBuf::from("/proc/a")),
            error: std::io::Error::from_raw_os_error(2),
        };
        let b = Error::Io {
            path: Some(std::path::PathBuf::from("/proc/b")),
            error: std::io::Error::from_raw_os_error(2),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn io_source_is_forwarded() {
        let err = err_noent();
        let source = std::error::Error::source(&err);
        if let Some(src) = source {
            assert_eq!(src.to_string(), "No such file or directory (os error 2)");
        } else {
            panic!("expected an Io source");
        }
    }

    #[test]
    fn unsupported_kernel_at_least() {
        let k = KernelVersion {
            major: 5,
            minor: 15,
            patch: 0,
        };
        assert!(k.at_least(5, 15));
        assert!(k.at_least(5, 14));
        assert!(!k.at_least(6, 0));
    }
}
