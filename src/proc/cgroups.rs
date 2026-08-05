use crate::error::{Error, Result};
use crate::util::parse;

/// System-wide cgroup hierarchy information from `/proc/cgroups`.
///
/// Lists each registered cgroup controller (subsystem) with its
/// hierarchy ID, number of cgroups, and enabled status.
#[derive(Debug)]
pub struct CgroupStat {
    /// Controller name (e.g. `cpu`, `memory`, `pids`).
    pub subsys_name: Box<str>,
    /// Hierarchy ID. Multiple controllers may share the same ID
    /// if they are bound to the same hierarchy.
    pub hierarchy: u32,
    /// Number of cgroups in this hierarchy.
    pub num_cgroups: u32,
    /// Whether the controller is currently enabled.
    pub enabled: bool,
}

/// Reads `/proc/cgroups` and returns all registered cgroup controllers.
///
/// The file has a one-line header followed by one line per controller:
/// ```text
/// #subsys_name    hierarchy       num_cgroups     enabled
/// cpuset  2       1       1
/// cpu     3       42      1
/// memory  4       42      1
/// ```
pub fn cgroups() -> Result<Vec<CgroupStat>> {
    let path = "/proc/cgroups";
    let bytes = parse::read_file(std::path::Path::new(path))?;

    let mut entries = Vec::new();

    for line in bytes.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        // Skip the header line starting with '#'.
        if line.starts_with(b"#") {
            continue;
        }

        let fields: Vec<&[u8]> = parse::split_spaces(line);
        if fields.len() < 4 {
            return Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            });
        }

        let subsys_name = std::str::from_utf8(fields[0])
            .unwrap_or("")
            .to_string()
            .into_boxed_str();

        let hierarchy = parse::parse_dec_u32(fields[1]).unwrap_or(0);
        let num_cgroups = parse::parse_dec_u32(fields[2]).unwrap_or(0);
        let enabled = parse::parse_dec_u32(fields[3]).unwrap_or(0) != 0;

        entries.push(CgroupStat {
            subsys_name,
            hierarchy,
            num_cgroups,
            enabled,
        });
    }

    Ok(entries)
}
