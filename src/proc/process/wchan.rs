use crate::error::{Error, Result};
use crate::util::parse;

/// Where a task is blocked in the kernel, as reported by `/proc/PID/wchan`.
///
/// The kernel writes one of three things: the name of the function the
/// task is sleeping in (`do_wait`), the raw address of that function
/// when symbol lookup is unavailable (`ffffffff8a2b1c40`), or `0` when
/// there is nothing to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wchan {
    /// The kernel wrote `0`.
    ///
    /// That covers a runnable task, a task blocked outside the kernel
    /// (a page fault, say), and an address withheld by
    /// `/proc/sys/kernel/kptr_restrict` — which hides other users'
    /// processes at `1` and everyone at `2`. The three are not
    /// distinguishable from this file, so nothing more specific is
    /// claimed here.
    RunningOrUnavailable,
    /// Name of the kernel function the task is blocked in.
    ///
    /// Only emitted when the kernel is built with `CONFIG_KALLSYMS`. A
    /// name that happens to consist solely of hex digits is
    /// indistinguishable from an address and is reported as
    /// [`Address`](Self::Address).
    Symbol(String),
    /// Address of the kernel function the task is blocked in.
    ///
    /// The kernel prints this in hex, and emits it for kernel threads
    /// (which have no address space) and for user processes whenever
    /// the symbol could not be resolved. Never `0`: the kernel's zero
    /// is reported as [`RunningOrUnavailable`](Self::RunningOrUnavailable).
    Address(u64),
}

impl Wchan {
    /// Parses a `/proc/PID/wchan` file from raw bytes.
    ///
    /// The file holds a single token followed by a newline, so the
    /// whole read is one field with no line structure to speak of.
    /// Being a bare snapshot of a scheduling decision, it is stale the
    /// moment it is read, and it says nothing about *why* the task is
    /// blocked — `/proc/PID/stack` is the privileged way to find out.
    pub fn from_bytes(bytes: &[u8]) -> Result<Wchan> {
        let field = parse::trim_end(bytes);
        if field.is_empty() || field == b"0" {
            return Ok(Wchan::RunningOrUnavailable);
        }

        if let Ok(address) = parse::parse_hex_u64(field) {
            return Ok(Wchan::Address(address));
        }

        let symbol = std::str::from_utf8(field).map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<wchan>"),
            line: 0,
            msg: "invalid utf8 in wchan symbol",
        })?;

        Ok(Wchan::Symbol(symbol.to_owned()))
    }
}
