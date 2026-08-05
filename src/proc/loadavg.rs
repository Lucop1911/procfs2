use crate::error::{Error, Result};
use crate::util::parse;

/// System load averages.
///
/// Sourced from `/proc/loadavg`. The three load average values are
/// exponentially-damped moving averages over 1, 5, and 15 minute
/// intervals.
#[derive(Debug)]
pub struct LoadAvg {
    /// 1-minute load average.
    pub one: f64,
    /// 5-minute load average.
    pub five: f64,
    /// 15-minute load average.
    pub fifteen: f64,
    /// Number of currently runnable kernel scheduling entities.
    pub runnable: u32,
    /// Total number of kernel scheduling entities.
    pub total: u32,
}

/// Reads `/proc/loadavg` and returns [`LoadAvg`].
///
/// The raw format is: `one five fifteen runnable/total last_pid`
pub fn loadavg() -> Result<LoadAvg> {
    let bytes = parse::read_file(std::path::Path::new("/proc/loadavg"))?;
    let fields = parse::split_spaces(&bytes);

    if fields.len() < 4 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("/proc/loadavg"),
            line: 1,
            msg: "expected at least 4 fields",
        });
    }

    let one = parse::parse_dec_f64(fields[0])?;
    let five = parse::parse_dec_f64(fields[1])?;
    let fifteen = parse::parse_dec_f64(fields[2])?;

    let sched = fields[3];
    let slash_idx = parse::memchr(b'/', sched).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("/proc/loadavg"),
        line: 1,
        msg: "missing slash in sched field",
    })?;

    let runnable = parse::parse_dec_u32(&sched[..slash_idx])?;
    let total = parse::parse_dec_u32(&sched[slash_idx + 1..])?;

    Ok(LoadAvg {
        one,
        five,
        fifteen,
        runnable,
        total,
    })
}
