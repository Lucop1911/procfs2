use crate::error::{Error, Result};
use crate::util::parse;

/// A single mount entry from `/proc/PID/mountinfo`.
///
/// This format is richer than `/proc/mounts` (which is equivalent to
/// `/proc/self/mounts`) because it includes mount IDs, parent IDs,
/// optional fields, and separate superblock options.
///
/// Each line has the format:
/// ```text
/// 36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw,errors=continue
/// ```
/// Fields: id, parent_id, major:minor, root, mount_point, options,
/// optional_fields, `-`, fs_type, source, super_options.
#[derive(Debug)]
pub struct MountInfo {
    /// Unique mount ID (assigned by the kernel).
    pub id: u64,
    /// Mount ID of the parent mount.
    pub parent_id: u64,
    /// Device major number.
    pub major: u32,
    /// Device minor number.
    pub minor: u32,
    /// Path within the filesystem that is the root of this mount.
    pub root: Box<str>,
    /// Mount point relative to the process's root.
    pub mount_point: Box<str>,
    /// Per-mount options (e.g. `rw,noatime`).
    pub options: Box<str>,
    /// Optional fields that appear before the `-` separator.
    ///
    /// These include shared/peer/slave/master relationships and
    /// propagate information. The format varies by kernel version.
    pub optional_fields: Vec<Box<str>>,
    /// Filesystem type (e.g. `ext4`, `tmpfs`, `proc`).
    pub filesystem_type: Box<str>,
    /// Mount source (e.g. `/dev/sda1`, `none`).
    pub source: Box<str>,
    /// Per-superblock options (e.g. `rw,errors=continue`).
    pub super_options: Box<str>,
}

impl MountInfo {
    /// Parses all lines of a `/proc/PID/mountinfo` file.
    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>> {
        let mut mounts = Vec::new();

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            mounts.push(Self::parse_line(line)?);
        }

        Ok(mounts)
    }

    fn parse_line(line: &[u8]) -> Result<Self> {
        let fields: Vec<&[u8]> = parse::split_spaces(line);
        if fields.len() < 10 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<mountinfo>"),
                line: 0,
                msg: "not enough fields",
            });
        }

        let id = parse::parse_dec_u64(fields[0])?;
        let parent_id = parse::parse_dec_u64(fields[1])?;

        let dev = fields[2];
        let colon = parse::memchr(b':', dev).ok_or_else(|| Error::Parse {
            path: std::path::PathBuf::from("<mountinfo>"),
            line: 0,
            msg: "missing colon in device",
        })?;
        let major = parse::parse_dec_u32(&dev[..colon])?;
        let minor = parse::parse_dec_u32(&dev[colon + 1..])?;

        let root = bytes_to_box_str(fields[3]);
        let mount_point = bytes_to_box_str(fields[4]);
        let options = bytes_to_box_str(fields[5]);

        // The `-` separator marks the boundary between optional
        // fields and the fixed trailing fields (fs_type, source,
        // super_options).
        match fields.iter().position(|f| *f == b"-") {
            Some(sep) => {
                if fields.len() <= sep + 3 {
                    return Err(Error::Parse {
                        path: std::path::PathBuf::from("<mountinfo>"),
                        line: 0,
                        msg: "missing fields after separator",
                    });
                }

                let optional_fields: Vec<Box<str>> =
                    fields[6..sep].iter().map(|f| bytes_to_box_str(f)).collect();

                Ok(MountInfo {
                    id,
                    parent_id,
                    major,
                    minor,
                    root,
                    mount_point,
                    options,
                    optional_fields,
                    filesystem_type: bytes_to_box_str(fields[sep + 1]),
                    source: bytes_to_box_str(fields[sep + 2]),
                    super_options: bytes_to_box_str(fields[sep + 3]),
                })
            }
            None => {
                // No `-` separator means no optional fields and no
                // trailing fs_type/source/super_options. This is
                // unusual but we handle it gracefully.
                let optional_fields: Vec<Box<str>> =
                    fields[6..].iter().map(|f| bytes_to_box_str(f)).collect();

                Ok(MountInfo {
                    id,
                    parent_id,
                    major,
                    minor,
                    root,
                    mount_point,
                    options,
                    optional_fields,
                    filesystem_type: Box::from(""),
                    source: Box::from(""),
                    super_options: Box::from(""),
                })
            }
        }
    }
}

fn bytes_to_box_str(b: &[u8]) -> Box<str> {
    std::str::from_utf8(b)
        .unwrap_or("")
        .to_string()
        .into_boxed_str()
}
