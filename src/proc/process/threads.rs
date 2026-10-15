use crate::error::{Error, Result};
use crate::proc::process::{Process, with_proc_path};

/// Iterates over the threads of a process.
///
/// Scans `/proc/PID/task/` for numeric entries. Each thread is
/// returned as a `Process` with its TID as the `pid` field,
/// allowing all `Process` methods to be called on individual
/// threads.
pub fn read_threads(pid: u32) -> impl Iterator<Item = Result<Process>> {
    let threads = with_proc_path(pid, "/task", |path| {
        let entries = match std::fs::read_dir(path) {
            Ok(iter) => iter,
            Err(e) => {
                return vec![Err(Error::Io {
                    path: Some(path.to_path_buf()),
                    error: e,
                })];
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
                Err(e) => Some(Err(Error::Io {
                    path: Some(path.to_path_buf()),
                    error: e,
                })),
            })
            .collect::<Vec<_>>()
    });

    threads.into_iter()
}
