use crate::error::{Error, Result};
use crate::util::parse;

/// A single mount entry from `/proc/mounts`.
///
/// This is the simpler mount format (equivalent to `/proc/self/mounts`),
/// containing device, mount point, filesystem type, and options.
/// For richer data including mount IDs and optional fields, see
/// [`Process::mountinfo`](crate::proc::Process::mountinfo).
#[derive(Debug)]
pub struct MountEntry {
    /// Device special file or pseudo-device name (e.g. `/dev/sda1`, `proc`, `tmpfs`).
    pub spec: Box<str>,
    /// Mount point in the filesystem tree.
    pub file: Box<str>,
    /// Filesystem type (e.g. `ext4`, `tmpfs`, `proc`).
    pub vfstype: Box<str>,
    /// Mount options as a comma-separated string (e.g. `rw,noatime`).
    pub mntops: Box<str>,
    /// Dump frequency (from `/etc/fstab`'s `freq` field).
    ///
    /// Always 0 for pseudo-filesystems.
    pub freq: i32,
    /// fsck pass number (from `/etc/fstab`'s `passno` field).
    ///
    /// 0 means "don't check", 1 is reserved for the root filesystem.
    pub passno: i32,
}

/// Reads `/proc/mounts` and returns all mounted filesystems.
///
/// The format is identical to `/proc/self/mounts` — one line per
/// mount with six space-separated fields. Backslash-escaped
/// characters (spaces, tabs, newlines, backslashes) in paths are
/// decoded.
pub fn mounts() -> Result<Vec<MountEntry>> {
    let path = "/proc/mounts";
    let bytes = parse::read_file(std::path::Path::new(path))?;

    let mut entries = Vec::new();

    for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        let fields: Vec<&[u8]> = parse::split_spaces(line);
        if fields.len() < 6 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            });
        }

        entries.push(MountEntry {
            spec: decode_escaped_path(fields[0]),
            file: decode_escaped_path(fields[1]),
            vfstype: bytes_to_box_str(fields[2]),
            mntops: bytes_to_box_str(fields[3]),
            freq: parse::parse_dec_i64(fields[4]).unwrap_or(0) as i32,
            passno: parse::parse_dec_i64(fields[5]).unwrap_or(0) as i32,
        });
    }

    Ok(entries)
}

/// Decodes backslash-escaped octal sequences in a path.
///
/// `/proc/mounts` escapes spaces (`\040`), tabs (`\011`), newlines
/// (`\012`), and backslashes (`\134`) using octal notation. This
/// function converts them back to their literal characters.
fn decode_escaped_path(s: &[u8]) -> Box<str> {
    let mut result = Vec::with_capacity(s.len());
    let mut i = 0;

    while i < s.len() {
        if s[i] == b'\\' && i + 3 < s.len() {
            if let Ok(val) =
                u8::from_str_radix(std::str::from_utf8(&s[i + 1..i + 4]).unwrap_or("000"), 8)
            {
                result.push(val);
                i += 4;
                continue;
            }
        }
        result.push(s[i]);
        i += 1;
    }

    std::str::from_utf8(&result)
        .unwrap_or("")
        .to_string()
        .into_boxed_str()
}

fn bytes_to_box_str(b: &[u8]) -> Box<str> {
    std::str::from_utf8(b)
        .unwrap_or("")
        .to_string()
        .into_boxed_str()
}
