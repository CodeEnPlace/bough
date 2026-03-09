use crate::{HashError, TypedHash, bytes_to_hex};
use tracing::{debug, warn};

/// Intermediate type for deserializing hashes without validation.
///
/// `TypedHash` does not implement deserialization — deserialized hashes would
/// bypass validation. Use `UnvalidatedHash` as the target, then
/// call [`.validate()`](UnvalidatedHash::validate) with a slice of known hashes.
///
/// # Example
///
/// ```
/// use bough_typed_hash::{UnvalidatedHash, TypedHash, TypedHashable};
///
/// #[derive(Clone, bough_typed_hash::TypedHashable)]
/// pub struct Record { id: u32 }
///
/// let record = Record { id: 1 };
/// let hash = record.hash().unwrap();
/// let hex = hash.to_string();
///
/// let unvalidated = UnvalidatedHash::new(hex);
/// let validated = unvalidated.validate(&[hash]).unwrap();
/// assert_eq!(validated.as_bytes(), hash.as_bytes());
/// ```
#[derive(Debug, Clone, facet::Facet)]
#[facet(transparent)]
pub struct UnvalidatedHash(String);

impl UnvalidatedHash {
    pub fn new(hex: String) -> Self {
        Self(hex)
    }

    /// Validate against a set of known hashes, supporting both full hex and unique prefixes.
    pub fn validate<H: TypedHash>(self, known: &[H]) -> Result<H, HashError<H>> {
        debug!(hex_len = self.0.len(), "validating unvalidated hash");
        let s = &self.0;

        if s.chars().any(|c| !c.is_ascii_hexdigit()) {
            return Err(HashError::InvalidHex(s.clone()));
        }

        if s.len() == 64 {
            let bytes = crate::hex_to_bytes(s).map_err(HashError::InvalidHex)?;
            let target = H::from_raw(bytes);
            return if known.iter().any(|h| h.as_bytes() == target.as_bytes()) {
                Ok(target)
            } else {
                Err(HashError::NotFound(s.clone()))
            };
        }

        let prefix_lower = s.to_ascii_lowercase();
        let matches: Vec<H> = known
            .iter()
            .filter(|h| bytes_to_hex(h.as_bytes()).starts_with(&prefix_lower))
            .map(|h| H::from_raw(*h.as_bytes()))
            .collect();

        match matches.len() {
            0 => Err(HashError::NotFound(s.clone())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => {
                warn!(prefix = %s, count = matches.len(), "ambiguous hash prefix");
                Err(HashError::Ambiguous {
                    prefix: s.clone(),
                    matches,
                })
            }
        }
    }
}
