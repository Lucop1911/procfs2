//! Serde serialization implementations for procfs2 types.
//! 
//! This module provides manual Serialize and Deserialize implementations
//! for types that benefit from custom serialization logic.
//! 
//! The unit newtypes (Bytes, Kibibytes, etc.) serialize to/from their
//! underlying `u64` value rather than as objects with a named field.
//! 
//! # Usage
//! 
//! Enable the `serde` feature and use `#[derive(Serialize, Deserialize)]`
//! on your types along with `#[serde(with = "procfs2::serde_impls")]`.

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Serialize unit newtypes as their underlying u64 value.
#[cfg(feature = "serde")]
macro_rules! impl_serialize_newtype {
    ($ty:ty) => {
        #[cfg(feature = "serde")]
        impl Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_u64(self.0)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(Self(u64::deserialize(deserializer)?))
            }
        }
    };
}

use crate::util::{Bytes, Jiffies, Kibibytes, Milliseconds, Pages};

impl_serialize_newtype!(Bytes);
impl_serialize_newtype!(Kibibytes);
impl_serialize_newtype!(Pages);
impl_serialize_newtype!(Jiffies);
impl_serialize_newtype!(Milliseconds);