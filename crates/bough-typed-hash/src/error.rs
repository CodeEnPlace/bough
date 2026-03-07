use crate::TypedHash;

#[derive(Debug)]
pub enum HashError<H: TypedHash> {
    InvalidHex(String),
    NotFound(String),
    Ambiguous { prefix: String, matches: Vec<H> },
}

impl<H: TypedHash> std::fmt::Display for HashError<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHex(s) => write!(f, "invalid hex: {s}"),
            Self::NotFound(s) => write!(f, "hash not found: {s}"),
            Self::Ambiguous { prefix, matches } => {
                write!(f, "ambiguous prefix '{prefix}', {} matches", matches.len())
            }
        }
    }
}

impl<H: TypedHash + std::fmt::Debug> std::error::Error for HashError<H> {}
