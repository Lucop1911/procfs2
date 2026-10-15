use std::net::Ipv4Addr;
use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

use super::IPV4_HEX_LEN;

/// Columns in a `/proc/net/route` row, header excluded.
const ROUTE_FIELDS: usize = 11;
// Column indices, in the order the header prints them.
const COL_IFACE: usize = 0;
const COL_DESTINATION: usize = 1;
const COL_GATEWAY: usize = 2;
const COL_FLAGS: usize = 3;
const COL_REFCNT: usize = 4;
const COL_USE: usize = 5;
const COL_METRIC: usize = 6;
const COL_MASK: usize = 7;
const COL_MTU: usize = 8;
const COL_WINDOW: usize = 9;
const COL_IRTT: usize = 10;

/// A single routing table entry from `/proc/net/route`.
///
/// The kernel routing table contains one entry per route. The
/// default route has `destination = 0.0.0.0` and `mask = 0.0.0.0`.
#[derive(Debug)]
pub struct RouteEntry {
    /// Network interface this route applies to.
    pub iface: Box<str>,
    /// Destination network address.
    pub destination: Ipv4Addr,
    /// Gateway address (0.0.0.0 for direct routes).
    pub gateway: Ipv4Addr,
    /// Subnet mask.
    pub mask: Ipv4Addr,
    /// Route flags (e.g. `RTF_UP`, `RTF_GATEWAY`, `RTF_HOST`).
    pub flags: u16,
    /// Reference count (number of active uses).
    pub refcnt: u16,
    /// Usage count (packets routed via this entry).
    pub use_: u32,
    /// Metric (cost of this route).
    pub metric: u32,
    /// MTU for this route.
    pub mtu: u32,
    /// Window size for this route.
    pub window: u32,
    /// Initial RTT (round-trip time) estimate.
    pub irtt: u32,
}

/// Reads `/proc/net/route` and returns an iterator over IPv4
/// routing table entries.
///
/// The file format is:
/// ```text
/// Iface   Destination Gateway     Flags   RefCnt  Use Metric  Mask        MTU Window  IRTT
/// eth0    00000000    0100000A    0003    0       0   100     00000000    0   0       0
/// ```
/// Addresses are stored in little-endian hex (same as `/proc/net/tcp`).
pub fn route() -> impl Iterator<Item = Result<RouteEntry>> {
    let path = Path::new("/proc/net/route");
    let bytes = match parse::read_file(path) {
        Ok(b) => b,
        Err(e) => return vec![Err(e)].into_iter(),
    };

    let mut entries = Vec::new();
    for line in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
    {
        let fields = parse::SplitFields::<ROUTE_FIELDS>::new(line);
        if fields.len() < ROUTE_FIELDS {
            entries.push(Err(Error::Parse {
                path: path.to_path_buf(),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let iface = std::str::from_utf8(fields[COL_IFACE])
            .unwrap_or("")
            .to_string()
            .into_boxed_str();

        let destination = parse_hex_ipv4(fields[COL_DESTINATION]);
        let gateway = parse_hex_ipv4(fields[COL_GATEWAY]);
        let mask = parse_hex_ipv4(fields[COL_MASK]);

        let flags = parse::parse_hex_fast(fields[COL_FLAGS]) as u16;
        let refcnt = parse::parse_dec_fast(fields[COL_REFCNT]) as u16;
        let use_ = parse::parse_dec_fast(fields[COL_USE]) as u32;
        let metric = parse::parse_dec_fast(fields[COL_METRIC]) as u32;
        let mtu = parse::parse_dec_fast(fields[COL_MTU]) as u32;
        let window = parse::parse_dec_fast(fields[COL_WINDOW]) as u32;
        let irtt = parse::parse_dec_fast(fields[COL_IRTT]) as u32;

        entries.push(Ok(RouteEntry {
            iface,
            destination,
            gateway,
            mask,
            flags,
            refcnt,
            use_,
            metric,
            mtu,
            window,
            irtt,
        }));
    }

    entries.into_iter()
}

/// Parses a little-endian hex IPv4 address from `/proc/net/route`.
///
/// The kernel stores addresses in reversed byte order: `0100000A`
/// means `10.0.0.1`.
fn parse_hex_ipv4(s: &[u8]) -> Ipv4Addr {
    if s.len() != IPV4_HEX_LEN {
        return Ipv4Addr::UNSPECIFIED;
    }

    Ipv4Addr::from(parse::decode_ipv4_fast(s))
}
