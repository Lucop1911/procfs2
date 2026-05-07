use std::path::PathBuf;

use bitflags::bitflags;

use crate::error::{Error, Result};
use crate::util::parse;
use crate::util::Bytes;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct NetIfFlags: u32 {
        const UP          = 1 << 0;
        const BROADCAST   = 1 << 1;
        const DEBUG       = 1 << 2;
        const LOOPBACK    = 1 << 3;
        const POINTOPOINT = 1 << 4;
        const NOTRAILERS  = 1 << 5;
        const RUNNING     = 1 << 6;
        const NOARP       = 1 << 7;
        const PROMISC     = 1 << 8;
        const ALLMULTI    = 1 << 9;
        const MASTER      = 1 << 10;
        const SLAVE       = 1 << 11;
        const MULTICAST   = 1 << 12;
        const PORTSEL     = 1 << 13;
        const AUTOMEDIA   = 1 << 14;
        const DYNAMIC     = 1 << 15;
    }
}

/// Operational state of a network interface.
///
/// Sourced from `/sys/class/net/<name>/operstate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperState {
    Unknown,
    NotPresent,
    Down,
    LowerLayerDown,
    Testing,
    Dormant,
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
    pub rx_bytes: Bytes,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_drop: u64,
    pub tx_bytes: Bytes,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_drop: u64,
}

/// A network interface exposed under `/sys/class/net/<name>/`.
pub struct NetInterface {
    pub name: Box<str>,
    base: PathBuf,
}

impl NetInterface {
    /// Iterates over all network interfaces in `/sys/class/net/`.
    pub fn all() -> impl Iterator<Item = Result<Self>> {
        let entries = match std::fs::read_dir("/sys/class/net") {
            Ok(iter) => iter,
            Err(e) => return vec![Err(Error::Io(e))].into_iter(),
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
                Err(e) => Some(Err(Error::Io(e))),
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
