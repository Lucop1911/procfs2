//! Inotify-based file system watcher for procfs2.
//!
//! This module provides a safe interface to Linux's inotify API for watching
//! changes to `/proc` and `/sys` files. The watcher supports both blocking
//! and non-blocking event retrieval.
//!
//! # Usage
//!
//! ```ignore
//! use procfs2::watch::Watcher;
//!
//! let mut watcher = Watcher::new()?;
//! watcher.watch("/proc/stat")?;
//!
//! loop {
//!     match watcher.try_next_event()? {
//!         Some(event) => println!("{:?}", event),
//!         None => continue,
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};

/// Inotify-based file system watcher.
///
/// Watches files and directories for changes using Linux's inotify API.
/// Supports watching multiple paths and retrieving events in both blocking
/// and non-blocking modes.
pub struct Watcher {
    /// The inotify file descriptor.
    fd: RawFd,
    /// Mapping from watch descriptors to watched paths.
    watches: Arc<Mutex<HashMap<u32, PathBuf>>>,
}

impl Watcher {
    /// Creates a new inotify watcher instance.
    ///
    /// Initializes a new inotify instance using `inotify_init()` and sets the
    /// file descriptor to non-blocking mode so `try_next_event` can return
    /// immediately when no events are available.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` if the inotify initialization fails.
    pub fn new() -> Result<Self> {
        // Safe: inotify_init is a safe syscall that only creates an inotify instance.
        // It returns a valid file descriptor or -1 on error (errno set).
        let fd = unsafe { libc::inotify_init() };
        if fd < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }

        // Set O_NONBLOCK on fd so read returns EAGAIN instead of blocking
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }

        Ok(Watcher {
            fd,
            watches: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Adds a watch for the given path.
    ///
    /// Watches the specified file or directory for all events.
    /// Returns a `WatchHandle` that will remove the watch when dropped.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to watch (must be a valid path string)
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` if the path cannot be watched or doesn't exist.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<WatchHandle> {
        let path = path.as_ref();

        // Convert path to C string for syscall
        let path_bytes = path.as_os_str().as_bytes();
        let c_string = std::ffi::CString::new(path_bytes).map_err(|_| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path contains null byte",
            ))
        })?;

        // inotify mask constants:
        const MASK: u32 = libc::IN_ACCESS
            | libc::IN_MODIFY
            | libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO
            | libc::IN_ISDIR;

        // Safe: inotify_add_watch is a syscall
        let wd = unsafe { libc::inotify_add_watch(self.fd, c_string.as_ptr(), MASK) };

        if wd < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }

        // Store the path for later lookup
        let path_buf = std::path::Path::new(&path).to_path_buf();
        self.watches.lock().unwrap().insert(wd as u32, path_buf);

        Ok(WatchHandle {
            wd: wd as u32,
            fd: self.fd,
            watches: Arc::clone(&self.watches),
        })
    }

    /// Retrieves the next watch event (blocking).
    ///
    /// Blocks until an inotify event is available, then returns it.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` if reading fails.
    pub fn next_event(&self) -> Result<WatchEvent> {
        loop {
            match self.try_next_event()? {
                Some(event) => return Ok(event),
                None => {
                    // Wait for data to be available using poll
                    let mut pollfd = libc::pollfd {
                        fd: self.fd,
                        events: libc::POLLIN,
                        revents: 0,
                    };

                    let ret = unsafe { libc::poll(&mut pollfd, 1, -1) };

                    if ret < 0 {
                        return Err(Error::Io(std::io::Error::last_os_error()));
                    }
                }
            }
        }
    }

    /// Retrieves the next watch event (non-blocking).
    ///
    /// Returns immediately with `None` if no events are available.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` if reading fails.
    pub fn try_next_event(&self) -> Result<Option<WatchEvent>> {
        // Buffer for inotify events
        let mut buf = [0u8; 4096];

        // Read from the non-blocking fd
        let n = unsafe { libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

        if n < 0 {
            let err = std::io::Error::last_os_error();
            // EAGAIN / WouldBlock means no data available on non-blocking fd
            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(Error::Io(err));
        }

        if n == 0 {
            return Ok(None);
        }

        // Parse potentially multiple events in the buffer; return the first parsed event
        let mut offset = 0usize;
        while offset < n as usize {
            // Ensure there is enough space for the fixed-size header
            if offset + std::mem::size_of::<libc::inotify_event>() > n as usize {
                break;
            }

            // SAFETY: buffer is valid and large enough for header slice
            let ev_ptr = unsafe { buf.as_ptr().add(offset) as *const libc::inotify_event };
            let ev = unsafe { &*ev_ptr };

            let name_len = ev.len as usize;
            let header_size = std::mem::size_of::<libc::inotify_event>();
            let name_start = offset + header_size;
            let name_end = name_start + name_len;

            let name = if name_len > 0 && name_end <= n as usize {
                // Extract name bytes (null-terminated)
                let raw = &buf[name_start..name_end];
                // Trim trailing nulls
                let trimmed = if let Some(pos) = raw.iter().position(|&b| b == 0) {
                    &raw[..pos]
                } else {
                    raw
                };
                Some(std::path::PathBuf::from(std::ffi::OsString::from_vec(trimmed.to_vec())))
            } else {
                None
            };

            // Move offset to next event (aligned to sizeof inotify_event + name rounded up)
            let total_event_size = header_size + name_len;
            let aligned = (total_event_size + std::mem::size_of::<libc::c_long>() - 1)
                & !(std::mem::size_of::<libc::c_long>() - 1);

            offset += aligned;

            // Build path: if watch path exists and name present, join
            let base_path = self
                .watches
                .lock()
                .unwrap()
                .get(&(ev.wd as u32))
                .cloned();

            let full_path = match (&base_path, &name) {
                (Some(bp), Some(nm)) => Some(bp.join(nm)),
                (Some(bp), None) => Some(bp.clone()),
                _ => None,
            };

            // Convert masks to events
            if (ev.mask & libc::IN_CREATE) != 0 {
                return Ok(Some(WatchEvent::Created(full_path)));
            }
            if (ev.mask & libc::IN_MODIFY) != 0 {
                return Ok(Some(WatchEvent::Modified(full_path)));
            }
            if (ev.mask & libc::IN_DELETE) != 0 {
                return Ok(Some(WatchEvent::Deleted(full_path)));
            }
            if (ev.mask & libc::IN_MOVED_FROM) != 0 {
                return Ok(Some(WatchEvent::MovedFrom {
                    path: full_path,
                    cookie: ev.cookie,
                }));
            }
            if (ev.mask & libc::IN_MOVED_TO) != 0 {
                return Ok(Some(WatchEvent::MovedTo {
                    path: full_path,
                    cookie: ev.cookie,
                }));
            }
            if (ev.mask & libc::IN_ACCESS) != 0 {
                return Ok(Some(WatchEvent::Accessed(full_path)));
            }

            // Otherwise continue to next event in buffer
        }

        Ok(None)
    }

    /// Removes a watch by watch descriptor.
    #[allow(dead_code)]
    fn remove_watch(&mut self, wd: u32) -> Result<()> {
        // inotify_rm_watch is a safe syscall
        let ret = unsafe { libc::inotify_rm_watch(self.fd, wd as libc::c_int) };

        if ret < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }

        self.watches.lock().unwrap().remove(&wd);
        Ok(())
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // Safe: close is always safe for a valid file descriptor
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[cfg(feature = "async")]
impl Watcher {
    /// Retrieves the next watch event asynchronously.
    ///
    /// Requires the `async` feature to be enabled.
    ///
    /// # Errors
    ///
    /// Returns `Error::Io` if the async operation fails.
    pub async fn next_event_async(&self) -> Result<WatchEvent> {
        // Use tokio's async I/O to wait for events
        use tokio::io::unix::AsyncFd;

        let afd =
            AsyncFd::new(self.fd).map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;

        loop {
            // Try to read without blocking
            match self.try_next_event()? {
                Some(event) => return Ok(event),
                None => {
                    // Wait for the fd to become readable
                    let guard = afd
                        .readable()
                        .await
                        .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
                    let mut guard = guard;
                    guard.clear_ready();
                }
            }
        }
    }
}

/// Handle for a watched path.
/// 
/// Dropping a `WatchHandle` removes the associated watch.
pub struct WatchHandle {
    /// The watch descriptor.
    wd: u32,
    /// The inotify fd (needed to remove the watch on drop).
    fd: RawFd,
    /// Shared watches map for bookkeeping.
    watches: Arc<Mutex<HashMap<u32, PathBuf>>>,
}

impl WatchHandle {
    /// Returns the watch descriptor.
    pub fn watch_descriptor(&self) -> u32 {
        self.wd
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        // Attempt to remove the watch; ignore errors in Drop
        unsafe {
            let _ = libc::inotify_rm_watch(self.fd, self.wd as libc::c_int);
        }
        let _ = self.watches.lock().map(|mut m| m.remove(&self.wd));
    }
}

/// Events that can occur on a watched path.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file or directory was created.
    Created(Option<PathBuf>),
    /// A file or directory was modified.
    Modified(Option<PathBuf>),
    /// A file or directory was deleted.
    Deleted(Option<PathBuf>),
    /// A file or directory was moved from this location.
    ///
    /// The `cookie` field can be used to match with a corresponding `MovedTo` event.
    MovedFrom { path: Option<PathBuf>, cookie: u32 },
    /// A file or directory was moved to this location.
    ///
    /// The `cookie` field can be used to match with a corresponding `MovedFrom` event.
    MovedTo { path: Option<PathBuf>, cookie: u32 },
    /// A file was accessed (read).
    Accessed(Option<PathBuf>),
    /// An unknown event occurred.
    Unknown,
}

impl WatchEvent {
    /// Returns the path associated with this event, if available.
    pub fn path(&self) -> Option<&Path> {
        match self {
            WatchEvent::Created(p) => p.as_deref(),
            WatchEvent::Modified(p) => p.as_deref(),
            WatchEvent::Deleted(p) => p.as_deref(),
            WatchEvent::MovedFrom { path, .. } => path.as_deref(),
            WatchEvent::MovedTo { path, .. } => path.as_deref(),
            WatchEvent::Accessed(p) => p.as_deref(),
            WatchEvent::Unknown => None,
        }
    }
}
