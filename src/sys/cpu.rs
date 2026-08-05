use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::util::parse;

/// CPU frequency information from `/sys/devices/system/cpu/cpu<N>/cpufreq/`.
///
/// All frequency values are in kilohertz. The `governor` field
/// indicates the active CPU frequency scaling policy.
#[derive(Debug)]
pub struct CpuFreqInfo {
    pub cpu: u32,
    /// Current operating frequency.
    pub current_khz: u32,
    /// Minimum allowed frequency.
    pub min_khz: u32,
    /// Maximum allowed frequency.
    pub max_khz: u32,
    /// Active frequency governor (e.g. `powersave`, `performance`, `schedutil`).
    pub governor: Box<str>,
}

/// Reads `/sys/devices/system/cpu/online` and returns the total
/// number of logical CPUs.
///
/// This is the most reliable way to get the CPU count, as it
/// accounts for hot-plugged and offline CPUs.
pub fn cpu_count() -> Result<u32> {
    let cpus = online_cpus()?;
    Ok(cpus.len() as u32)
}

/// Reads `/sys/devices/system/cpu/online` and returns a sorted
/// vector of online CPU IDs.
///
/// The file uses a compact range format: `0-3,6` means CPUs
/// 0, 1, 2, 3, and 6 are online.
pub fn online_cpus() -> Result<Vec<u32>> {
    let path = PathBuf::from("/sys/devices/system/cpu/online");
    let bytes = parse::read_file(&path)?;
    let s = std::str::from_utf8(&bytes).unwrap_or("").trim();

    if s.is_empty() {
        return Ok(vec![0]);
    }

    let mut cpus = Vec::new();

    for segment in s.split(',') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        if let Some(dash) = segment.find('-') {
            let start = segment[..dash].parse::<u32>().map_err(|_| Error::Parse {
                path: path.clone(),
                line: 0,
                msg: "invalid range start",
            })?;
            let end = segment[dash + 1..]
                .parse::<u32>()
                .map_err(|_| Error::Parse {
                    path: path.clone(),
                    line: 0,
                    msg: "invalid range end",
                })?;
            for cpu in start..=end {
                cpus.push(cpu);
            }
        } else {
            let cpu = segment.parse::<u32>().map_err(|_| Error::Parse {
                path: path.clone(),
                line: 0,
                msg: "invalid cpu number",
            })?;
            cpus.push(cpu);
        }
    }

    cpus.sort();
    cpus.dedup();
    Ok(cpus)
}

/// Reads frequency information for a specific CPU from
/// `/sys/devices/system/cpu/cpu<N>/cpufreq/`.
///
/// Returns `Error::Io` if the cpufreq directory doesn't exist
/// for this CPU (e.g. the CPU doesn't support frequency scaling).
pub fn cpu_freq(cpu: u32) -> Result<CpuFreqInfo> {
    let base = PathBuf::from(format!("/sys/devices/system/cpu/cpu{}/cpufreq", cpu));

    let current_khz = {
        let path = base.join("scaling_cur_freq");
        match parse::read_file(&path) {
            Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(0),
            Err(_) => 0,
        }
    };

    let min_khz = {
        let path = base.join("scaling_min_freq");
        match parse::read_file(&path) {
            Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(0),
            Err(_) => 0,
        }
    };

    let max_khz = {
        let path = base.join("scaling_max_freq");
        match parse::read_file(&path) {
            Ok(bytes) => parse::parse_dec_u32(&bytes).unwrap_or(0),
            Err(_) => 0,
        }
    };

    let governor = {
        let path = base.join("scaling_governor");
        match parse::read_file(&path) {
            Ok(bytes) => std::str::from_utf8(&bytes)
                .unwrap_or("")
                .trim()
                .to_string()
                .into_boxed_str(),
            Err(_) => Box::from(""),
        }
    };

    Ok(CpuFreqInfo {
        cpu,
        current_khz,
        min_khz,
        max_khz,
        governor,
    })
}
