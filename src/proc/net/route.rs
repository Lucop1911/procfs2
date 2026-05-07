use std::net::Ipv4Addr;

use crate::error::{Error, Result};
use crate::util::parse;

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
    let path = "/proc/net/route";
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
        if fields.len() < 11 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let iface = std::str::from_utf8(fields[0])
            .unwrap_or("")
            .to_string()
            .into_boxed_str();

        let destination = parse_hex_ipv4(fields[1]);
        let gateway = parse_hex_ipv4(fields[2]);
        let mask = parse_hex_ipv4(fields[7]);

        let flags = parse::parse_hex_u64(fields[3]).unwrap_or(0) as u16;
        let refcnt = parse::parse_dec_u64(fields[4]).unwrap_or(0) as u16;
        let use_ = parse::parse_dec_u32(fields[5]).unwrap_or(0);
        let metric = parse::parse_dec_u32(fields[6]).unwrap_or(0);
        let mtu = parse::parse_dec_u32(fields[8]).unwrap_or(0);
        let window = parse::parse_dec_u32(fields[9]).unwrap_or(0);
        let irtt = parse::parse_dec_u32(fields[10]).unwrap_or(0);

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
    if s.len() != 8 {
        return Ipv4Addr::UNSPECIFIED;
    }

    let mut bytes = [0u8; 4];
    for i in 0..4 {
        let pair = &s[i * 2..i * 2 + 2];
        bytes[3 - i] =
            u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16).unwrap_or(0);
    }

    Ipv4Addr::from(bytes)
}
