use std::path::PathBuf;

use crate::error::{Error, Result};

/// Entry type tags used in the auxiliary vector.
///
/// These are the `AT_*` constants from `<elf.h>`. The set is open
/// ended, so [`AuxvEntry::type_tag`] always carries the raw tag
/// even for types not listed here.
pub mod auxv_type {
    /// End of the vector.
    pub const AT_NULL: u64 = 0;
    /// Address of the program headers.
    pub const AT_PHDR: u64 = 3;
    /// Size of one program header entry.
    pub const AT_PHENT: u64 = 4;
    /// Number of program headers.
    pub const AT_PHNUM: u64 = 5;
    /// System page size in bytes.
    pub const AT_PAGESZ: u64 = 6;
    /// Base address of the interpreter.
    pub const AT_BASE: u64 = 7;
    /// Entry point of the executable.
    pub const AT_ENTRY: u64 = 9;
    /// Real user ID.
    pub const AT_UID: u64 = 11;
    /// Effective user ID.
    pub const AT_EUID: u64 = 12;
    /// Real group ID.
    pub const AT_GID: u64 = 13;
    /// Effective group ID.
    pub const AT_EGID: u64 = 14;
    /// Platform name.
    pub const AT_PLATFORM: u64 = 15;
    /// Hardware capability bits.
    pub const AT_HWCAP: u64 = 16;
    /// Clock ticks per second.
    pub const AT_CLKTCK: u64 = 17;
    /// Set when running setuid/setgid.
    pub const AT_SECURE: u64 = 23;
    /// Pointer to 16 random bytes.
    pub const AT_RANDOM: u64 = 25;
    /// Executable name.
    pub const AT_EXECFN: u64 = 31;
    /// Address of the vDSO.
    pub const AT_SYSINFO_EHDR: u64 = 33;
}

/// A single entry from `/proc/PID/auxv`.
///
/// The auxiliary vector is a list of `(type, value)` pairs the
/// kernel hands to userspace at exec time. Each field is a native
/// machine word, so both are represented as `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxvEntry {
    /// Entry type tag (one of the `AT_*` constants).
    pub type_tag: u64,
    /// Entry value. Its meaning depends on `type_tag`.
    pub value: u64,
}

/// The auxiliary vector of a process.
///
/// Sourced from `/proc/PID/auxv`. It is populated once at startup
/// and does not change while the process runs.
#[derive(Debug, Clone)]
pub struct Auxv {
    /// Entries in file order, excluding the terminating `AT_NULL`.
    pub entries: Vec<AuxvEntry>,
}

impl Auxv {
    /// Parses a `/proc/PID/auxv` file from raw bytes.
    ///
    /// The file is a flat array of `(type, value)` word pairs in
    /// native byte order, terminated by an `AT_NULL` entry.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() % 16 != 0 {
            return Err(Error::Parse {
                path: PathBuf::from("<auxv>"),
                line: 0,
                msg: "size is not a multiple of 16 bytes",
            });
        }

        let mut entries = Vec::new();

        for chunk in bytes.chunks_exact(16) {
            let type_tag = u64::from_ne_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
            let value = u64::from_ne_bytes([
                chunk[8], chunk[9], chunk[10], chunk[11], chunk[12], chunk[13], chunk[14],
                chunk[15],
            ]);

            if type_tag == auxv_type::AT_NULL {
                break;
            }

            entries.push(AuxvEntry { type_tag, value });
        }

        Ok(Auxv { entries })
    }

    /// Returns the value of the first entry with the given type tag.
    pub fn get(&self, type_tag: u64) -> Option<u64> {
        self.entries
            .iter()
            .find(|e| e.type_tag == type_tag)
            .map(|e| e.value)
    }

    /// Returns the system page size from `AT_PAGESZ`.
    ///
    /// Falls back to 4096 if the entry is missing, which should not
    /// happen on a normal Linux system.
    pub fn page_size(&self) -> u64 {
        self.get(auxv_type::AT_PAGESZ).unwrap_or(4096)
    }
}
