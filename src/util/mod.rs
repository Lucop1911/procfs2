/// Zero-sized unit newtypes for /proc field values.
pub mod bytes;
/// Low-level byte-slice parsing primitives.
pub mod parse;

pub use bytes::{Bytes, Jiffies, Kibibytes, Milliseconds, Pages};
