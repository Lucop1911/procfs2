use crate::error::{Error, Result};
use crate::util::parse;

/// A single cgroup membership entry from `/proc/PID/cgroup`.
///
/// Each line describes one cgroup hierarchy the process belongs to:
/// ```text
/// 0::/
/// 1:name=systemd:/user.slice/user-1000.slice/session-1.scope
/// ```
///
/// With cgroup v2, there is typically a single entry with hierarchy
/// ID 0 and an empty controller list.
#[derive(Debug)]
pub struct CgroupEntry {
    /// Hierarchy ID. For cgroup v2 unified hierarchy this is always 0.
    pub hierarchy: u32,
    /// Comma-separated list of controllers bound to this hierarchy.
    ///
    /// Empty for the cgroup v2 unified hierarchy (controllers are
    /// enabled per-cgroup, not per-hierarchy).
    pub controllers: Vec<Box<str>>,
    /// Cgroup path within the hierarchy.
    pub path: std::path::PathBuf,
}

impl CgroupEntry {
    /// Parses all lines of a `/proc/PID/cgroup` file.
    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>> {
        let mut entries = Vec::new();

        for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
            entries.push(Self::parse_line(line)?);
        }

        Ok(entries)
    }

    fn parse_line(line: &[u8]) -> Result<Self> {
        // /proc/PID/cgroup uses colon-separated fields:
        // hierarchy:controllers:path
        let fields: Vec<&[u8]> = line.splitn(3, |&b| b == b':').collect();
        if fields.len() < 3 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from("<cgroup>"),
                line: 0,
                msg: "not enough fields",
            });
        }

        let hierarchy = parse::parse_dec_u32(fields[0])?;

        // Controller list is comma-separated. An empty string means
        // no controllers (cgroup v2 unified hierarchy).
        let controllers_raw = fields[1];
        let controllers: Vec<Box<str>> = if controllers_raw.is_empty() {
            Vec::new()
        } else {
            controllers_raw
                .split(|&b| b == b',')
                .filter(|c| !c.is_empty())
                .map(|c| {
                    std::str::from_utf8(c)
                        .unwrap_or("")
                        .to_string()
                        .into_boxed_str()
                })
                .collect()
        };

        let path =
            std::path::PathBuf::from(std::str::from_utf8(fields[2]).unwrap_or("/").to_string());

        Ok(CgroupEntry {
            hierarchy,
            controllers,
            path,
        })
    }
}
