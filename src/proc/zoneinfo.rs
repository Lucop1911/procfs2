use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Watermark and size information for one memory zone.
///
/// Values are in units of memory pages. Fields absent on a given
/// kernel default to zero (e.g. `promo` predates Linux 6.4).
#[derive(Debug, Default)]
pub struct ZonePages {
    /// Number of free pages in the zone.
    pub free: u64,
    /// Watermark boost applied under memory pressure.
    pub boost: u64,
    /// `min` watermark; below this, allocations can stall.
    pub min: u64,
    /// `low` watermark; below this, kswapd starts reclaiming.
    pub low: u64,
    /// `high` watermark; above this, kswapd stops reclaiming.
    pub high: u64,
    /// `promo` watermark (Linux 6.4+).
    pub promo: u64,
    /// Pages spanned by the zone (may exceed physical memory).
    pub spanned: u64,
    /// Pages actually present in the zone.
    pub present: u64,
    /// Pages managed by the buddy allocator (excludes bootmem).
    pub managed: u64,
    /// Pages managed by the zone that belong to CMA.
    pub cma: u64,
}

/// Per-CPU per-zone page list (`PCP`) state.
#[derive(Debug, Default)]
pub struct Pcp {
    /// CPU number this pageset belongs to.
    pub cpu: u32,
    /// Number of pages currently on the list.
    pub count: u64,
    /// Target maximum number of pages on the list.
    pub high: u64,
    /// Number of pages transferred to/from the zone at once.
    pub batch: u64,
    /// Lower bound for `high` (Linux 6.5+).
    pub high_min: u64,
    /// Upper bound for `high` (Linux 6.5+).
    pub high_max: u64,
}

/// Information about one memory zone from `/proc/zoneinfo`.
///
/// Zones with no managed pages (e.g. `Movable`/`Device` when empty)
/// are still returned, but their counters, pagesets and node-tail
/// fields default to empty/zero.
#[derive(Debug, Default)]
pub struct ZoneInfo {
    /// NUMA node the zone belongs to.
    pub node: u32,
    /// Zone name (e.g. `DMA`, `DMA32`, `Normal`).
    pub name: Box<str>,
    /// Node-wide virtual memory counters.
    ///
    /// The kernel prints these once per node (under the node's first
    /// zone); only that `ZoneInfo` is populated with them.
    pub per_node_stats: HashMap<String, u64>,
    /// Watermark and size information for the zone.
    pub pages: ZonePages,
    /// Per-zone `lowmem_reserve` values, one per zone type.
    pub protection: Vec<u64>,
    /// Per-zone virtual memory counters (e.g. `nr_free_pages`,
    /// `nr_zone_*`, `numa_*`). Keys vary from kernel to kernel, so a
    /// `HashMap` is used rather than a fixed struct.
    pub stats: HashMap<String, u64>,
    /// Per-CPU page lists.
    pub pagesets: Vec<Pcp>,
    /// Threshold at which per-CPU vmstat counters are flushed.
    pub vm_stats_threshold: u64,
    /// Whether the node is considered unreclaimable.
    pub node_unreclaimable: u64,
    /// First page frame number of the zone.
    pub start_pfn: u64,
    /// Pages reserved for high-atomic allocations.
    pub reserved_highatomic: u64,
    /// Free pages reserved for high-atomic allocations.
    pub free_highatomic: u64,
}

/// Reads `/proc/zoneinfo` and returns one [`ZoneInfo`] per zone.
///
/// The file groups data into indented sections. Indentation is
/// informational only; this parser dispatches on the leading token of
/// each line, which makes it robust to whitespace changes.
pub fn zoneinfo() -> Result<Vec<ZoneInfo>> {
    let path = Path::new("/proc/zoneinfo");
    let bytes = parse::read_file(path)?;

    let mut out: Vec<ZoneInfo> = Vec::new();
    let mut current: Option<ZoneInfo> = None;
    let mut in_per_node = false;
    let mut current_pcp: Option<Pcp> = None;

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let fields = parse::SplitFields::<8>::new(line);
        let first = match fields.first() {
            Some(&f) => f,
            None => continue,
        };

        match first {
            b"Node" => {
                if let Some(zone) = current.as_mut() {
                    if let Some(pcp) = current_pcp.take() {
                        zone.pagesets.push(pcp);
                    }
                }
                if let Some(zone) = current.take() {
                    out.push(zone);
                }
                in_per_node = false;

                let node_str = fields[1].strip_suffix(b",").unwrap_or(fields[1]);
                let node = parse::parse_dec_u32(node_str).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid node",
                })?;

                let name = std::str::from_utf8(fields[3])
                    .map_err(|_| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid zone name",
                    })?
                    .into();

                current = Some(ZoneInfo {
                    node,
                    name,
                    ..Default::default()
                });
            }
            b"per-node" => {
                in_per_node = true;
            }
            b"pages" => {
                in_per_node = false;
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.pages.free = parse_u64(fields[2], path, line_num, "invalid free pages")?;
            }
            b"protection:" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;

                let start = parse::memchr(b'(', line).ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "malformed protection",
                })?;
                let end = parse::memchr(b')', line).ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "malformed protection",
                })?;

                let inner = &line[start + 1..end];
                zone.protection = inner
                    .split(|&b| b == b',')
                    .map(|part| {
                        parse_u64(
                            parse::trim(part),
                            path,
                            line_num,
                            "invalid protection value",
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
            }
            b"pagesets" => {}
            b"cpu:" => {
                if let Some(pcp) = current_pcp.take() {
                    let zone = current.as_mut().ok_or_else(|| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "expected '<Node> <id>, zone <name>'",
                    })?;
                    zone.pagesets.push(pcp);
                }
                let cpu = parse::parse_dec_u32(fields[1]).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid cpu",
                })?;
                current_pcp = Some(Pcp {
                    cpu,
                    ..Default::default()
                });
            }
            b"vm" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.vm_stats_threshold =
                    parse_u64(fields[3], path, line_num, "invalid vm stats threshold")?;
            }
            b"node_unreclaimable:" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.node_unreclaimable =
                    parse_u64(fields[1], path, line_num, "invalid node_unreclaimable")?;
            }
            b"start_pfn:" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.start_pfn = parse_u64(fields[1], path, line_num, "invalid start_pfn")?;
            }
            b"reserved_highatomic:" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.reserved_highatomic =
                    parse_u64(fields[1], path, line_num, "invalid reserved_highatomic")?;
            }
            b"free_highatomic:" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                zone.free_highatomic =
                    parse_u64(fields[1], path, line_num, "invalid free_highatomic")?;
            }
            b"boost" | b"min" | b"low" | b"high" | b"promo" | b"spanned" | b"present"
            | b"managed" | b"cma" => {
                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                let value = parse_u64(fields[1], path, line_num, "invalid page count")?;
                match first {
                    b"boost" => zone.pages.boost = value,
                    b"min" => zone.pages.min = value,
                    b"low" => zone.pages.low = value,
                    b"high" => zone.pages.high = value,
                    b"promo" => zone.pages.promo = value,
                    b"spanned" => zone.pages.spanned = value,
                    b"present" => zone.pages.present = value,
                    b"managed" => zone.pages.managed = value,
                    b"cma" => zone.pages.cma = value,
                    _ => unreachable!(),
                }
            }
            b"count:" | b"high:" | b"batch:" | b"high_min:" | b"high_max:" => {
                let pcp = current_pcp.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected 'cpu: <n>'",
                })?;
                let value = parse_u64(fields[1], path, line_num, "invalid pageset value")?;
                match first {
                    b"count:" => pcp.count = value,
                    b"high:" => pcp.high = value,
                    b"batch:" => pcp.batch = value,
                    b"high_min:" => pcp.high_min = value,
                    b"high_max:" => pcp.high_max = value,
                    _ => unreachable!(),
                }
            }
            _ => {
                let key = std::str::from_utf8(first).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid key",
                })?;
                let value = parse_u64(fields[1], path, line_num, "invalid value")?;

                let zone = current.as_mut().ok_or_else(|| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "expected '<Node> <id>, zone <name>'",
                })?;
                if in_per_node {
                    zone.per_node_stats.insert(key.into(), value);
                } else {
                    zone.stats.insert(key.into(), value);
                }
            }
        }
    }

    if let Some(mut zone) = current.take() {
        if let Some(pcp) = current_pcp.take() {
            zone.pagesets.push(pcp);
        }
        out.push(zone);
    }

    Ok(out)
}

/// Parses a decimal `u64` field, wrapping failures in [`Error::Parse`].
fn parse_u64(field: &[u8], path: &Path, line_num: usize, msg: &'static str) -> Result<u64> {
    parse::parse_dec_u64(field).map_err(move |_| Error::Parse {
        path: path.to_path_buf(),
        line: line_num + 1,
        msg,
    })
}
