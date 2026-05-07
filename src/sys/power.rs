use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::util::parse;

/// Type of power supply.
///
/// Sourced from `/sys/class/power_supply/<name>/type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSupplyType {
    Battery,
    Mains,
    Usb,
    UsbDcp,
    UsbCdp,
    UsbAca,
    Wireless,
    BatteryBackup,
}

impl PowerSupplyType {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "Battery" => PowerSupplyType::Battery,
            "Mains" => PowerSupplyType::Mains,
            "USB" => PowerSupplyType::Usb,
            "USB_DCP" => PowerSupplyType::UsbDcp,
            "USB_CDP" => PowerSupplyType::UsbCdp,
            "USB_ACA" => PowerSupplyType::UsbAca,
            "Wireless" => PowerSupplyType::Wireless,
            "BatteryBackup" => PowerSupplyType::BatteryBackup,
            _ => PowerSupplyType::Battery,
        }
    }
}

/// Charging state of a power supply.
///
/// Sourced from `/sys/class/power_supply/<name>/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeStatus {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl ChargeStatus {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "Charging" => ChargeStatus::Charging,
            "Discharging" => ChargeStatus::Discharging,
            "Not charging" => ChargeStatus::NotCharging,
            "Full" => ChargeStatus::Full,
            _ => ChargeStatus::Unknown,
        }
    }
}

/// A power supply exposed under `/sys/class/power_supply/<name>/`.
///
/// Covers batteries, AC adapters, and USB power sources.
pub struct PowerSupply {
    pub name: Box<str>,
    base: PathBuf,
}

impl PowerSupply {
    /// Iterates over all power supplies in `/sys/class/power_supply/`.
    pub fn all() -> impl Iterator<Item = Result<Self>> {
        let entries = match std::fs::read_dir("/sys/class/power_supply") {
            Ok(iter) => iter,
            Err(e) => return vec![Err(Error::Io(e))].into_iter(),
        };

        entries
            .filter_map(|entry| match entry {
                Ok(e) => {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.is_empty() {
                        Some(Ok(PowerSupply {
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

    /// Reads `/sys/class/power_supply/<name>/type`.
    pub fn kind(&self) -> Result<PowerSupplyType> {
        let path = self.base.join("type");
        let bytes = parse::read_file(&path)?;
        let s = std::str::from_utf8(&bytes).unwrap_or("");
        Ok(PowerSupplyType::from_str(s))
    }

    /// Reads `/sys/class/power_supply/<name>/status`.
    pub fn status(&self) -> Result<ChargeStatus> {
        let path = self.base.join("status");
        let bytes = parse::read_file(&path)?;
        let s = std::str::from_utf8(&bytes).unwrap_or("");
        Ok(ChargeStatus::from_str(s))
    }

    /// Reads `/sys/class/power_supply/<name>/capacity`.
    ///
    /// Returns `None` if the file is absent (e.g. AC adapters
    /// don't have a capacity).
    pub fn capacity(&self) -> Result<Option<u8>> {
        let path = self.base.join("capacity");
        match parse::read_file(&path) {
            Ok(bytes) => {
                let val = parse::parse_dec_u64(&bytes).unwrap_or(0) as u8;
                Ok(Some(val))
            }
            Err(_) => Ok(None),
        }
    }

    /// Reads `/sys/class/power_supply/<name>/voltage_now`.
    ///
    /// Returns voltage in microvolts. `None` if the file is absent.
    pub fn voltage_now(&self) -> Result<Option<u64>> {
        let path = self.base.join("voltage_now");
        match parse::read_file(&path) {
            Ok(bytes) => parse::parse_dec_u64(&bytes).map(Some),
            Err(_) => Ok(None),
        }
    }

    /// Reads `/sys/class/power_supply/<name>/current_now`.
    ///
    /// Returns current in microamps. Negative values indicate
    /// discharging. `None` if the file is absent.
    pub fn current_now(&self) -> Result<Option<i64>> {
        let path = self.base.join("current_now");
        match parse::read_file(&path) {
            Ok(bytes) => parse::parse_dec_i64(&bytes).map(Some),
            Err(_) => Ok(None),
        }
    }
}
