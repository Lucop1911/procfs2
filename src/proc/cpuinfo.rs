use crate::error::{Error, Result};
use crate::util::parse;

/// A single CPU feature flag as reported by `/proc/cpuinfo`.
#[derive(Debug)]
pub struct CpuFlag(pub Box<str>);

/// Information about a single logical CPU core.
///
/// Parsed from one block of `/proc/cpuinfo`. Each block corresponds
/// to one logical processor (hyper-thread).
#[derive(Debug)]
pub struct CpuCore {
    /// Logical processor number.
    pub processor: u32,
    /// CPU vendor (e.g. `GenuineIntel`, `AuthenticAMD`).
    pub vendor_id: Box<str>,
    /// CPU family (x86 architecture).
    pub cpu_family: u32,
    /// CPU model number.
    pub model: u32,
    /// Human-readable model name.
    pub model_name: Box<str>,
    /// Stepping (revision) number.
    pub stepping: u32,
    /// Microcode version.
    pub microcode: Box<str>,
    /// Current clock speed in MHz.
    pub cpu_mhz: f64,
    /// L2 cache size in kilobytes.
    pub cache_size_kb: u32,
    /// Physical socket ID (for multi-socket systems).
    pub physical_id: u32,
    /// Number of logical processors sharing the same physical package.
    pub siblings: u32,
    /// Core ID within the physical package.
    pub core_id: u32,
    /// Number of physical cores in this package.
    pub cpu_cores: u32,
    /// CPU feature flags (e.g. `sse`, `avx2`, `aes`).
    pub flags: Vec<CpuFlag>,
}

/// Intermediate representation for a parsed `/proc/cpuinfo` field.
enum CpuFieldValue {
    U32(u32),
    F64(f64),
    Str(Box<str>),
    Flags(Vec<CpuFlag>),
    None,
}

fn bytes_to_box_str(b: &[u8]) -> Box<str> {
    std::str::from_utf8(b)
        .unwrap_or("")
        .to_string()
        .into_boxed_str()
}

fn parse_cpu_flags(value: &[u8]) -> Vec<CpuFlag> {
    parse::split_spaces(value)
        .into_iter()
        .filter(|f| !f.is_empty())
        .map(|f| CpuFlag(bytes_to_box_str(f)))
        .collect()
}

/// Dispatches a raw key-value pair from `/proc/cpuinfo` into a
/// typed intermediate representation.
///
/// The `cache size` field includes a unit suffix (` KB`) that is
/// stripped before parsing. All other fields are parsed directly.
fn parse_cpu_field(key: &[u8], value: &[u8]) -> CpuFieldValue {
    match key {
        b"processor" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"vendor_id" => CpuFieldValue::Str(bytes_to_box_str(value)),
        b"cpu family" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"model" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"model name" => CpuFieldValue::Str(bytes_to_box_str(value)),
        b"stepping" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"microcode" => CpuFieldValue::Str(bytes_to_box_str(value)),
        b"cpu MHz" => CpuFieldValue::F64(parse::parse_dec_f64(value).unwrap_or(0.0)),
        b"cache size" => {
            let v = parse::trim_end(value);
            let num = v.split(|&b| b == b' ').next().unwrap_or(v);
            CpuFieldValue::U32(parse::parse_dec_u32(num).unwrap_or(0))
        }
        b"physical id" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"siblings" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"core id" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"cpu cores" => CpuFieldValue::U32(parse::parse_dec_u32(value).unwrap_or(0)),
        b"flags" => CpuFieldValue::Flags(parse_cpu_flags(value)),
        _ => CpuFieldValue::None,
    }
}

/// Assembles a [`CpuCore`] from a list of parsed key-value pairs.
///
/// Missing fields default to zero or empty values. This is lenient
/// because `/proc/cpuinfo` format varies across architectures.
fn build_cpu_core(fields: &[(Box<str>, Box<str>)]) -> CpuCore {
    let mut processor = 0u32;
    let mut vendor_id = Box::from("");
    let mut cpu_family = 0u32;
    let mut model = 0u32;
    let mut model_name = Box::from("");
    let mut stepping = 0u32;
    let mut microcode = Box::from("");
    let mut cpu_mhz = 0.0f64;
    let mut cache_size_kb = 0u32;
    let mut physical_id = 0u32;
    let mut siblings = 0u32;
    let mut core_id = 0u32;
    let mut cpu_cores = 0u32;
    let mut flags = Vec::new();

    for (key, value) in fields {
        let key_bytes = key.as_bytes();
        let value_bytes = value.as_bytes();
        match parse_cpu_field(key_bytes, value_bytes) {
            CpuFieldValue::U32(v) => match key.as_ref() {
                "processor" => processor = v,
                "cpu family" => cpu_family = v,
                "model" => model = v,
                "stepping" => stepping = v,
                "cache size" => cache_size_kb = v,
                "physical id" => physical_id = v,
                "siblings" => siblings = v,
                "core id" => core_id = v,
                "cpu cores" => cpu_cores = v,
                _ => {}
            },
            CpuFieldValue::F64(v) => {
                if key.as_ref() == "cpu MHz" {
                    cpu_mhz = v;
                }
            }
            CpuFieldValue::Str(v) => match key.as_ref() {
                "vendor_id" => vendor_id = v,
                "model name" => model_name = v,
                "microcode" => microcode = v,
                _ => {}
            },
            CpuFieldValue::Flags(v) => flags = v,
            CpuFieldValue::None => {}
        }
    }

    CpuCore {
        processor,
        vendor_id,
        cpu_family,
        model,
        model_name,
        stepping,
        microcode,
        cpu_mhz,
        cache_size_kb,
        physical_id,
        siblings,
        core_id,
        cpu_cores,
        flags,
    }
}

/// Reads `/proc/cpuinfo` and returns a vector of [`CpuCore`], one
/// per logical processor.
///
/// `/proc/cpuinfo` uses a blank-line-separated block format where
/// each block describes one logical CPU. Fields within a block are
/// key-value pairs (`key : value`).
pub fn cpuinfo() -> Result<Vec<CpuCore>> {
    let bytes = parse::read_file(std::path::Path::new("/proc/cpuinfo"))?;
    let path = std::path::PathBuf::from("/proc/cpuinfo");

    let mut cores = Vec::new();
    let mut current_fields: Vec<(Box<str>, Box<str>)> = Vec::new();

    for (line_num, line) in bytes.split(|&b| b == b'\n').enumerate() {
        if line.is_empty() {
            if !current_fields.is_empty() {
                cores.push(build_cpu_core(&current_fields));
                current_fields.clear();
            }
            continue;
        }

        let (key, value) = parse::parse_key_value_line(line).ok_or_else(|| Error::Parse {
            path: path.clone(),
            line: line_num + 1,
            msg: "invalid key-value line",
        })?;

        let key_str = std::str::from_utf8(key)
            .map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid utf8 in key",
            })?
            .to_string()
            .into_boxed_str();

        let value_str = std::str::from_utf8(value)
            .map_err(|_| Error::Parse {
                path: path.clone(),
                line: line_num + 1,
                msg: "invalid utf8 in value",
            })?
            .to_string()
            .into_boxed_str();

        current_fields.push((key_str, value_str));
    }

    if !current_fields.is_empty() {
        cores.push(build_cpu_core(&current_fields));
    }

    if cores.is_empty() {
        return Err(Error::Parse {
            path,
            line: 0,
            msg: "no CPU cores found",
        });
    }

    Ok(cores)
}
