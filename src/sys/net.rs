use std::path::PathBuf;

use bitflags::bitflags;

use crate::error::{Error, Result};
use crate::util::Bytes;
use crate::util::parse;

bitflags! {
    /// Network interface flags.
    ///
    /// Parsed from the hex value in `/sys/class/net/<name>/flags`.
    /// Mirrors the `IFF_*` constants from `<net/if.h>`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct NetIfFlags: u32 {
        /// Interface is up.
        const UP          = 1 << 0;
        /// Broadcast address is valid.
        const BROADCAST   = 1 << 1;
        /// Debugging is enabled.
        const DEBUG       = 1 << 2;
        /// Loopback interface.
        const LOOPBACK    = 1 << 3;
        /// Point-to-point link.
        const POINTOPOINT = 1 << 4;
        /// No packet headers.
        const NOTRAILERS  = 1 << 5;
        /// Interface is operational.
        const RUNNING     = 1 << 6;
        /// No ARP protocol.
        const NOARP       = 1 << 7;
        /// Promiscuous mode enabled.
        const PROMISC     = 1 << 8;
        /// Receive all multicast packets.
        const ALLMULTI    = 1 << 9;
        /// Master of a bonding/bridging slave.
        const MASTER      = 1 << 10;
        /// Slave of a bonding/bridging master.
        const SLAVE       = 1 << 11;
        /// Supports multicast.
        const MULTICAST   = 1 << 12;
        /// Media selection (obsolete).
        const PORTSEL     = 1 << 13;
        /// Auto media selection active.
        const AUTOMEDIA   = 1 << 14;
        /// Dynamic address is in use.
        const DYNAMIC     = 1 << 15;
    }
}

/// Operational state of a network interface.
///
/// Sourced from `/sys/class/net/<name>/operstate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperState {
    /// State is unknown.
    Unknown,
    /// Device is not present in the system.
    NotPresent,
    /// Link is down.
    Down,
    /// Link is down due to lower-layer issues (e.g. no carrier).
    LowerLayerDown,
    /// Interface is in testing mode.
    Testing,
    /// Interface is operational but not transmitting.
    Dormant,
    /// Link is up and ready to transmit.
    Up,
}

impl OperState {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "unknown" => OperState::Unknown,
            "notpresent" => OperState::NotPresent,
            "down" => OperState::Down,
            "lowerlayerdown" => OperState::LowerLayerDown,
            "testing" => OperState::Testing,
            "dormant" => OperState::Dormant,
            "up" => OperState::Up,
            _ => OperState::Unknown,
        }
    }
}

/// A MAC address (6 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

/// Per-interface statistics from `/sys/class/net/<name>/statistics/`.
#[derive(Debug)]
pub struct NetIfStat {
    /// Total bytes received.
    pub rx_bytes: Bytes,
    /// Total packets received.
    pub rx_packets: u64,
    /// Receive errors.
    pub rx_errors: u64,
    /// Receive drops.
    pub rx_drop: u64,
    /// Total bytes transmitted.
    pub tx_bytes: Bytes,
    /// Total packets transmitted.
    pub tx_packets: u64,
    /// Transmit errors.
    pub tx_errors: u64,
    /// Transmit drops.
    pub tx_drop: u64,
}

/// A network interface exposed under `/sys/class/net/<name>/`.
pub struct NetInterface {
    /// Interface name (e.g. `eth0`, `wlan0`, `lo`).
    pub name: Box<str>,
    base: PathBuf,
}

impl NetInterface {
    /// Iterates over all network interfaces in `/sys/class/net/`.
    pub fn all() -> impl Iterator<Item = Result<Self>> {
        let entries = match std::fs::read_dir("/sys/class/net") {
            Ok(iter) => iter,
            Err(e) => {
                return vec![Err(Error::Io {
                    path: Some(std::path::PathBuf::from("/sys/class/net")),
                    error: e,
                })]
                .into_iter();
            }
        };

        entries
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.is_empty() {
                        Some(Ok(NetInterface {
                            name: name_str.into_owned().into_boxed_str(),
                            base: e.path(),
                        }))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(Error::Io {
                    path: Some(std::path::PathBuf::from("/sys/class/net")),
                    error: e,
                })),
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Reads statistics from `/sys/class/net/<name>/statistics/`.
    pub fn stats(&self) -> Result<NetIfStat> {
        let stats = self.base.join("statistics");
        let get = |file: &str| -> u64 {
            let path = stats.join(file);
            match parse::read_file(&path) {
                Ok(bytes) => parse::parse_dec_u64(&bytes).unwrap_or(0),
                Err(_) => 0,
            }
        };

        Ok(NetIfStat {
            rx_bytes: Bytes(get("rx_bytes")),
            rx_packets: get("rx_packets"),
            rx_errors: get("rx_errors"),
            rx_drop: get("rx_drop"),
            tx_bytes: Bytes(get("tx_bytes")),
            tx_packets: get("tx_packets"),
            tx_errors: get("tx_errors"),
            tx_drop: get("tx_drop"),
        })
    }

    /// Reads `/sys/class/net/<name>/operstate`.
    pub fn operstate(&self) -> Result<OperState> {
        let path = self.base.join("operstate");
        let bytes = parse::read_file(&path)?;
        let s = std::str::from_utf8(&bytes).unwrap_or("").trim();
        Ok(OperState::from_str(s))
    }

    /// Reads `/sys/class/net/<name>/address` (MAC address).
    pub fn address(&self) -> Result<MacAddress> {
        let path = self.base.join("address");
        let bytes = parse::read_file(&path)?;
        let s = std::str::from_utf8(&bytes).unwrap_or("").trim();
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(Error::Parse {
                path,
                line: 0,
                msg: "invalid MAC address format",
            });
        }

        let mut mac = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            mac[i] = u8::from_str_radix(part, 16).map_err(|_| Error::Parse {
                path: path.clone(),
                line: 0,
                msg: "invalid hex byte in MAC",
            })?;
        }

        Ok(MacAddress(mac))
    }

    /// Reads `/sys/class/net/<name>/mtu`.
    pub fn mtu(&self) -> Result<u32> {
        let path = self.base.join("mtu");
        let bytes = parse::read_file(&path)?;
        parse::parse_dec_u32(&bytes)
    }

    /// Reads `/sys/class/net/<name>/flags`.
    ///
    /// The kernel reports flags as a hex number.
    pub fn flags(&self) -> Result<NetIfFlags> {
        let path = self.base.join("flags");
        let bytes = parse::read_file(&path)?;
        let val = parse::parse_hex_u64(&bytes)? as u32;
        Ok(NetIfFlags::from_bits_truncate(val))
    }

    /// Reads `/sys/class/net/<name>/speed`.
    ///
    /// Returns `None` if the file is absent or contains `EIO`
    /// (common for wireless interfaces and loopback).
    pub fn speed(&self) -> Result<Option<u32>> {
        let path = self.base.join("speed");
        match parse::read_file(&path) {
            Ok(bytes) => {
                let s = std::str::from_utf8(&bytes).unwrap_or("").trim();
                if s.is_empty() || s == "-1" {
                    Ok(None)
                } else {
                    parse::parse_dec_u32(s.as_bytes()).map(Some)
                }
            }
            Err(_) => Ok(None),
        }
    }
}
