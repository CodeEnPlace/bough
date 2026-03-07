use crate::{HashError, HashStore, TypedHash, TypedHashable};
use tracing::debug;

/// Intermediate type for deserializing hashes without store validation.
///
/// `TypedHash` does not implement deserialization — deserialized hashes would
/// bypass store validation. Use `UnvalidatedHash` as the target, then
/// call [`.validate()`](UnvalidatedHash::validate) with a store.
///
/// # Example
///
/// ```
/// use bough_typed_hash::{UnvalidatedHash, TypedHash, TypedHashable, HashStore, MemoryHashStore};
///
/// #[derive(Clone, bough_typed_hash::TypedHashable)]
/// pub struct Record { id: u32 }
///
/// let mut store = MemoryHashStore::new();
/// let record = Record { id: 1 };
/// let hash = record.hash(&mut store).unwrap();
/// let hex = hash.to_string();
///
/// let unvalidated = UnvalidatedHash::new(hex);
/// let validated = unvalidated.validate::<Record>(&store).unwrap();
/// assert_eq!(validated.as_bytes(), hash.as_bytes());
/// ```
#[derive(Debug, Clone, facet::Facet)]
#[facet(transparent)]
pub struct UnvalidatedHash(String);

impl UnvalidatedHash {
    pub fn new(hex: String) -> Self {
        Self(hex)
    }

    /// Validate against a store, supporting both full hex and unique prefixes.
    pub fn validate<T: TypedHashable>(
        self,
        store: &dyn HashStore<T>,
    ) -> Result<T::Hash, HashError<T::Hash>> {
        debug!(hex_len = self.0.len(), "validating unvalidated hash");
        T::Hash::parse::<T>(&self.0, store)
    }
}
