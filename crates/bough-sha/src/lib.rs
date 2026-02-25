use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub use bough_sha_derive::ShaHashable;

#[doc(hidden)]
pub use crate as bough_sha;

pub type ShaState = Sha256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaHash([u8; 16]);

impl ShaHash {
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Serialize for ShaHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ShaHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = <&str>::deserialize(deserializer)?;
        hex.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for ShaHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for ShaHash {
    type Err = String;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        let arr: [u8; 16] = bytes
            .try_into()
            .map_err(|_| "expected 32 hex chars".to_string())?;
        Ok(Self(arr))
    }
}

pub trait ShaHashable {
    fn sha_hash_into(&self, state: &mut ShaState);

    fn sha_hash(&self) -> ShaHash {
        let mut state = Sha256::new();
        self.sha_hash_into(&mut state);
        let full: [u8; 32] = state.finalize().into();
        let mut truncated = [0u8; 16];
        truncated.copy_from_slice(&full[..16]);
        ShaHash(truncated)
    }
}

impl ShaHashable for str {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.len().to_le_bytes());
        state.update(self.as_bytes());
    }
}

impl ShaHashable for String {
    fn sha_hash_into(&self, state: &mut ShaState) {
        self.as_str().sha_hash_into(state);
    }
}

impl ShaHashable for bool {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update([*self as u8]);
    }
}

impl ShaHashable for u8 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update([*self]);
    }
}

impl ShaHashable for u16 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for u32 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for u64 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for usize {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update((*self as u64).to_le_bytes());
    }
}

impl ShaHashable for i8 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for i16 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for i32 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for i64 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for f32 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl ShaHashable for f64 {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.to_le_bytes());
    }
}

impl<T: ShaHashable> ShaHashable for Vec<T> {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.len().to_le_bytes());
        for item in self {
            item.sha_hash_into(state);
        }
    }
}

impl<T: ShaHashable> ShaHashable for Option<T> {
    fn sha_hash_into(&self, state: &mut ShaState) {
        match self {
            None => state.update([0]),
            Some(v) => {
                state.update([1]);
                v.sha_hash_into(state);
            }
        }
    }
}

impl<T: ShaHashable> ShaHashable for &T {
    fn sha_hash_into(&self, state: &mut ShaState) {
        (*self).sha_hash_into(state);
    }
}

impl<T: ShaHashable> ShaHashable for [T] {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.len().to_le_bytes());
        for item in self {
            item.sha_hash_into(state);
        }
    }
}

impl ShaHashable for ShaHash {
    fn sha_hash_into(&self, state: &mut ShaState) {
        state.update(self.0);
    }
}

impl ShaHashable for Path {
    fn sha_hash_into(&self, state: &mut ShaState) {
        self.to_string_lossy().as_ref().sha_hash_into(state);
    }
}

impl ShaHashable for PathBuf {
    fn sha_hash_into(&self, state: &mut ShaState) {
        self.as_path().sha_hash_into(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!("hello".sha_hash(), "hello".sha_hash());
    }

    #[test]
    fn different_inputs_differ() {
        assert_ne!("hello".sha_hash(), "world".sha_hash());
    }

    #[test]
    fn display_is_hex() {
        let h = "hello".sha_hash();
        let s = h.to_string();
        assert_eq!(s.len(), 32);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn roundtrip_parse() {
        let h = "test".sha_hash();
        let parsed: ShaHash = h.to_string().parse().unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn serde_roundtrip() {
        let h = "test".sha_hash();
        let json = serde_json::to_string(&h).unwrap();
        let parsed: ShaHash = serde_json::from_str(&json).unwrap();
        assert_eq!(h, parsed);
        assert!(json.starts_with('"') && json.ends_with('"'));
    }

    #[test]
    fn derive_struct() {
        #[derive(ShaHashable)]
        struct Foo {
            x: u32,
            y: String,
        }

        let a = Foo { x: 1, y: "a".into() };
        let b = Foo { x: 1, y: "a".into() };
        let c = Foo { x: 2, y: "a".into() };
        assert_eq!(a.sha_hash(), b.sha_hash());
        assert_ne!(a.sha_hash(), c.sha_hash());
    }

    #[test]
    fn derive_tuple_struct() {
        #[derive(ShaHashable)]
        struct Bar(u32, String);

        let a = Bar(1, "a".into());
        let b = Bar(1, "a".into());
        let c = Bar(1, "b".into());
        assert_eq!(a.sha_hash(), b.sha_hash());
        assert_ne!(a.sha_hash(), c.sha_hash());
    }

    #[test]
    fn derive_enum() {
        #[derive(ShaHashable)]
        enum E {
            A,
            B(u32),
            C { x: String },
        }

        assert_eq!(E::A.sha_hash(), E::A.sha_hash());
        assert_ne!(E::A.sha_hash(), E::B(0).sha_hash());
        assert_eq!(E::B(42).sha_hash(), E::B(42).sha_hash());
        assert_ne!(E::B(1).sha_hash(), E::B(2).sha_hash());
        assert_eq!(
            E::C { x: "hi".into() }.sha_hash(),
            E::C { x: "hi".into() }.sha_hash()
        );
    }

    #[test]
    fn derive_nested() {
        #[derive(ShaHashable)]
        struct Inner {
            v: u32,
        }

        #[derive(ShaHashable)]
        struct Outer {
            inner: Inner,
            label: String,
        }

        let a = Outer { inner: Inner { v: 1 }, label: "x".into() };
        let b = Outer { inner: Inner { v: 1 }, label: "x".into() };
        let c = Outer { inner: Inner { v: 2 }, label: "x".into() };
        assert_eq!(a.sha_hash(), b.sha_hash());
        assert_ne!(a.sha_hash(), c.sha_hash());
    }

    #[test]
    fn option_none_vs_some_differ() {
        let none: Option<u32> = None;
        let some = Some(0u32);
        assert_ne!(none.sha_hash(), some.sha_hash());
    }

    #[test]
    fn vec_order_matters() {
        let a = vec![1u32, 2];
        let b = vec![2u32, 1];
        assert_ne!(a.sha_hash(), b.sha_hash());
    }

    #[test]
    fn string_length_prefix_prevents_collisions() {
        #[derive(ShaHashable)]
        struct Pair { a: String, b: String }

        let x = Pair { a: "ab".into(), b: "c".into() };
        let y = Pair { a: "a".into(), b: "bc".into() };
        assert_ne!(x.sha_hash(), y.sha_hash());
    }
}
