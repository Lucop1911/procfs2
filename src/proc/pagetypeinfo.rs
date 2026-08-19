use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Buddy free pages grouped by migrate type, from `/proc/pagetypeinfo`.
///
/// `/proc/buddyinfo` aggregates free chunks into per-zone lists; this
/// file breaks those same lists down by migrate type, which is how the
/// allocator pages are grouped for memory mobility (e.g. `Movable` vs
/// `Unmovable`). Values are only meaningful as a snapshot — the kernel
/// briefly releases the zone lock between orders while printing, so a
/// single row can be momentarily inconsistent under allocation pressure.
#[derive(Debug)]
pub struct PageTypeInfo {
    /// Page block size as a power of two (`Page block order`).
    ///
    /// A block covers `1 << page_block_order` pages.
    pub page_block_order: u64,
    /// Number of base pages per page block (`Pages per block`).
    pub pages_per_block: u64,
    /// Free blocks per migrate type and order, one entry per
    /// zone/migrate-type row.
    pub free: Vec<PageTypeFree>,
    /// Page-block totals per migrate type, one entry per zone row.
    pub blocks: Vec<PageTypeBlock>,
}

/// Free blocks of one migrate type within a zone.
///
/// One row of the `Free pages count per migrate type at order` table:
/// `Node <id>, zone <name>, type <migrate type>` followed by the
/// per-order free-list counts.
#[derive(Debug)]
pub struct PageTypeFree {
    /// NUMA node ID.
    pub node: u32,
    /// Memory zone name.
    pub zone: Box<str>,
    /// Migrate type: how the zone's pages are grouped for mobility.
    ///
    /// Examples are `Unmovable`, `Movable`, `Reclaimable` and `CMA`;
    /// the set varies by kernel configuration.
    pub migrate_type: Box<str>,
    /// Free blocks of buddy order `i`; a block covers `2^i` pages.
    ///
    /// Index = order. The number of orders is `MAX_ORDER`, which varies
    /// across kernels, so the length is taken from the row itself.
    pub orders: Vec<u64>,
}

/// Page-block counts of each migrate type within a zone.
///
/// One row of the `Number of blocks type` table: `Node <id>, zone
/// <name>` followed by one count per migrate type.
#[derive(Debug)]
pub struct PageTypeBlock {
    /// NUMA node ID.
    pub node: u32,
    /// Memory zone name.
    pub zone: Box<str>,
    /// Page blocks of each migrate type, aligned with the table
    /// header's column order (which varies by kernel configuration).
    pub types: Vec<u64>,
}

/// Reads `/proc/pagetypeinfo` and returns the free-page breakdown by
/// migrate type.
///
/// Each NUMA node prints its own copy of the block-size header followed
/// by two tables: `Free pages count per migrate type at order` (one row
/// per zone/migrate-type combination) and `Number of blocks type` (one
/// row per zone). The block-size values are global, so only the first
/// copy is kept and later copies are ignored. A third table, `Number of
/// mixed blocks`, appears only on kernels built with `CONFIG_PAGE_OWNER`
/// and is skipped.
///
/// The column counts are kernel-dependent — the number of order columns
/// is `MAX_ORDER` and the number of migrate-type columns depends on the
/// kernel configuration — so rows are stored in [`Vec`]s sized from the
/// row itself rather than hardcoded. Free-list counts are capped at
/// 100000 by the kernel, which prints a leading `>` in that case; the
/// capped value is kept as-is.
///
/// The file is usually world-readable, but may be restricted to root
/// (mode 0400) on some systems; that surfaces as [`Error::Io`] with a
/// permission-denied kind.
pub fn pagetypeinfo() -> Result<PageTypeInfo> {
    let path = Path::new("/proc/pagetypeinfo");
    let bytes = parse::read_file(path)?;

    let mut free: Vec<PageTypeFree> = Vec::new();
    let mut blocks: Vec<PageTypeBlock> = Vec::new();
    let mut page_block_order = 0u64;
    let mut pages_per_block = 0u64;

    enum Section {
        Free,
        Blocks,
    }
    let mut section = None;

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        if let Some((key, value)) = parse::parse_key_value_line(line) {
            match key {
                b"Page block order" => {
                    page_block_order = parse::parse_dec_u64(value).map_err(|_| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid page block order",
                    })?;
                    section = None;
                    continue;
                }
                b"Pages per block" => {
                    pages_per_block = parse::parse_dec_u64(value).map_err(|_| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid pages per block",
                    })?;
                }
                _ => {}
            }
        }
        if line.starts_with(b"Free pages count per migrate type at order") {
            section = Some(Section::Free);
            continue;
        }
        if line.starts_with(b"Number of blocks type") {
            section = Some(Section::Blocks);
            continue;
        }
        if line.starts_with(b"Number of mixed blocks") {
            section = None;
            continue;
        }

        if !line.starts_with(b"Node ") {
            continue;
        }

        let fields = parse::SplitFields::<32>::new(line);

        match section {
            Some(Section::Free) => {
                if fields.len() < 7 {
                    return Err(Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "expected 'Node <id>, zone <name>, type <name> <counts>'",
                    });
                }
                let (node, _) = parse::split_at_byte(fields[1], b',');
                let node = parse::parse_dec_u32(node).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid node",
                })?;
                let (zone, _) = parse::split_at_byte(fields[3], b',');
                let zone = std::str::from_utf8(zone).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid zone",
                })?;
                let migrate_type = std::str::from_utf8(fields[5]).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid migrate type",
                })?;
                let mut orders = Vec::with_capacity(fields.len() - 6);
                for field in &fields[6..] {
                    let field = if field.first() == Some(&b'>') {
                        &field[1..]
                    } else {
                        field
                    };
                    orders.push(parse::parse_dec_u64(field).map_err(|_| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid order count",
                    })?);
                }
                free.push(PageTypeFree {
                    node,
                    zone: zone.into(),
                    migrate_type: migrate_type.into(),
                    orders,
                });
            }
            Some(Section::Blocks) => {
                if fields.len() < 5 {
                    return Err(Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "expected 'Node <id>, zone <name> <counts>'",
                    });
                }
                let (node, _) = parse::split_at_byte(fields[1], b',');
                let node = parse::parse_dec_u32(node).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid node",
                })?;
                let (zone, _) = parse::split_at_byte(fields[3], b',');
                let zone = std::str::from_utf8(zone).map_err(|_| Error::Parse {
                    path: path.to_path_buf(),
                    line: line_num + 1,
                    msg: "invalid zone",
                })?;
                let mut types = Vec::with_capacity(fields.len() - 4);
                for field in &fields[4..] {
                    types.push(parse::parse_dec_u64(field).map_err(|_| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: "invalid block count",
                    })?);
                }
                blocks.push(PageTypeBlock {
                    node,
                    zone: zone.into(),
                    types,
                });
            }
            None => {}
        }
    }

    Ok(PageTypeInfo {
        page_block_order,
        pages_per_block,
        free,
        blocks,
    })
}
