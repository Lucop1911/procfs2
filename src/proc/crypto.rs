use std::path::Path;

use crate::error::{Error, Result};
use crate::util::parse;

/// A cryptographic implementation registered with the kernel.
#[derive(Debug)]
pub struct Crypto {
    /// Algorithm name (e.g. `gcm(aes)` or `sha256`).
    pub name: Box<str>,
    /// Driver implementing the algorithm.
    pub driver: Box<str>,
    /// Kernel module providing the driver.
    pub module: Box<str>,
    /// Algorithm priority.
    pub priority: i64,
    /// Number of active references.
    pub ref_count: i64,
    /// Self-test result (usually `passed`).
    pub self_test: Box<str>,
    /// Whether the implementation is internal to the kernel.
    pub internal: bool,
    /// Whether FIPS mode is enabled for this implementation.
    pub fips_enabled: bool,
    /// Kernel crypto API type (e.g. `aead`, `skcipher`, or `shash`).
    pub crypto_type: Box<str>,
}

/// Reads `/proc/crypto` and returns all registered cryptographic
/// implementations.
pub fn crypto() -> Result<Vec<Crypto>> {
    let path = Path::new("/proc/crypto");
    let bytes = parse::read_file(path)?;
    let mut out = Vec::new();

    let mut name = None;
    let mut driver = None;
    let mut module = None;
    let mut priority = None;
    let mut ref_count = None;
    let mut self_test = None;
    let mut internal = None;
    let mut fips_enabled = false;
    let mut crypto_type = None;
    let mut block_line = 1;

    macro_rules! finish {
        ($line:expr) => {{
            let missing = |msg| Error::Parse {
                path: path.to_path_buf(),
                line: $line,
                msg,
            };
            out.push(Crypto {
                name: name.take().ok_or_else(|| missing("missing name"))?,
                driver: driver.take().ok_or_else(|| missing("missing driver"))?,
                module: module.take().ok_or_else(|| missing("missing module"))?,
                priority: priority.take().ok_or_else(|| missing("missing priority"))?,
                ref_count: ref_count.take().ok_or_else(|| missing("missing refcnt"))?,
                self_test: self_test
                    .take()
                    .ok_or_else(|| missing("missing selftest"))?,
                internal: internal.take().ok_or_else(|| missing("missing internal"))?,
                fips_enabled,
                crypto_type: crypto_type.take().ok_or_else(|| missing("missing type"))?,
            });
        }};
    }

    for (line_num, raw_line) in bytes.split(|&b| b == b'\n').enumerate() {
        let line = parse::trim_end(raw_line);
        if line.is_empty() {
            if name.is_some() {
                finish!(block_line);
                fips_enabled = false;
            }
            block_line = line_num + 2;
            continue;
        }

        let (key, value) = parse::parse_key_value_line(line).ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            line: line_num + 1,
            msg: "expected '<key> : <value>'",
        })?;
        let value = parse::trim_start(value);
        if value.is_empty() {
            return Err(Error::Parse {
                path: path.to_path_buf(),
                line: line_num + 1,
                msg: "empty crypto field",
            });
        }

        match key {
            b"name" => {
                name = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid name",
                        })?
                        .into(),
                )
            }
            b"driver" => {
                driver = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid driver",
                        })?
                        .into(),
                )
            }
            b"module" => {
                module = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid module",
                        })?
                        .into(),
                )
            }
            b"selftest" => {
                self_test = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid selftest",
                        })?
                        .into(),
                )
            }
            b"type" => {
                crypto_type = Some(
                    std::str::from_utf8(value)
                        .map_err(|_| Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid type",
                        })?
                        .into(),
                )
            }
            b"priority" | b"refcnt" => {
                let number = std::str::from_utf8(value)
                    .ok()
                    .and_then(|v| v.parse::<i64>().ok())
                    .ok_or_else(|| Error::Parse {
                        path: path.to_path_buf(),
                        line: line_num + 1,
                        msg: if key == b"priority" {
                            "invalid priority"
                        } else {
                            "invalid reference count"
                        },
                    })?;
                if key == b"priority" {
                    priority = Some(number);
                } else {
                    ref_count = Some(number);
                }
            }
            b"internal" | b"fips" => {
                let value = match value {
                    b"yes" => true,
                    b"no" => false,
                    _ => {
                        return Err(Error::Parse {
                            path: path.to_path_buf(),
                            line: line_num + 1,
                            msg: "invalid boolean value",
                        });
                    }
                };
                if key == b"internal" {
                    internal = Some(value);
                } else {
                    fips_enabled = value;
                }
            }
            _ => {}
        }
    }

    if name.is_some() {
        finish!(block_line);
    }
    Ok(out)
}
