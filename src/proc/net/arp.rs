use std::net::Ipv4Addr;

use crate::error::{Error, Result};
use crate::util::parse;

/// A single ARP table entry from `/proc/net/arp`.
///
/// The ARP table maps IPv4 addresses to MAC addresses on the local
/// network. Each entry is associated with a network interface.
#[derive(Debug)]
pub struct ArpEntry {
    /// IP address of the neighbor.
    pub ip: Ipv4Addr,
    /// Hardware type (1 = Ethernet).
    pub hw_type: u16,
    /// ARP flags (e.g. `0x02` = complete, `0x06` = published).
    pub flags: u16,
    /// MAC address of the neighbor.
    pub mac: [u8; 6],
    /// Network interface this entry belongs to.
    pub device: Box<str>,
}

/// Reads `/proc/net/arp` and returns an iterator over ARP entries.
///
/// The file format is:
/// ```text
/// IP address       HW type    Flags    HW address            Mask    Device
/// 192.168.1.1      0x1        0x2      aa:bb:cc:dd:ee:ff     *       eth0
/// ```
pub fn arp() -> impl Iterator<Item = Result<ArpEntry>> {
    let path = "/proc/net/arp";
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
        if fields.len() < 6 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let ip_str = std::str::from_utf8(fields[0]).unwrap_or("");
        let ip: Ipv4Addr = match ip_str.parse() {
            Ok(addr) => addr,
            Err(_) => {
                entries.push(Err(Error::Parse {
                    path: std::path::PathBuf::from(path),
                    line: 0,
                    msg: "invalid IPv4 address",
                }));
                continue;
            }
        };

        let hw_type = parse::parse_hex_u64(fields[1]).unwrap_or(0) as u16;
        let flags = parse::parse_hex_u64(fields[2]).unwrap_or(0) as u16;

        let mac_str = std::str::from_utf8(fields[3]).unwrap_or("");
        let mac = parse_mac(mac_str);

        let device = std::str::from_utf8(fields[5])
            .unwrap_or("")
            .to_string()
            .into_boxed_str();

        entries.push(Ok(ArpEntry {
            ip,
            hw_type,
            flags,
            mac,
            device,
        }));
    }

    entries.into_iter()
}

/// Parses a colon-separated MAC address string into `[u8; 6]`.
///
/// Returns `[0; 6]` on parse failure rather than erroring, since
/// incomplete ARP entries may have placeholder MAC addresses.
fn parse_mac(s: &str) -> [u8; 6] {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return [0; 6];
    }

    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16).unwrap_or(0);
    }
    mac
}
