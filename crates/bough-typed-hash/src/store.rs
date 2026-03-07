use crate::{TypedHash, TypedHashable};
use sha2::{Digest, Sha256};
use tracing::trace;

fn raw_hash<T: TypedHashable>(value: &T) -> T::Hash {
    let mut state = Sha256::new();
    value.hash_into(&mut state).expect("hash computation failed");
    T::Hash::from_raw(state.finalize().into())
}

/// Reverse lookup: hash → value.
///
/// Used by [`TypedHash::parse`] and [`TypedHash::from_bytes`] to validate
/// that a hash corresponds to a known value.
pub trait HashStore<T: TypedHashable> {
    fn get(&self, hash: &T::Hash) -> Option<&T>;
    fn insert(&mut self, value: T);

    fn contains(&self, hash: &T::Hash) -> bool {
        self.get(hash).is_some()
    }

    /// Minimum hex prefix length this store accepts.
    fn min_prefix_len(&self) -> usize;

    /// All hashes whose hex representation starts with `hex_prefix`.
    fn resolve_prefix(&self, hex_prefix: &str) -> Vec<&T::Hash>;
}

struct Entry<T: TypedHashable> {
    hash: T::Hash,
    value: T,
}

/// In-memory hash store.
///
/// # Example
///
/// ```
/// use bough_typed_hash::{TypedHashable, HashStore, MemoryHashStore};
///
/// #[derive(Clone, bough_typed_hash::TypedHashable)]
/// pub struct Item { value: u32 }
///
/// let mut store = MemoryHashStore::new();
/// let item = Item { value: 42 };
/// let hash = item.hash(&mut store).unwrap();
/// assert!(store.contains(&hash));
/// ```
pub struct MemoryHashStore<T: TypedHashable> {
    entries: Vec<Entry<T>>,
}

impl<T: TypedHashable> MemoryHashStore<T> {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
}

impl<T: TypedHashable> Default for MemoryHashStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TypedHashable> HashStore<T> for MemoryHashStore<T> {
    fn get(&self, hash: &T::Hash) -> Option<&T> {
        trace!("memory store: get");
        self.entries.iter()
            .find(|e| e.hash.as_bytes() == hash.as_bytes())
            .map(|e| &e.value)
    }

    fn insert(&mut self, value: T) {
        let hash = raw_hash(&value);
        trace!("memory store: insert");
        if let Some(entry) = self.entries.iter_mut().find(|e| e.hash.as_bytes() == hash.as_bytes()) {
            entry.value = value;
            entry.hash = hash;
        } else {
            self.entries.push(Entry { hash, value });
        }
    }

    fn min_prefix_len(&self) -> usize {
        2
    }

    fn resolve_prefix(&self, hex_prefix: &str) -> Vec<&T::Hash> {
        let prefix_lower = hex_prefix.to_ascii_lowercase();
        self.entries.iter()
            .filter(|e| bytes_to_hex(e.hash.as_bytes()).starts_with(&prefix_lower))
            .map(|e| &e.hash)
            .collect()
    }
}

/// Checks multiple stores in sequence.
///
/// `get` returns the first match. `resolve_prefix` collects from all (deduped).
/// `insert` delegates to the first store. `min_prefix_len` is the max across stores.
pub struct ChainStore<'a, T: TypedHashable> {
    stores: Vec<&'a mut dyn HashStore<T>>,
}

impl<'a, T: TypedHashable> ChainStore<'a, T> {
    pub fn new(stores: Vec<&'a mut dyn HashStore<T>>) -> Self {
        Self { stores }
    }
}

impl<T: TypedHashable> HashStore<T> for ChainStore<'_, T> {
    fn get(&self, hash: &T::Hash) -> Option<&T> {
        self.stores.iter()
            .find_map(|s| s.get(hash))
    }

    fn insert(&mut self, value: T) {
        if let Some(first) = self.stores.first_mut() {
            first.insert(value);
        }
    }

    fn min_prefix_len(&self) -> usize {
        self.stores.iter().map(|s| s.min_prefix_len()).max().unwrap_or(2)
    }

    fn resolve_prefix(&self, hex_prefix: &str) -> Vec<&T::Hash> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for store in &self.stores {
            for hash in store.resolve_prefix(hex_prefix) {
                if seen.insert(*hash.as_bytes()) {
                    result.push(hash);
                }
            }
        }
        result
    }
}

/// File-backed hash store. Values stored as `{hash_hex}.json` in a directory.
///
/// Requires the `disk` feature. Values must impl `Facet`.
/// Scans the directory at construction to build an index. Values are lazily
/// loaded and cached on first access via interior mutability.
#[cfg(feature = "disk")]
pub struct DiskHashStore<T: TypedHashable> {
    dir: std::path::PathBuf,
    entries: std::cell::RefCell<Vec<DiskEntry<T>>>,
}

#[cfg(feature = "disk")]
struct DiskEntry<T: TypedHashable> {
    hash: T::Hash,
    value: Option<T>,
}

#[cfg(feature = "disk")]
impl<T: TypedHashable + for<'a> facet::Facet<'a>> DiskHashStore<T> {
    /// Create a store backed by `dir`. Scans existing `*.json` files to build the index.
    pub fn new(dir: std::path::PathBuf) -> Self {
        debug!(dir = %dir.display(), "creating disk hash store");
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&dir) {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(hex) = name.strip_suffix(".json") {
                    if let Ok(bytes) = crate::hex_to_bytes(hex) {
                        entries.push(DiskEntry {
                            hash: T::Hash::from_raw(bytes),
                            value: None,
                        });
                    }
                }
            }
        }
        Self { dir, entries: std::cell::RefCell::new(entries) }
    }

    fn hash_path(&self, hash: &T::Hash) -> std::path::PathBuf {
        self.dir.join(format!("{}.json", bytes_to_hex(hash.as_bytes())))
    }
}

#[cfg(feature = "disk")]
impl<T: TypedHashable + for<'a> facet::Facet<'a>> HashStore<T> for DiskHashStore<T> {
    fn get(&self, hash: &T::Hash) -> Option<&T> {
        let mut entries = self.entries.borrow_mut();
        let idx = entries.iter().position(|e| e.hash.as_bytes() == hash.as_bytes())?;
        if entries[idx].value.is_none() {
            let path = self.hash_path(hash);
            let data = std::fs::read_to_string(&path).ok()?;
            let value: T = facet_json::from_str(&data).ok()?;
            entries[idx].value = Some(value);
        }
        drop(entries);
        // SAFETY: entry is populated above, RefCell borrow is released,
        // and the Vec is only appended to (never removed), so the pointer is stable.
        let entries = self.entries.borrow();
        entries[idx].value.as_ref().map(|v| unsafe { &*(v as *const T) })
    }

    fn insert(&mut self, value: T) {
        let hash = raw_hash(&value);
        std::fs::create_dir_all(&self.dir).expect("failed to create store directory");
        let json = facet_json::to_string(&value).expect("failed to serialize");
        std::fs::write(self.hash_path(&hash), json).expect("failed to write hash file");
        let mut entries = self.entries.borrow_mut();
        if let Some(entry) = entries.iter_mut().find(|e| e.hash.as_bytes() == hash.as_bytes()) {
            entry.value = Some(value);
        } else {
            entries.push(DiskEntry { hash, value: Some(value) });
        }
    }

    fn min_prefix_len(&self) -> usize {
        2
    }

    fn resolve_prefix(&self, hex_prefix: &str) -> Vec<&T::Hash> {
        let prefix_lower = hex_prefix.to_ascii_lowercase();
        let entries = self.entries.borrow();
        entries.iter()
            .enumerate()
            .filter(|(_, e)| bytes_to_hex(e.hash.as_bytes()).starts_with(&prefix_lower))
            .map(|(_, e)| {
                // SAFETY: entries vec is only appended to, so references to hash fields are stable
                unsafe { &*(&e.hash as *const T::Hash) }
            })
            .collect()
    }
}

pub(crate) fn bytes_to_hex(bytes: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write;
        write!(hex, "{b:02x}").unwrap();
    }
    hex
}
