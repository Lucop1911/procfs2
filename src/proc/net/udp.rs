use std::net::{SocketAddrV4, SocketAddrV6};

use crate::error::{Error, Result};
use crate::util::parse;

/// A single UDP socket entry from `/proc/net/udp`.
///
/// UDP is connectionless, so `remote` is typically `0.0.0.0:0`
/// unless the socket has called `connect()`.
#[derive(Debug)]
pub struct UdpEntry {
    /// Local socket address.
    pub local: SocketAddrV4,
    /// Remote socket address.
    pub remote: SocketAddrV4,
    /// Kernel socket state (usually 0x07 for unconnected, 0x01 for connected).
    pub state: u32,
    /// UID of the process that opened this socket.
    pub uid: u32,
    /// Kernel inode number for the socket.
    pub inode: u64,
    /// Receive queue length (bytes).
    pub rx_queue: u32,
    /// Transmit queue length (bytes).
    pub tx_queue: u32,
}

/// A single UDP6 socket entry from `/proc/net/udp6`.
#[derive(Debug)]
pub struct Udp6Entry {
    /// Local socket address.
    pub local: SocketAddrV6,
    /// Remote socket address.
    pub remote: SocketAddrV6,
    /// Kernel socket state.
    pub state: u32,
    /// UID of the process that opened this socket.
    pub uid: u32,
    /// Kernel inode number for the socket.
    pub inode: u64,
    /// Receive queue length (bytes).
    pub rx_queue: u32,
    /// Transmit queue length (bytes).
    pub tx_queue: u32,
}

/// Parses a hex-encoded IPv4 address and port from `/proc/net/udp`.
fn parse_ipv4(s: &[u8]) -> Result<SocketAddrV4> {
    let colon = parse::memchr(b':', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<udp>"),
        line: 0,
        msg: "missing colon in address",
    })?;

    let addr_hex = &s[..colon];
    let port_hex = &s[colon + 1..];

    if addr_hex.len() != 8 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<udp>"),
            line: 0,
            msg: "invalid IPv4 hex length",
        });
    }

    let bytes = parse::decode_ipv4_fast(addr_hex);
    let port = parse::parse_hex_fast(port_hex) as u16;

    Ok(SocketAddrV4::new(std::net::Ipv4Addr::from(bytes), port))
}

/// Parses a hex-encoded IPv6 address and port from `/proc/net/udp6`.
fn parse_ipv6(s: &[u8]) -> Result<SocketAddrV6> {
    let colon = parse::memchr(b':', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<udp6>"),
        line: 0,
        msg: "missing colon in address",
    })?;

    let addr_hex = &s[..colon];
    let port_hex = &s[colon + 1..];

    if addr_hex.len() != 32 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<udp6>"),
            line: 0,
            msg: "invalid IPv6 hex length",
        });
    }

    let bytes = parse::decode_ipv6_fast(addr_hex);

    let port = parse::parse_hex_fast(port_hex) as u16;

    Ok(SocketAddrV6::new(
        std::net::Ipv6Addr::from(bytes),
        port,
        0,
        0,
    ))
}

/// Reads `/proc/net/udp` and returns an iterator over UDP sockets (IPv4).
pub fn udp() -> impl Iterator<Item = Result<UdpEntry>> {
    parse_udp_file("/proc/net/udp", false)
}

/// Reads `/proc/net/udp6` and returns an iterator over UDP sockets (IPv6).
pub fn udp6() -> impl Iterator<Item = Result<Udp6Entry>> {
    parse_udp6_file("/proc/net/udp6")
}

fn parse_udp_file(path: &str, _is_v6: bool) -> impl Iterator<Item = Result<UdpEntry>> {
    let bytes = match parse::read_file(std::path::Path::new(path)) {
        Ok(b) => b,
        Err(e) => return vec![Err(e)].into_iter(),
    };

    let mut entries = Vec::with_capacity(parse::count_byte(b'\n', &bytes));

    for line in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
    {
        let fields = parse::SplitFields::<12>::new(line);
        if fields.len() < 12 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let local = match parse_ipv4(fields[1]) {
            Ok(addr) => addr,
            Err(e) => {
                entries.push(Err(e));
                continue;
            }
        };

        let remote = match parse_ipv4(fields[2]) {
            Ok(addr) => addr,
            Err(e) => {
                entries.push(Err(e));
                continue;
            }
        };

        let state = parse::parse_hex_fast(fields[3]) as u32;

        let (tx_queue_raw, rx_queue_raw) = parse::split_at_byte(fields[4], b':');
        let tx_queue = parse::parse_hex_fast(tx_queue_raw) as u32;
        let rx_queue = parse::parse_hex_fast(rx_queue_raw) as u32;

        let uid = parse::parse_dec_fast(fields[7]) as u32;
        let inode = parse::parse_dec_fast(fields[9]);

        entries.push(Ok(UdpEntry {
            local,
            remote,
            state,
            uid,
            inode,
            rx_queue,
            tx_queue,
        }));
    }

    entries.into_iter()
}

fn parse_udp6_file(path: &str) -> impl Iterator<Item = Result<Udp6Entry>> {
    let bytes = match parse::read_file(std::path::Path::new(path)) {
        Ok(b) => b,
        Err(e) => return vec![Err(e)].into_iter(),
    };

    let mut entries = Vec::with_capacity(parse::count_byte(b'\n', &bytes));

    for line in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
    {
        let fields = parse::SplitFields::<12>::new(line);
        if fields.len() < 12 {
            entries.push(Err(Error::Parse {
                path: std::path::PathBuf::from(path),
                line: 0,
                msg: "not enough fields",
            }));
            continue;
        }

        let local = match parse_ipv6(fields[1]) {
            Ok(addr) => addr,
            Err(e) => {
                entries.push(Err(e));
                continue;
            }
        };

        let remote = match parse_ipv6(fields[2]) {
            Ok(addr) => addr,
            Err(e) => {
                entries.push(Err(e));
                continue;
            }
        };

        let state = parse::parse_hex_fast(fields[3]) as u32;

        let (tx_queue_raw, rx_queue_raw) = parse::split_at_byte(fields[4], b':');
        let tx_queue = parse::parse_hex_fast(tx_queue_raw) as u32;
        let rx_queue = parse::parse_hex_fast(rx_queue_raw) as u32;

        let uid = parse::parse_dec_fast(fields[7]) as u32;
        let inode = parse::parse_dec_fast(fields[9]);

        entries.push(Ok(Udp6Entry {
            local,
            remote,
            state,
            uid,
            inode,
            rx_queue,
            tx_queue,
        }));
    }

    entries.into_iter()
}
