use crate::error::{Error, Result};
use crate::proc::process::{Process, proc_path};

/// Iterates over the threads of a process.
///
/// Scans `/proc/PID/task/` for numeric entries. Each thread is
/// returned as a `Process` with its TID as the `pid` field,
/// allowing all `Process` methods to be called on individual
/// threads.
pub fn read_threads(pid: u32) -> impl Iterator<Item = Result<Process>> {
    let mut buf = [0u8; 32];
    let path = proc_path(&mut buf, pid, "/task");
    let entries = match std::fs::read_dir(path) {
        Ok(iter) => iter,
        Err(e) => {
            return vec![Err(Error::Io {
                path: Some(std::path::PathBuf::from(&path)),
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
                    let tid = name_str.parse::<u32>().ok()?;
                    Some(Ok(Process { pid: tid }))
                } else {
                    None
                }
            }
            Err(e) => Some(Err(Error::Io {
                path: Some(std::path::PathBuf::from(&path)),
                error: e,
            })),
        })
        .collect::<Vec<_>>()
        .into_iter()
}
