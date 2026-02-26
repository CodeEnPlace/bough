use crate::TypedHash;

/// Errors from hash parsing, lookup, and I/O.
#[derive(Debug)]
pub enum HashError<H: TypedHash> {
    Io(std::io::Error),
    InvalidHex(String),
    PrefixTooShort { prefix: String, min_prefix_len: usize },
    NotFound(String),
    Ambiguous { prefix: String, matches: Vec<H> },
}

impl<H: TypedHash> std::fmt::Display for HashError<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::InvalidHex(s) => write!(f, "invalid hex: {s}"),
            Self::PrefixTooShort { prefix, min_prefix_len } => {
                write!(f, "prefix '{prefix}' too short (min {min_prefix_len})")
            }
            Self::NotFound(s) => write!(f, "hash not found: {s}"),
            Self::Ambiguous { prefix, matches } => {
                write!(f, "ambiguous prefix '{prefix}', {} matches", matches.len())
            }
        }
    }
}

impl<H: TypedHash + std::fmt::Debug> std::error::Error for HashError<H> {}

impl<H: TypedHash> From<std::io::Error> for HashError<H> {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
