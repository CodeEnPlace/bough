use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    DoesNotMatch,
    PartialMatch,
    Matches,
    MatchesAll,
}

#[derive(Debug)]
pub enum GlobError {
    InvalidPattern { pattern: String, reason: String },
}

impl std::fmt::Display for GlobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobError::InvalidPattern { pattern, reason } => {
                write!(f, "invalid glob pattern '{pattern}': {reason}")
            }
        }
    }
}

impl std::error::Error for GlobError {}

#[derive(Debug, Clone)]
pub struct Glob {
    _private: (),
}

impl TryFrom<&str> for Glob {
    type Error = GlobError;

    fn try_from(_pattern: &str) -> Result<Self, GlobError> {
        todo!()
    }
}

impl Glob {
    pub fn is_match(&self, _path: &Path) -> bool {
        todo!()
    }

    pub fn match_info(&self, _path: &Path) -> MatchResult {
        todo!()
    }
}
