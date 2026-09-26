use std::net::{SocketAddrV4, SocketAddrV6};

use crate::error::{Error, Result};
use crate::util::parse;

/// TCP connection state as reported in `/proc/net/tcp`.
///
/// Maps the kernel's hex state codes to typed variants.
/// See `include/net/tcp_states.h` in the kernel source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    /// Connection established.
    Established,
    /// SYN sent, waiting for a matching SYN-ACK.
    SynSent,
    /// SYN received, sent an ACK.
    SynRecv,
    /// First FIN sent, waiting for an ACK or a FIN from the other side.
    FinWait1,
    /// Other side's FIN acknowledged, waiting for a FIN from the other side.
    FinWait2,
    /// Both FINs acknowledged; waiting for a final timeout.
    TimeWait,
    /// Both sides closed, or a reset was received.
    Close,
    /// Remote side sent a FIN; waiting for a close from the local application.
    CloseWait,
    /// Last ACK sent, waiting for a final ACK.
    LastAck,
    /// Listening for incoming connections.
    Listen,
    /// Both sides sent a FIN simultaneously.
    Closing,
    /// SYN received in LISTEN state (syncookies).
    NewSynRecv,
}

impl TcpState {
    fn from_hex(code: u32) -> Self {
        match code {
            0x01 => TcpState::Established,
            0x02 => TcpState::SynSent,
            0x03 => TcpState::SynRecv,
            0x04 => TcpState::FinWait1,
            0x05 => TcpState::FinWait2,
            0x06 => TcpState::TimeWait,
            0x07 => TcpState::Close,
            0x08 => TcpState::CloseWait,
            0x09 => TcpState::LastAck,
            0x0A => TcpState::Listen,
            0x0B => TcpState::Closing,
            0x0C => TcpState::NewSynRecv,
            _ => TcpState::Close,
        }
    }
}

/// A single TCP connection entry from `/proc/net/tcp`.
///
/// Each line represents one socket in the kernel's TCP hash tables.
/// Addresses are stored in network byte order (big-endian) as
/// reported by the kernel.
#[derive(Debug)]
pub struct TcpEntry {
    /// Local socket address.
    pub local: SocketAddrV4,
    /// Remote socket address.
    pub remote: SocketAddrV4,
    /// Connection state.
    pub state: TcpState,
    /// Kernel inode number for the socket.
    ///
    /// Can be cross-referenced with `/proc/PID/fd` to find which
    /// process owns this socket.
    pub inode: u64,
    /// UID of the process that opened this socket.
    pub uid: u32,
    /// Receive queue length (bytes not yet consumed by the application).
    pub rx_queue: u32,
    /// Transmit queue length (bytes not yet acknowledged by the peer).
    pub tx_queue: u32,
}

/// A single TCP6 connection entry from `/proc/net/tcp6`.
///
/// Same layout as [`TcpEntry`] but with IPv6 addresses.
#[derive(Debug)]
pub struct Tcp6Entry {
    /// Local socket address.
    pub local: SocketAddrV6,
    /// Remote socket address.
    pub remote: SocketAddrV6,
    /// Connection state.
    pub state: TcpState,
    /// Kernel inode number for the socket.
    pub inode: u64,
    /// UID of the process that opened this socket.
    pub uid: u32,
    /// Receive queue length (bytes).
    pub rx_queue: u32,
    /// Transmit queue length (bytes).
    pub tx_queue: u32,
}

/// Parses a hex-encoded IPv4 address and port from `/proc/net/tcp`.
///
/// The kernel stores addresses in little-endian hex: `0100007F:1F90`
/// means `127.0.0.1:8080`. The 32-bit address bytes are reversed
/// compared to standard network byte order.
fn parse_ipv4(s: &[u8]) -> Result<SocketAddrV4> {
    let colon = parse::memchr(b':', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<tcp>"),
        line: 0,
        msg: "missing colon in address",
    })?;

    let addr_hex = &s[..colon];
    let port_hex = &s[colon + 1..];

    if addr_hex.len() != 8 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<tcp>"),
            line: 0,
            msg: "invalid IPv4 hex length",
        });
    }

    let bytes = parse::decode_ipv4_fast(addr_hex);
    let port = parse::parse_hex_fast(port_hex) as u16;

    Ok(SocketAddrV4::new(std::net::Ipv4Addr::from(bytes), port))
}

/// Parses a hex-encoded IPv6 address and port from `/proc/net/tcp6`.
///
/// IPv6 addresses in `/proc/net/tcp6` are stored as four 32-bit
/// words in little-endian order, each word itself in big-endian
/// hex. The format is:
/// `AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABBBBBBBB:PPPP`
/// where A = address (32 hex chars), B = scope (8 hex chars, usually 0),
/// PPPP = port.
fn parse_ipv6(s: &[u8]) -> Result<SocketAddrV6> {
    let colon = parse::memchr(b':', s).ok_or_else(|| Error::Parse {
        path: std::path::PathBuf::from("<tcp6>"),
        line: 0,
        msg: "missing colon in address",
    })?;

    let addr_hex = &s[..colon];
    let port_hex = &s[colon + 1..];

    if addr_hex.len() != 32 {
        return Err(Error::Parse {
            path: std::path::PathBuf::from("<tcp6>"),
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

/// Internal helper: parses a `/proc/net/tcp` or `/proc/net/tcp6` file
/// into an iterator of entries.
fn parse_tcp_file(path: &str, is_v6: bool) -> impl Iterator<Item = Result<TcpEntry>> {
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

        let local = if is_v6 {
            match parse_ipv6(fields[1]) {
                Ok(v6) => {
                    if let Some(v4) = v6.ip().to_ipv4_mapped() {
                        SocketAddrV4::new(v4, v6.port())
                    } else {
                        // Not mappable to IPv4 — skip or use default
                        entries.push(Err(Error::Parse {
                            path: std::path::PathBuf::from(path),
                            line: 0,
                            msg: "cannot map IPv6 to IPv4",
                        }));
                        continue;
                    }
                }
                Err(e) => {
                    entries.push(Err(e));
                    continue;
                }
            }
        } else {
            match parse_ipv4(fields[1]) {
                Ok(addr) => addr,
                Err(e) => {
                    entries.push(Err(e));
                    continue;
                }
            }
        };

        let remote = if is_v6 {
            match parse_ipv6(fields[2]) {
                Ok(v6) => {
                    if let Some(v4) = v6.ip().to_ipv4_mapped() {
                        SocketAddrV4::new(v4, v6.port())
                    } else {
                        entries.push(Err(Error::Parse {
                            path: std::path::PathBuf::from(path),
                            line: 0,
                            msg: "cannot map IPv6 to IPv4",
                        }));
                        continue;
                    }
                }
                Err(e) => {
                    entries.push(Err(e));
                    continue;
                }
            }
        } else {
            match parse_ipv4(fields[2]) {
                Ok(addr) => addr,
                Err(e) => {
                    entries.push(Err(e));
                    continue;
                }
            }
        };

        let state_code = parse::parse_hex_fast(fields[3]) as u32;
        let state = TcpState::from_hex(state_code);

        let (tx_queue_raw, rx_queue_raw) = parse::split_at_byte(fields[4], b':');
        let tx_queue = parse::parse_hex_fast(tx_queue_raw) as u32;
        let rx_queue = parse::parse_hex_fast(rx_queue_raw) as u32;

        let uid = parse::parse_dec_fast(fields[7]) as u32;
        let inode = parse::parse_dec_fast(fields[9]);

        entries.push(Ok(TcpEntry {
            local,
            remote,
            state,
            inode,
            uid,
            rx_queue,
            tx_queue,
        }));
    }

    entries.into_iter()
}

/// Reads `/proc/net/tcp` and returns an iterator over active TCP
/// connections (IPv4 only).
///
/// Each entry includes local/remote addresses, connection state,
/// queue lengths, and the owning UID and kernel inode.
pub fn tcp() -> impl Iterator<Item = Result<TcpEntry>> {
    parse_tcp_file("/proc/net/tcp", false)
}

/// Reads `/proc/net/tcp6` and returns an iterator over active TCP
/// connections (IPv6).
///
/// IPv6-mapped IPv4 addresses are converted to [`SocketAddrV4`]
/// where possible. Pure IPv6 entries yield a parse error since
/// [`TcpEntry`] only supports IPv4.
pub fn tcp6() -> impl Iterator<Item = Result<TcpEntry>> {
    parse_tcp_file("/proc/net/tcp6", true)
}

/// Parses the raw IPv6 address bytes into a [`SocketAddrV6`].
/// Used by the tcp6-specific entry parser.
pub fn parse_tcp6_entries() -> impl Iterator<Item = Result<Tcp6Entry>> {
    let path = "/proc/net/tcp6";
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

        let state_code = parse::parse_hex_fast(fields[3]) as u32;
        let state = TcpState::from_hex(state_code);

        let (tx_queue_raw, rx_queue_raw) = parse::split_at_byte(fields[4], b':');
        let tx_queue = parse::parse_hex_fast(tx_queue_raw) as u32;
        let rx_queue = parse::parse_hex_fast(rx_queue_raw) as u32;

        let uid = parse::parse_dec_fast(fields[7]) as u32;
        let inode = parse::parse_dec_fast(fields[9]);

        entries.push(Ok(Tcp6Entry {
            local,
            remote,
            state,
            inode,
            uid,
            rx_queue,
            tx_queue,
        }));
    }

    entries.into_iter()
}
