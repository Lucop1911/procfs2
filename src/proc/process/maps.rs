use std::ops::Range;

use bitflags::bitflags;

use crate::error::{Error, Result};
use crate::util::parse;

bitflags! {
    /// Memory region access permissions.
    ///
    /// Parsed from the 4-character permission string in `/proc/PID/maps`:
    /// `rwxp` or `rwxs`. The fifth character is always `s` (shared) or
    /// `p` (private), mapped to the [`SHARED`](Self::SHARED) and
    /// [`PRIVATE`](Self::PRIVATE) flags respectively.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct MapPermissions: u8 {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const EXEC    = 1 << 2;
        const SHARED  = 1 << 3;
        const PRIVATE = 1 << 4;
    }
}

/// The backing source of a memory mapping.
///
/// Special regions (heap, stack, vdso) are identified by their
/// bracketed names in `/proc/PID/maps`. Everything else is treated
/// as a file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapPathname {
    /// A file-backed mapping with the given path.
    Path(std::path::PathBuf),
    /// The process heap (`[heap]`).
    Heap,
    /// The initial process stack (`[stack]`).
    Stack,
    /// The virtual dynamic shared object (`[vdso]`).
    ///
    /// A kernel-provided shared library for fast system calls.
    Vdso,
    /// Legacy vsyscall page (`[vsyscall]`), present on x86_64
    /// kernels with compatibility enabled.
    Vsyscall,
    /// Variable storage area (`[vvar]`), used alongside vdso.
    Vvar,
    /// An anonymous mapping with no backing file.
    Anonymous,
}

/// A single entry from `/proc/PID/maps`.
///
/// Describes one virtual memory region: its address range,
/// permissions, offset into the backing file, device/inode of the
/// backing file, and optional pathname.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMap {
    /// Virtual address range of this mapping.
    pub address: Range<u64>,
    pub perms: MapPermissions,
    /// Offset into the backing file (zero for anonymous mappings).
    pub offset: u64,
    /// Device major and minor numbers of the backing file.
    pub device: (u32, u32),
    /// Inode number of the backing file (zero for anonymous).
    pub inode: u64,
    pub pathname: MapPathname,
}

/// A single entry from `/proc/PID/smaps` with per-region memory details.
///
/// Each smaps block starts with a maps-style header line followed
/// by key-value lines with memory statistics in kilobytes.
#[derive(Debug, Clone)]
pub struct MemoryMapDetail {
    pub address: Range<u64>,
    pub perms: MapPermissions,
    pub offset: u64,
    pub device: (u32, u32),
    pub inode: u64,
    pub pathname: MapPathname,
    /// Total size of the mapping in kB.
    pub size_kb: u64,
    /// Resident set size in kB (pages actually in RAM).
    pub rss_kb: u64,
    /// Proportional set size in kB.
    ///
    /// PSS divides shared pages proportionally among the processes
    /// that map them, giving a more accurate per-process memory
    /// footprint than RSS.
    pub pss_kb: u64,
    /// Shared pages that are clean (not modified).
    pub shared_clean_kb: u64,
    /// Shared pages that are dirty (modified).
    pub shared_dirty_kb: u64,
    /// Private pages that are clean.
    pub private_clean_kb: u64,
    /// Private pages that are dirty.
    pub private_dirty_kb: u64,
    /// Pages that have been accessed (referenced bit set).
    pub referenced_kb: u64,
    /// Pages not backed by a file (heap, stack, anonymous mmap).
    pub anonymous_kb: u64,
    /// Pages swapped out.
    pub swap_kb: u64,
}

/// Rollup summary of `/proc/PID/smaps_rollup`.
///
/// Unlike [`MemoryMapDetail`], which exposes every memory region,
/// this is a single aggregate block with the same key-value fields.
/// It is far cheaper to read than `/proc/PID/smaps` and is preferred
/// when only totals are needed.
#[derive(Debug, Clone)]
pub struct SmapsRollup {
    /// Resident set size in kB (pages actually in RAM).
    pub rss_kb: u64,
    /// Proportional set size in kB.
    pub pss_kb: u64,
    /// Shared pages that are clean (not modified).
    pub shared_clean_kb: u64,
    /// Shared pages that are dirty (modified).
    pub shared_dirty_kb: u64,
    /// Private pages that are clean.
    pub private_clean_kb: u64,
    /// Private pages that are dirty.
    pub private_dirty_kb: u64,
    /// Pages that have been accessed (referenced bit set).
    pub referenced_kb: u64,
    /// Pages not backed by a file (heap, stack, anonymous mmap).
    pub anonymous_kb: u64,
    /// Pages swapped out.
    pub swap_kb: u64,
}

impl SmapsRollup {
    /// Parses a `/proc/PID/smaps_rollup` file from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut rollup = SmapsRollup {
            rss_kb: 0,
            pss_kb: 0,
            shared_clean_kb: 0,
            shared_dirty_kb: 0,
            private_clean_kb: 0,
            private_dirty_kb: 0,
            referenced_kb: 0,
            anonymous_kb: 0,
            swap_kb: 0,
        };

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            let (key, value) = match parse::parse_key_value_line(line) {
                Some(kv) => kv,
                None => continue,
            };

            // Values carry a ` kB` suffix. Take the first token.
            let val = parse::trim_start(value)
                .split(|&b| b == b' ' || b == b'\t')
                .next()
                .and_then(|t| parse::parse_dec_u64(t).ok())
                .unwrap_or(0);

            match key {
                b"Rss" => rollup.rss_kb = val,
                b"Pss" => rollup.pss_kb = val,
                b"Shared_Clean" => rollup.shared_clean_kb = val,
                b"Shared_Dirty" => rollup.shared_dirty_kb = val,
                b"Private_Clean" => rollup.private_clean_kb = val,
                b"Private_Dirty" => rollup.private_dirty_kb = val,
                b"Referenced" => rollup.referenced_kb = val,
                b"Anonymous" => rollup.anonymous_kb = val,
                b"Swap" => rollup.swap_kb = val,
                _ => {}
            }
        }

        Ok(rollup)
    }
}

impl MemoryMap {
    /// Parses all lines of a `/proc/PID/maps` file.
    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>> {
        let mut maps = Vec::new();

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            maps.push(Self::parse_line(line)?);
        }

        Ok(maps)
    }

    fn parse_line(line: &[u8]) -> Result<Self> {
        let fields: Vec<&[u8]> = parse::split_spaces(line);
        if fields.len() < 5 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<maps>"),
                line: 0,
                msg: "not enough fields",
            });
        }

        let address = parse_address_range(fields[0])?;
        let perms = parse_permissions(fields[1])?;
        let offset = parse::parse_hex_u64(fields[2])?;
        let device = parse_device(fields[3])?;
        let inode = parse::parse_dec_u64(fields[4])?;
        let pathname = if fields.len() > 5 {
            parse_pathname(&fields[5..])
        } else {
            MapPathname::Anonymous
        };

        Ok(MemoryMap {
            address,
            perms,
            offset,
            device,
            inode,
            pathname,
        })
    }
}

impl MemoryMapDetail {
    /// Parses a full `/proc/PID/smaps` file.
    ///
    /// Each region begins with a header line (same format as
    /// `/proc/PID/maps`) followed by indented key-value lines.
    /// A new header line terminates the previous region.
    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>> {
        let mut details = Vec::new();
        let mut current: Option<MemoryMapDetail> = None;

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            // Header lines start with a hex address. We detect them
            // by checking if the first character is a hex digit.
            if line.first().is_some_and(|b| b.is_ascii_hexdigit()) && line.contains(&b'-') {
                if let Some(detail) = current.take() {
                    details.push(detail);
                }
                current = Some(MemoryMapDetail::from_map_line(line)?);
            } else if let Some(ref mut detail) = current {
                detail.parse_detail_line(line);
            }
        }

        if let Some(detail) = current {
            details.push(detail);
        }

        Ok(details)
    }

    fn from_map_line(line: &[u8]) -> Result<Self> {
        let map = MemoryMap::parse_line(line)?;
        Ok(MemoryMapDetail {
            address: map.address,
            perms: map.perms,
            offset: map.offset,
            device: map.device,
            inode: map.inode,
            pathname: map.pathname,
            size_kb: 0,
            rss_kb: 0,
            pss_kb: 0,
            shared_clean_kb: 0,
            shared_dirty_kb: 0,
            private_clean_kb: 0,
            private_dirty_kb: 0,
            referenced_kb: 0,
            anonymous_kb: 0,
            swap_kb: 0,
        })
    }

    fn parse_detail_line(&mut self, line: &[u8]) {
        let (key, value) = match parse::parse_key_value_line(line) {
            Some(kv) => kv,
            None => return,
        };

        let val = parse::trim_start(value)
            .split(|&b| b == b' ' || b == b'\t')
            .next()
            .and_then(|t| parse::parse_dec_u64(t).ok())
            .unwrap_or(0);

        match key {
            b"Size" => self.size_kb = val,
            b"Rss" => self.rss_kb = val,
            b"Pss" => self.pss_kb = val,
            b"Shared_Clean" => self.shared_clean_kb = val,
            b"Shared_Dirty" => self.shared_dirty_kb = val,
            b"Private_Clean" => self.private_clean_kb = val,
            b"Private_Dirty" => self.private_dirty_kb = val,
            b"Referenced" => self.referenced_kb = val,
            b"Anonymous" => self.anonymous_kb = val,
            b"Swap" => self.swap_kb = val,
            _ => {}
        }
    }
}

/// Parses a hex address range like `55a1b2c3d000-55a1b2c3e000`.
fn parse_address_range(s: &[u8]) -> Result<Range<u64>> {
    let dash = parse::memchr(b'-', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<maps>"),
        line: 0,
        msg: "missing dash in address range",
    })?;

    let start = parse::parse_hex_u64(&s[..dash])?;
    let end = parse::parse_hex_u64(&s[dash + 1..])?;

    Ok(start..end)
}

/// Parses a 4-character permission string like `rwxp`.
fn parse_permissions(s: &[u8]) -> Result<MapPermissions> {
    if s.len() < 4 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<maps>"),
            line: 0,
            msg: "permission string too short",
        });
    }

    let mut perms = MapPermissions::empty();

    if s[0] == b'r' {
        perms |= MapPermissions::READ;
    }
    if s[1] == b'w' {
        perms |= MapPermissions::WRITE;
    }
    if s[2] == b'x' {
        perms |= MapPermissions::EXEC;
    }
    if s[3] == b's' {
        perms |= MapPermissions::SHARED;
    } else if s[3] == b'p' {
        perms |= MapPermissions::PRIVATE;
    }

    Ok(perms)
}

/// Parses a device string like `103:02` (hexadecimal major:minor).
///
/// The device field in `/proc/PID/maps` is formatted in base-16, not
/// decimal (e.g. `00:00`, `00:2c`, `103:02`, `fd:01`), so both the
/// major and minor numbers must be parsed as hex.
fn parse_device(s: &[u8]) -> Result<(u32, u32)> {
    let colon = parse::memchr(b':', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<maps>"),
        line: 0,
        msg: "missing colon in device",
    })?;

    let major = parse::parse_hex_u64(&s[..colon])? as u32;
    let minor = parse::parse_hex_u64(&s[colon + 1..])? as u32;

    Ok((major, minor))
}

/// Classifies a pathname token from `/proc/PID/maps`.
///
/// Known special names (`[heap]`, `[stack]`, `[vdso]`, etc.) are
/// mapped to their enum variants. Everything else is treated as a
/// file path.
fn parse_pathname(fields: &[&[u8]]) -> MapPathname {
    let path = std::str::from_utf8(fields[0]).unwrap_or("");

    match path {
        "[heap]" => MapPathname::Heap,
        "[stack]" => MapPathname::Stack,
        "[vdso]" => MapPathname::Vdso,
        "[vsyscall]" => MapPathname::Vsyscall,
        "[vvar]" => MapPathname::Vvar,
        _ => MapPathname::Path(std::path::PathBuf::from(path)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn device_is_parsed_as_hex() {
        assert_eq!(parse_device(b"103:02").unwrap(), (0x103, 0x2));
        assert_eq!(parse_device(b"00:00").unwrap(), (0x0, 0x0));
        assert_eq!(parse_device(b"00:2c").unwrap(), (0x0, 0x2c));
        assert_eq!(parse_device(b"fd:01").unwrap(), (0xfd, 0x1));
    }

    #[test]
    fn maps_line_with_hex_device_parses() {
        let line = b"7f0000000000-7f0000002000 r-xp 00001000 00:2c 396620 /usr/lib/libfoo.so";
        let map = MemoryMap::parse_line(line).unwrap();
        assert_eq!(map.device, (0x0, 0x2c));
        assert_eq!(map.inode, 396620);
    }
}
