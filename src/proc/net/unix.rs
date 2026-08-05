use crate::error::{Error, Result};
use crate::util::parse;

/// A single Unix domain socket entry from `/proc/net/unix`.
///
/// Unix sockets use a different format than TCP/UDP — a space-
/// separated table with numeric flags and an optional pathname.
#[derive(Debug)]
pub struct UnixEntry {
    /// Kernel object number (unique identifier).
    pub ino: u64,
    /// Number of references to this socket.
    pub ref_count: u32,
    /// Protocol (always 0 for Unix sockets).
    pub protocol: u32,
    /// Socket type: `SOCK_STREAM` (1), `SOCK_DGRAM` (2), etc.
    pub type_: u32,
    /// Socket state: `0` = unconnected, `1` = connected.
    pub state: u32,
    /// Inode number in the filesystem namespace.
    pub inode: u64,
    /// Bound pathname, if any.
    ///
    /// Abstract sockets (starting with a null byte) are not
    /// included here since they have no filesystem representation.
    pub path: Option<Box<str>>,
}

/// Reads `/proc/net/unix` and returns an iterator over all Unix
/// domain sockets.
///
/// Each line has 7 space-separated fields (the 8th, `Path`, is optional):
/// ```text
/// 0000000000000000 00000002 00000000 00010000 0001 00000 12345 /var/run/dbus/system_bus_socket
/// ^Num             ^RefCnt  ^Proto   ^Type    ^St  ^Inode ^Path
/// ```
/// All numeric fields except `Inode` are hexadecimal. `Inode` is decimal.
pub fn unix() -> impl Iterator<Item = Result<UnixEntry>> {
    let path = "/proc/net/unix";
    let bytes = match parse::read_file(std::path::Path::new(path)) {
        Ok(b) => b,
        Err(e) => return vec![Err(e)].into_iter(),
    };

    let lines: Vec<&[u8]> = bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
        .collect();

    let mut entries = Vec::with_capacity(lines.len());

    for line in lines {
        let fields: Vec<&[u8]> = parse::split_spaces(line);

        // /proc/net/unix has variable field counts. Minimum is 7
        // (without path), 8 with path.
        if fields.len() < 7 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        // Fields: Num, RefCount, Protocol, Type, State, Inode, [Path]
        // Some kernels have an extra flags field between Type and State.
        // We locate fields by position from the end.
        let inode = parse::parse_dec_u64(fields[fields.len() - 2]).unwrap_or(0);
        let state = parse::parse_hex_u64(fields[fields.len() - 3]).unwrap_or(0) as u32;
        let type_ = parse::parse_hex_u64(fields[fields.len() - 4]).unwrap_or(0) as u32;
        let protocol = parse::parse_hex_u64(fields[fields.len() - 5]).unwrap_or(0) as u32;
        let ref_count = parse::parse_hex_u64(fields[fields.len() - 6]).unwrap_or(0) as u32;
        let ino = parse::parse_hex_u64(fields[0]).unwrap_or(0);

        let path_str = if fields.len() > 7 {
            std::str::from_utf8(fields[fields.len() - 1])
                .ok()
                .map(|s| s.to_string().into_boxed_str())
        } else {
            None
        };

        entries.push(Ok(UnixEntry {
            ino,
            ref_count,
            protocol,
            type_,
            state,
            inode,
            path: path_str,
        }));
    }

    entries.into_iter()
}
