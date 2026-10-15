use std::net::Ipv4Addr;
use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// Bytes in a MAC address.
const MAC_LEN: usize = 6;
/// Hex digits in one byte.
const HEX_PAIR_LEN: usize = 2;
/// Characters per MAC group: a hex byte plus its `:` separator.
const MAC_GROUP_STRIDE: usize = HEX_PAIR_LEN + 1;
/// Characters in a `aa:bb:cc:dd:ee:ff` string.
const MAC_STR_LEN: usize = MAC_LEN * MAC_GROUP_STRIDE - 1;
/// Columns in the `/proc/net/arp` table, header excluded.
const ARP_FIELDS: usize = 6;
// Column indices, in the order the header prints them.
const COL_IP: usize = 0;
const COL_HW_TYPE: usize = 1;
const COL_FLAGS: usize = 2;
const COL_MAC: usize = 3;
const COL_DEVICE: usize = 5;

/// Strips the `0x` prefix from a hex column, if present.
fn strip_0x(field: &[u8]) -> &[u8] {
    field.strip_prefix(b"0x").unwrap_or(field)
}

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
    pub mac: [u8; MAC_LEN],
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
    let path = Path::new("/proc/net/arp");
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
        let fields = parse::SplitFields::<ARP_FIELDS>::new(line);
        if fields.len() < ARP_FIELDS {
            entries.push(Err(Error::Parse {
                path: path.to_path_buf(),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let ip_str = std::str::from_utf8(fields[COL_IP]).unwrap_or("");
        let ip: Ipv4Addr = match ip_str.parse() {
            Ok(addr) => addr,
            Err(_) => {
                entries.push(Err(Error::Parse {
                    path: path.to_path_buf(),
                    line: 0,
                    msg: "invalid IPv4 address",
                }));
                continue;
            }
        };

        let hw_type = parse::parse_hex_fast(strip_0x(fields[COL_HW_TYPE])) as u16;
        let flags = parse::parse_hex_fast(strip_0x(fields[COL_FLAGS])) as u16;

        let mac = parse_mac(fields[COL_MAC]);

        let device = std::str::from_utf8(fields[COL_DEVICE])
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

/// Parses a colon-separated MAC address string into a [`MAC_LEN`]-byte array.
///
/// Returns an all-zero MAC on parse failure rather than erroring, since
/// incomplete ARP entries may have placeholder MAC addresses.
fn parse_mac(bytes: &[u8]) -> [u8; MAC_LEN] {
    if bytes.len() != MAC_STR_LEN {
        return [0; MAC_LEN];
    }

    // Every group but the last is two hex digits followed by `:`.
    for i in 0..MAC_LEN - 1 {
        if bytes[i * MAC_GROUP_STRIDE + HEX_PAIR_LEN] != b':' {
            return [0; MAC_LEN];
        }
    }

    let mut mac = [0u8; MAC_LEN];
    for (i, byte) in mac.iter_mut().enumerate() {
        let start = i * MAC_GROUP_STRIDE;
        *byte = parse::parse_hex_fast(&bytes[start..start + HEX_PAIR_LEN]) as u8;
    }

    mac
}
