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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentPattern {
    Literal(String),
    Star,
    DoubleStar,
    Pattern(Vec<PatternPart>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternPart {
    Literal(String),
    Star,
    Any,
    Class { negated: bool, ranges: Vec<(char, char)> },
    Alternates(Vec<Vec<PatternPart>>),
}

#[derive(Debug, Clone)]
pub struct Glob {
    pattern: String,
    segments: Vec<SegmentPattern>,
}

fn parse_pattern_parts(s: &str) -> Result<Vec<PatternPart>, String> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let escaped = chars.next().ok_or("trailing backslash")?;
                literal.push(escaped);
            }
            '*' => {
                if !literal.is_empty() {
                    parts.push(PatternPart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(PatternPart::Star);
            }
            '?' => {
                if !literal.is_empty() {
                    parts.push(PatternPart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(PatternPart::Any);
            }
            '[' => {
                if !literal.is_empty() {
                    parts.push(PatternPart::Literal(std::mem::take(&mut literal)));
                }
                let negated = matches!(chars.peek(), Some('!' | '^'));
                if negated {
                    chars.next();
                }
                let mut ranges = Vec::new();
                loop {
                    let lo = chars.next().ok_or("unclosed character class")?;
                    if lo == ']' {
                        break;
                    }
                    if chars.peek() == Some(&'-') {
                        chars.next();
                        let hi = chars.next().ok_or("unclosed character class")?;
                        ranges.push((lo, hi));
                    } else {
                        ranges.push((lo, lo));
                    }
                }
                parts.push(PatternPart::Class { negated, ranges });
            }
            '{' => {
                if !literal.is_empty() {
                    parts.push(PatternPart::Literal(std::mem::take(&mut literal)));
                }
                let mut depth = 1u32;
                let mut alt_str = String::new();
                while depth > 0 {
                    let ac = chars.next().ok_or("unclosed alternates")?;
                    match ac {
                        '{' => {
                            depth += 1;
                            alt_str.push(ac);
                        }
                        '}' => {
                            depth -= 1;
                            if depth > 0 {
                                alt_str.push(ac);
                            }
                        }
                        _ => alt_str.push(ac),
                    }
                }
                let branches: Result<Vec<Vec<PatternPart>>, String> = alt_str
                    .split(',')
                    .map(|branch| parse_pattern_parts(branch))
                    .collect();
                parts.push(PatternPart::Alternates(branches?));
            }
            _ => literal.push(c),
        }
    }
    if !literal.is_empty() {
        parts.push(PatternPart::Literal(literal));
    }
    Ok(parts)
}

fn parse_segment(s: &str) -> Result<SegmentPattern, String> {
    if s == "**" {
        return Ok(SegmentPattern::DoubleStar);
    }
    if s == "*" {
        return Ok(SegmentPattern::Star);
    }
    if !s.contains('*') && !s.contains('?') && !s.contains('[') && !s.contains('{') && !s.contains('\\') {
        return Ok(SegmentPattern::Literal(s.to_string()));
    }
    let parts = parse_pattern_parts(s)?;
    Ok(SegmentPattern::Pattern(parts))
}

fn match_segments(patterns: &[SegmentPattern], path: &[&str]) -> MatchResult {
    if patterns.is_empty() && path.is_empty() {
        return MatchResult::Matches;
    }
    if patterns.is_empty() {
        return MatchResult::DoesNotMatch;
    }
    if path.is_empty() {
        return MatchResult::PartialMatch;
    }

    let pat = &patterns[0];
    let seg = path[0];

    match pat {
        SegmentPattern::Literal(lit) => {
            if lit == seg {
                match_segments(&patterns[1..], &path[1..])
            } else {
                MatchResult::DoesNotMatch
            }
        }
        _ => todo!(),
    }
}

impl TryFrom<&str> for Glob {
    type Error = GlobError;

    fn try_from(pattern: &str) -> Result<Self, GlobError> {
        let segments: Result<Vec<SegmentPattern>, String> =
            pattern.split('/').map(parse_segment).collect();
        let segments = segments.map_err(|reason| GlobError::InvalidPattern {
            pattern: pattern.to_string(),
            reason,
        })?;
        Ok(Self {
            pattern: pattern.to_string(),
            segments,
        })
    }
}

impl Glob {
    pub fn is_match(&self, path: &Path) -> bool {
        match self.match_info(path) {
            MatchResult::Matches | MatchResult::MatchesAll => true,
            MatchResult::DoesNotMatch | MatchResult::PartialMatch => false,
        }
    }

    pub fn match_info(&self, path: &Path) -> MatchResult {
        let path_segments: Vec<&str> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => s.to_str(),
                _ => None,
            })
            .collect();
        match_segments(&self.segments, &path_segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn literal_matches_exact_path() {
        let glob = Glob::try_from("src/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src/main.rs")), MatchResult::Matches);
    }

    #[test]
    fn literal_does_not_match_different_path() {
        let glob = Glob::try_from("src/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src/lib.rs")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn literal_partial_matches_prefix() {
        let glob = Glob::try_from("src/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src")), MatchResult::PartialMatch);
    }

    #[test]
    fn literal_does_not_match_wrong_prefix() {
        let glob = Glob::try_from("src/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("lib")), MatchResult::DoesNotMatch);
    }
}
