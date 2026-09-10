#![cfg(feature = "macros")]

use procfs2::macros::ProcKeyValue;
use procfs2::util::Bytes;
use procfs2::util::Kibibytes;
use procfs2::util::parse::ParseFromBytes;

#[derive(ProcKeyValue)]
struct Basic {
    #[proc_key = "A"]
    a: u64,
    #[proc_key = "B"]
    b: u64,
}

#[derive(ProcKeyValue)]
struct Units {
    #[proc_key = "MemTotal"]
    mem_total: Kibibytes,
    #[proc_key = "ReadBytes"]
    read_bytes: Bytes,
}

#[derive(ProcKeyValue)]
struct Optionals {
    #[proc_key = "Present"]
    present: u64,
    #[proc_key = "Missing"]
    missing: Option<u64>,
}

#[derive(ProcKeyValue)]
struct NumericTypes {
    #[proc_key = "U32Val"]
    u32_val: u32,
    #[proc_key = "I64Val"]
    i64_val: i64,
    #[proc_key = "F64Val"]
    f64_val: f64,
}

#[derive(ProcKeyValue)]
struct MemInfo {
    #[proc_key = "MemTotal"]
    total: Kibibytes,
    #[proc_key = "MemFree"]
    free: Kibibytes,
    #[proc_key = "MemAvailable"]
    available: Option<Kibibytes>,
    #[proc_key = "Buffers"]
    buffers: Kibibytes,
}

#[test]
fn parse_u64_fields() {
    let data = b"A: 42\nB: 100\n";
    let v = Basic::parse_from_bytes(data).unwrap();
    assert_eq!(v.a, 42);
    assert_eq!(v.b, 100);
}

#[test]
fn parse_missing_key_uses_default() {
    let data = b"A: 7\n";
    let v = Basic::parse_from_bytes(data).unwrap();
    assert_eq!(v.a, 7);
    assert_eq!(v.b, 0);
}

#[test]
fn parse_kibibytes_with_suffix() {
    let data = b"MemTotal: 16384008 kB\nReadBytes: 2048\n";
    let v = Units::parse_from_bytes(data).unwrap();
    assert_eq!(v.mem_total.0, 16384008);
    assert_eq!(v.read_bytes.0, 2048);
}

#[test]
fn parse_optional_present() {
    let data = b"Present: 10\nMissing: 20\n";
    let v = Optionals::parse_from_bytes(data).unwrap();
    assert_eq!(v.present, 10);
    assert_eq!(v.missing, Some(20));
}

#[test]
fn parse_optional_missing() {
    let data = b"Present: 10\n";
    let v = Optionals::parse_from_bytes(data).unwrap();
    assert_eq!(v.present, 10);
    assert_eq!(v.missing, None);
}

#[test]
fn parse_numeric_types() {
    let data = b"U32Val: 999\nI64Val: -42\nF64Val: 2.71\n";
    let v = NumericTypes::parse_from_bytes(data).unwrap();
    assert_eq!(v.u32_val, 999);
    assert_eq!(v.i64_val, -42);
    assert!((v.f64_val - 2.71).abs() < f64::EPSILON);
}

#[test]
fn parse_empty_input() {
    let v = Basic::parse_from_bytes(b"").unwrap();
    assert_eq!(v.a, 0);
    assert_eq!(v.b, 0);
}

#[test]
fn parse_blank_lines_ignored() {
    let data = b"\nA: 5\n\nB: 9\n\n";
    let v = Basic::parse_from_bytes(data).unwrap();
    assert_eq!(v.a, 5);
    assert_eq!(v.b, 9);
}

#[test]
fn parse_unknown_keys_ignored() {
    let data = b"Unknown: 0\nA: 11\nAlsoUnknown: 0\nB: 22\n";
    let v = Basic::parse_from_bytes(data).unwrap();
    assert_eq!(v.a, 11);
    assert_eq!(v.b, 22);
}

#[test]
fn parse_real_meminfo() {
    let bytes = procfs2::util::parse::read_file(std::path::Path::new("/proc/meminfo")).unwrap();
    let v = MemInfo::parse_from_bytes(&bytes).unwrap();
    assert!(v.total.0 > 0);
    assert!(v.free.0 > 0);
    assert!(v.total.0 >= v.free.0);
}
