//! Typed SHA-256 hashing: every hash carries its type.
//!
//! Three traits form the core:
//! - [`HashInto`] — contributes bytes to a hash
//! - [`TypedHash`] — a typed 32-byte hash value
//! - [`TypedHashable`] — produces a [`TypedHash`] from data
//!
//! [`HashStore`] provides reverse lookup for hash validation.
//!
//! # Example
//!
//! ```
//! use bough_typed_hash::{HashInto, TypedHash, TypedHashable, MemoryHashStore, HashStore};
//!
//! #[derive(Clone, bough_typed_hash::TypedHashable)]
//! pub struct Config {
//!     name: String,
//!     version: u32,
//! }
//!
//! let mut store = MemoryHashStore::new();
//! let cfg = Config { name: "app".into(), version: 1 };
//! let hash = cfg.hash(&mut store).unwrap();
//! assert!(store.contains(&hash));
//! ```

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{trace, warn};

pub use bough_typed_hash_derive::{HashInto, TypedHash, TypedHashable};

#[doc(hidden)]
pub use crate as bough_typed_hash;

#[doc(hidden)]
pub use sha2;

/// SHA-256 hasher state.
pub type ShaState = Sha256;

mod error;
mod store;
mod unvalidated;

pub use error::HashError;
pub use store::{ChainStore, HashStore, MemoryHashStore};
pub use unvalidated::UnvalidatedHash;

#[cfg(feature = "disk")]
pub use store::DiskHashStore;

/// Contributes bytes to a SHA-256 hash.
///
/// Primitives and building blocks implement this trait. Types that produce
/// standalone hashes should also implement [`TypedHashable`].
pub trait HashInto {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error>;
}

/// A typed 32-byte SHA-256 hash.
///
/// Not directly constructable from external input — use [`TypedHash::parse`]
/// or [`TypedHash::from_bytes`] with a [`HashStore`] to validate existence.
pub trait TypedHash: Sized {
    #[doc(hidden)]
    fn from_raw(bytes: [u8; 32]) -> Self;

    fn as_bytes(&self) -> &[u8; 32];

    /// Parse a hex string (full 64-char or unique prefix, min 2 chars) and
    /// validate against a store.
    fn parse<T: TypedHashable<Hash = Self>>(
        s: &str,
        store: &dyn HashStore<T>,
    ) -> Result<Self, HashError<Self>> {
        trace!(input_len = s.len(), "parsing hash");
        if s.len() == 64 {
            let bytes = hex_to_bytes(s).map_err(|e| HashError::InvalidHex(e))?;
            return Self::from_bytes(bytes, store);
        }

        if s.len() < store.min_prefix_len() {
            return Err(HashError::PrefixTooShort {
                prefix: s.to_string(),
                min_prefix_len: store.min_prefix_len(),
            });
        }

        if s.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err(HashError::InvalidHex(s.to_string()));
        }

        let matches: Vec<Self> = store.resolve_prefix(s)
            .into_iter()
            .map(|h| Self::from_raw(*h.as_bytes()))
            .collect();

        match matches.len() {
            0 => Err(HashError::NotFound(s.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => {
                warn!(prefix = s, count = matches.len(), "ambiguous hash prefix");
                Err(HashError::Ambiguous {
                    prefix: s.to_string(),
                    matches,
                })
            }
        }
    }

    /// Validate raw bytes against a store.
    fn from_bytes<T: TypedHashable<Hash = Self>>(
        bytes: [u8; 32],
        store: &dyn HashStore<T>,
    ) -> Result<Self, HashError<Self>> {
        let hash = Self::from_raw(bytes);
        if store.contains(&hash) {
            Ok(hash)
        } else {
            let mut hex = String::with_capacity(64);
            for b in &bytes {
                use std::fmt::Write;
                write!(hex, "{b:02x}").unwrap();
            }
            Err(HashError::NotFound(hex))
        }
    }
}

/// Data that produces a [`TypedHash`].
///
/// Only root objects addressed by hash implement this. Building blocks
/// implement [`HashInto`] only. Calling [`hash`](TypedHashable::hash)
/// computes the hash and inserts the value into the provided store.
pub trait TypedHashable: HashInto + Clone + Sized {
    type Hash: TypedHash;

    fn hash(&self, store: &mut dyn HashStore<Self>) -> Result<Self::Hash, std::io::Error> {
        let mut state = Sha256::new();
        self.hash_into(&mut state)?;
        let bytes: [u8; 32] = state.finalize().into();
        let hash = Self::Hash::from_raw(bytes);
        store.insert(self.clone());
        Ok(hash)
    }
}

pub fn hex_to_bytes(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 {
        return Err(format!("expected 64 hex chars, got {}", hex.len()));
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}

// --- Primitive HashInto impls ---

impl HashInto for str {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        state.update(self.len().to_le_bytes());
        state.update(self.as_bytes());
        Ok(())
    }
}

impl HashInto for String {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        self.as_str().hash_into(state)
    }
}

impl HashInto for bool {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        state.update([*self as u8]);
        Ok(())
    }
}

macro_rules! impl_hash_into_int {
    ($($t:ty),*) => {
        $(impl HashInto for $t {
            fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
                state.update(self.to_le_bytes());
                Ok(())
            }
        })*
    };
}

impl_hash_into_int!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

impl HashInto for usize {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        state.update((*self as u64).to_le_bytes());
        Ok(())
    }
}

impl<T: HashInto> HashInto for Vec<T> {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        state.update(self.len().to_le_bytes());
        for item in self {
            item.hash_into(state)?;
        }
        Ok(())
    }
}

impl<T: HashInto> HashInto for Option<T> {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        match self {
            None => {
                state.update([0]);
                Ok(())
            }
            Some(v) => {
                state.update([1]);
                v.hash_into(state)
            }
        }
    }
}

impl<T: HashInto + ?Sized> HashInto for &T {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        (*self).hash_into(state)
    }
}

impl<T: HashInto> HashInto for [T] {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        state.update(self.len().to_le_bytes());
        for item in self {
            item.hash_into(state)?;
        }
        Ok(())
    }
}

impl HashInto for Path {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        let contents = std::fs::read(self)?;
        state.update(contents.len().to_le_bytes());
        state.update(&contents);
        Ok(())
    }
}

impl HashInto for PathBuf {
    fn hash_into(&self, state: &mut ShaState) -> Result<(), std::io::Error> {
        self.as_path().hash_into(state)
    }
}

#[cfg(test)]
mod tests;
