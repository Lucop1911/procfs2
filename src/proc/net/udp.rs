use std::net::{SocketAddrV4, SocketAddrV6};

use crate::error::{Error, Result};
use crate::util::parse;

/// A single UDP socket entry from `/proc/net/udp`.
///
/// UDP is connectionless, so `remote` is typically `0.0.0.0:0`
/// unless the socket has called `connect()`.
#[derive(Debug)]
pub struct UdpEntry {
    pub local: SocketAddrV4,
    pub remote: SocketAddrV4,
    /// Kernel socket state (usually 0x07 for unconnected, 0x01 for connected).
    pub state: u32,
    /// UID of the process that opened this socket.
    pub uid: u32,
    pub inode: u64,
    pub rx_queue: u32,
    pub tx_queue: u32,
}

/// A single UDP6 socket entry from `/proc/net/udp6`.
#[derive(Debug)]
pub struct Udp6Entry {
    pub local: SocketAddrV6,
    pub remote: SocketAddrV6,
    pub state: u32,
    /// UID of the process that opened this socket.
    pub uid: u32,
    pub inode: u64,
    pub rx_queue: u32,
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

    let mut bytes = [0u8; 4];
    for i in 0..4 {
        let pair = &addr_hex[i * 2..i * 2 + 2];
        bytes[3 - i] = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| Error::Parse {
                path: std::path::PathBuf::from("<udp>"),
                line: 0,
                msg: "invalid utf8 in addr",
            })?,
            16,
        )
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<udp>"),
            line: 0,
            msg: "invalid hex byte",
        })?;
    }

    let port = u16::from_str_radix(
        std::str::from_utf8(port_hex).map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<udp>"),
            line: 0,
            msg: "invalid utf8 in port",
        })?,
        16,
    )
    .map_err(|_| Error::Parse {
        path: std::path::PathBuf::from("<udp>"),
        line: 0,
        msg: "invalid port",
    })?;

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

    let mut bytes = [0u8; 16];
    for i in 0..16 {
        let pair = &addr_hex[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| Error::Parse {
                path: std::path::PathBuf::from("<udp6>"),
                line: 0,
                msg: "invalid utf8 in addr",
            })?,
            16,
        )
        .map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<udp6>"),
            line: 0,
            msg: "invalid hex byte",
        })?;
    }

    for chunk in bytes.chunks_exact_mut(4) {
        chunk.reverse();
    }

    let port = u16::from_str_radix(
        std::str::from_utf8(port_hex).map_err(|_| Error::Parse {
            path: std::path::PathBuf::from("<udp6>"),
            line: 0,
            msg: "invalid utf8 in port",
        })?,
        16,
    )
    .map_err(|_| Error::Parse {
        path: std::path::PathBuf::from("<udp6>"),
        line: 0,
        msg: "invalid port",
    })?;

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

    let lines: Vec<&[u8]> = bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
        .collect();

    let mut entries = Vec::with_capacity(lines.len());

    for line in lines {
        let fields: Vec<&[u8]> = parse::split_spaces(line);
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

        let state = parse::parse_hex_u64(fields[3]).unwrap_or(0) as u32;

        let rx_tx = fields[4];
        let rx_tx_fields: Vec<&[u8]> = parse::split_spaces(rx_tx);
        let rx_queue = if !rx_tx_fields.is_empty() {
            parse::parse_hex_u64(rx_tx_fields[0]).unwrap_or(0) as u32
        } else {
            0
        };
        let tx_queue = if rx_tx_fields.len() >= 2 {
            parse::parse_hex_u64(rx_tx_fields[1]).unwrap_or(0) as u32
        } else {
            0
        };

        let uid = parse::parse_dec_u32(fields[7]).unwrap_or(0);
        let inode = parse::parse_dec_u64(fields[9]).unwrap_or(0);

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

    let lines: Vec<&[u8]> = bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .skip(1)
        .collect();

    let mut entries = Vec::with_capacity(lines.len());

    for line in lines {
        let fields: Vec<&[u8]> = parse::split_spaces(line);
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

        let state = parse::parse_hex_u64(fields[3]).unwrap_or(0) as u32;

        let rx_tx = fields[4];
        let rx_tx_fields: Vec<&[u8]> = parse::split_spaces(rx_tx);
        let rx_queue = if !rx_tx_fields.is_empty() {
            parse::parse_hex_u64(rx_tx_fields[0]).unwrap_or(0) as u32
        } else {
            0
        };
        let tx_queue = if rx_tx_fields.len() >= 2 {
            parse::parse_hex_u64(rx_tx_fields[1]).unwrap_or(0) as u32
        } else {
            0
        };

        let uid = parse::parse_dec_u32(fields[7]).unwrap_or(0);
        let inode = parse::parse_dec_u64(fields[9]).unwrap_or(0);

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
