use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::Bytes;

/// Per-interface network device statistics from `/proc/net/dev`.
///
/// Each line describes one network interface with separate RX
/// (receive) and TX (transmit) counters. The format is:
/// ```text
///  eth0: 1234567 12345 0 0 0 0 0 0 7654321 5432 0 0 0 0 0 0
/// ```
/// The first 8 counters are RX, the next 8 are TX.
#[derive(Debug)]
pub struct NetDevStat {
    /// Interface name (e.g. `eth0`, `lo`).
    pub name: Box<str>,
    /// Total bytes received.
    pub rx_bytes: Bytes,
    /// Total packets received.
    pub rx_packets: u64,
    /// Receive errors.
    pub rx_errors: u64,
    /// Packets dropped by the driver on receive.
    pub rx_drop: u64,
    /// FIFO buffer errors on receive.
    pub rx_fifo_errors: u64,
    /// Frame alignment errors on receive.
    pub rx_frame_errors: u64,
    /// Compressed packets received.
    pub rx_compressed: u64,
    /// Multicast packets received.
    pub rx_multicast: u64,
    /// Total bytes transmitted.
    pub tx_bytes: Bytes,
    /// Total packets transmitted.
    pub tx_packets: u64,
    /// Transmit errors.
    pub tx_errors: u64,
    /// Packets dropped by the driver on transmit.
    pub tx_drop: u64,
    /// FIFO buffer errors on transmit.
    pub tx_fifo_errors: u64,
    /// Collisions during transmission.
    pub tx_collisions: u64,
    /// Carrier errors during transmission.
    pub tx_carrier_errors: u64,
    /// Compressed packets transmitted.
    pub tx_compressed: u64,
}

/// Reads `/proc/net/dev` and returns an iterator over all network
/// interfaces.
///
/// The file has a two-line header followed by one line per interface.
/// Interface names are followed by a colon and may have leading
/// whitespace for alignment.
pub fn dev() -> impl Iterator<Item = Result<NetDevStat>> {
    let path = "/proc/net/dev";
    let bytes = match parse::read_file(std::path::Path::new(path)) {
        Ok(b) => b,
        Err(e) => return vec![Err(e)].into_iter(),
    };

    let lines: Vec<&[u8]> = bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(2) // skip two-line header
        .collect();

    let mut entries = Vec::with_capacity(lines.len());

    for line in lines {
        let colon = match parse::memchr(b':', line) {
            Some(idx) => idx,
            None => {
                entries.push(Err(Error::Parse {
                    path: std::path::PathBuf::from(path),
                    line: 0,
                    msg: "missing colon in device line",
                }));
                continue;
            }
        };

        let name = parse::trim_end(&line[..colon]);
        let name_str = std::str::from_utf8(name)
            .unwrap_or("")
            .trim()
            .to_string()
            .into_boxed_str();

        let counters = parse::trim_start(&line[colon + 1..]);
        let fields: Vec<&[u8]> = parse::split_spaces(counters);

        if fields.len() < 16 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "expected 16 counter fields",
            }));
            continue;
        }

        let get = |i: usize| parse::parse_dec_u64(fields[i]).unwrap_or(0);

        entries.push(Ok(NetDevStat {
            name: name_str,
            rx_bytes: Bytes(get(0)),
            rx_packets: get(1),
            rx_errors: get(2),
            rx_drop: get(3),
            rx_fifo_errors: get(4),
            rx_frame_errors: get(5),
            rx_compressed: get(6),
            rx_multicast: get(7),
            tx_bytes: Bytes(get(8)),
            tx_packets: get(9),
            tx_errors: get(10),
            tx_drop: get(11),
            tx_fifo_errors: get(12),
            tx_collisions: get(13),
            tx_carrier_errors: get(14),
            tx_compressed: get(15),
        }));
    }

    entries.into_iter()
}
