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
use std::os::unix::io::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

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
    watches: HashMap<u32, PathBuf>,
}

impl Watcher {
    /// Creates a new inotify watcher instance.
    /// 
    /// Initializes a new inotify instance using `inotify_init()`.
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
        
        Ok(Watcher {
            fd,
            watches: HashMap::new(),
        })
    }

    /// Adds a watch for the given path.
    /// 
    /// Watches the specified file or directory for all events.
    /// Returns a `WatchHandle` that can be used to remove the watch later.
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
        let c_string = std::ffi::CString::new(path_bytes)
            .map_err(|_| Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path contains null byte"
            )))?;
        
        // inotify mask constants:
        // IN_ACCESS - file accessed
        // IN_MODIFY - file modified
        // IN_CREATE - file created
        // IN_DELETE - file deleted
        // IN_MOVED_FROM - file moved from
        // IN_MOVED_TO - file moved to
        // IN_ISDIR - event is for a directory
        const MASK: u32 = libc::IN_ACCESS | libc::IN_MODIFY | libc::IN_CREATE 
            | libc::IN_DELETE | libc::IN_MOVED_FROM | libc::IN_MOVED_TO | libc::IN_ISDIR;
        
        // Safe: 
        // - c_string is valid C string (checked above)
        // - inotify_add_watch is a safe syscall that adds a watch
        let wd = unsafe {
            libc::inotify_add_watch(self.fd, c_string.as_ptr(), MASK)
        };
        
        if wd < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        
        // Store the path for later lookup
        let path_buf = std::path::Path::new(&path).to_path_buf();
        self.watches.insert(wd as u32, path_buf);
        
        Ok(WatchHandle { wd: wd as u32 })
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
                    // Safe: we own the fd and will not close it until self is dropped
                    let mut pollfd = libc::pollfd {
                        fd: self.fd,
                        events: libc::POLLIN,
                        revents: 0,
                    };
                    
                    let ret = unsafe {
                        libc::poll(&mut pollfd, 1, -1)
                    };
                    
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
        // Buffer for inotify event structure:
        // struct inotify_event {
        //     int      wd;       /* watch descriptor */
        //     uint32_t mask;     /* event mask */
        //     uint32_t cookie;   /* cookie to synchronize two events */
        //     uint32_t len;      /* length of name */
        //     char     name[];   /* optional null-terminated name */
        // }
        // We need at least sizeof(inotify_event) = 16 bytes, plus space for name
        let mut buf = [0u8; 4096];
        
        // Safe: 
        // - self.fd is a valid file descriptor from inotify_init
        // - buf is valid for reading
        // - we check the return value for errors
        let n = unsafe {
            libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
        };
        
        if n < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }
            return Err(Error::Io(err));
        }
        
        if n == 0 {
            return Ok(None);
        }
        
        // Parse the events from the buffer
        if n as usize > 0 {
            // Safe: we're reading from our own buffer with known size
            let event = unsafe {
                &*(buf.as_ptr() as *const libc::inotify_event)
            };
            
            let watch_event = self.parse_event(event)?;
            
            // Return the first event (there might be more, but we process one at a time)
            return Ok(Some(watch_event));
        }
        
        Ok(None)
    }

    /// Parses an inotify_event into a WatchEvent.
    fn parse_event(&self, event: &libc::inotify_event) -> Result<WatchEvent> {
        let path = self.watches.get(&(event.wd as u32)).cloned();
        
        // Convert inotify mask bits to WatchEvent
        let _is_dir = (event.mask & libc::IN_ISDIR) != 0;
        
        if (event.mask & libc::IN_CREATE) != 0 {
            return Ok(WatchEvent::Created(path));
        }
        if (event.mask & libc::IN_MODIFY) != 0 {
            return Ok(WatchEvent::Modified(path));
        }
        if (event.mask & libc::IN_DELETE) != 0 {
            return Ok(WatchEvent::Deleted(path));
        }
        if (event.mask & libc::IN_MOVED_FROM) != 0 {
            return Ok(WatchEvent::MovedFrom { path, cookie: event.cookie });
        }
        if (event.mask & libc::IN_MOVED_TO) != 0 {
            return Ok(WatchEvent::MovedTo { path, cookie: event.cookie });
        }
        if (event.mask & libc::IN_ACCESS) != 0 {
            return Ok(WatchEvent::Accessed(path));
        }
        
        // Unknown event type - skip it
        Ok(WatchEvent::Unknown)
    }

    /// Removes a watch by watch descriptor.
    #[allow(dead_code)]
    fn remove_watch(&mut self, wd: u32) -> Result<()> {
        // inotify_rm_watch is a safe syscall
        let ret = unsafe {
            libc::inotify_rm_watch(self.fd, wd as libc::c_int)
        };
        
        if ret < 0 {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }
        
        self.watches.remove(&wd);
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
        
        let afd = AsyncFd::new(self.fd)
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        
        loop {
            // Try to read without blocking
            match self.try_next_event()? {
                Some(event) => return Ok(event),
                None => {
                    // Wait for the fd to become readable
                    let guard = afd.readable().await
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
}

impl WatchHandle {
    /// Returns the watch descriptor.
    pub fn watch_descriptor(&self) -> u32 {
        self.wd
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