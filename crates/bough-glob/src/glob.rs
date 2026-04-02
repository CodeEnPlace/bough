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
    if s.is_empty() {
        return Ok(SegmentPattern::Literal(String::new()));
    }
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

fn match_pattern_parts(parts: &[PatternPart], s: &str) -> bool {
    match_pattern_inner(parts, s.as_bytes(), 0)
}

fn match_pattern_inner(parts: &[PatternPart], s: &[u8], pos: usize) -> bool {
    if parts.is_empty() {
        return pos == s.len();
    }

    let part = &parts[0];
    let rest = &parts[1..];

    match part {
        PatternPart::Literal(lit) => {
            let bytes = lit.as_bytes();
            if s[pos..].starts_with(bytes) {
                match_pattern_inner(rest, s, pos + bytes.len())
            } else {
                false
            }
        }
        PatternPart::Star => {
            for i in pos..=s.len() {
                if match_pattern_inner(rest, s, i) {
                    return true;
                }
            }
            false
        }
        PatternPart::Any => {
            if pos < s.len() {
                match_pattern_inner(rest, s, pos + 1)
            } else {
                false
            }
        }
        PatternPart::Class { negated, ranges } => {
            if pos >= s.len() {
                return false;
            }
            let c = s[pos] as char;
            let in_range = ranges.iter().any(|&(lo, hi)| c >= lo && c <= hi);
            if in_range != *negated {
                match_pattern_inner(rest, s, pos + 1)
            } else {
                false
            }
        }
        PatternPart::Alternates(branches) => {
            branches.iter().any(|branch| {
                let mut combined = branch.clone();
                combined.extend_from_slice(rest);
                match_pattern_inner(&combined, s, pos)
            })
        }
    }
}

fn best_match(a: MatchResult, b: MatchResult) -> MatchResult {
    fn rank(m: &MatchResult) -> u8 {
        match m {
            MatchResult::DoesNotMatch => 0,
            MatchResult::PartialMatch => 1,
            MatchResult::Matches => 2,
            MatchResult::MatchesAll => 3,
        }
    }
    if rank(&a) >= rank(&b) { a } else { b }
}

fn match_segments(patterns: &[SegmentPattern], path: &[&str]) -> MatchResult {
    if patterns.is_empty() && path.is_empty() {
        return MatchResult::Matches;
    }
    if patterns.is_empty() {
        return MatchResult::DoesNotMatch;
    }
    if path.is_empty() {
        if patterns.iter().all(|p| match p {
            SegmentPattern::DoubleStar => true,
            _ => false,
        }) {
            return MatchResult::MatchesAll;
        }
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
        SegmentPattern::Star => match_segments(&patterns[1..], &path[1..]),
        SegmentPattern::DoubleStar => {
            let rest_patterns = &patterns[1..];
            if rest_patterns.is_empty() {
                return MatchResult::MatchesAll;
            }
            let mut best = match_segments(rest_patterns, path);
            for i in 1..=path.len() {
                let result = match_segments(rest_patterns, &path[i..]);
                best = best_match(best, result);
            }
            best
        }
        SegmentPattern::Pattern(parts) => {
            if match_pattern_parts(parts, seg) {
                match_segments(&patterns[1..], &path[1..])
            } else {
                MatchResult::DoesNotMatch
            }
        }
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

    #[test]
    fn star_matches_any_single_segment() {
        let glob = Glob::try_from("src/*/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src/foo/main.rs")), MatchResult::Matches);
    }

    #[test]
    fn star_does_not_match_multiple_segments() {
        let glob = Glob::try_from("src/*/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src/foo/bar/main.rs")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn star_partial_matches_prefix() {
        let glob = Glob::try_from("src/*/main.rs").unwrap();
        assert_eq!(glob.match_info(Path::new("src/foo")), MatchResult::PartialMatch);
    }

    #[test]
    fn double_star_matches_deep_path() {
        let glob = Glob::try_from("src/**/*.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/a/b/c/main.js")), MatchResult::Matches);
    }

    #[test]
    fn double_star_matches_immediate_child() {
        let glob = Glob::try_from("src/**/*.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/main.js")), MatchResult::Matches);
    }

    #[test]
    fn double_star_wrong_extension_is_partial() {
        let glob = Glob::try_from("src/**/*.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/main.rs")), MatchResult::PartialMatch);
    }

    #[test]
    fn double_star_partial_matches_dir() {
        let glob = Glob::try_from("src/**/*.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/a/b")), MatchResult::PartialMatch);
    }

    #[test]
    fn double_star_does_not_match_wrong_prefix() {
        let glob = Glob::try_from("src/**/*.js").unwrap();
        assert_eq!(glob.match_info(Path::new("lib/main.js")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn double_star_alone_matches_everything() {
        let glob = Glob::try_from("**").unwrap();
        assert_eq!(glob.match_info(Path::new("anything/at/all")), MatchResult::MatchesAll);
    }

    #[test]
    fn double_star_alone_partial_matches_empty() {
        let glob = Glob::try_from("**").unwrap();
        assert_eq!(glob.match_info(Path::new("")), MatchResult::MatchesAll);
    }

    #[test]
    fn question_mark_matches_single_char() {
        let glob = Glob::try_from("src/?.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/a.js")), MatchResult::Matches);
    }

    #[test]
    fn question_mark_does_not_match_multiple_chars() {
        let glob = Glob::try_from("src/?.js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/ab.js")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn char_class_matches() {
        let glob = Glob::try_from("src/[abc].js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/a.js")), MatchResult::Matches);
    }

    #[test]
    fn char_class_does_not_match() {
        let glob = Glob::try_from("src/[abc].js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/d.js")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn negated_char_class() {
        let glob = Glob::try_from("src/[!abc].js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/d.js")), MatchResult::Matches);
        assert_eq!(glob.match_info(Path::new("src/a.js")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn alternates_match() {
        let glob = Glob::try_from("src/*.{js,ts}").unwrap();
        assert_eq!(glob.match_info(Path::new("src/main.js")), MatchResult::Matches);
        assert_eq!(glob.match_info(Path::new("src/main.ts")), MatchResult::Matches);
        assert_eq!(glob.match_info(Path::new("src/main.rs")), MatchResult::DoesNotMatch);
    }

    #[test]
    fn char_range_matches() {
        let glob = Glob::try_from("src/[a-z].js").unwrap();
        assert_eq!(glob.match_info(Path::new("src/m.js")), MatchResult::Matches);
        assert_eq!(glob.match_info(Path::new("src/A.js")), MatchResult::DoesNotMatch);
    }
}
