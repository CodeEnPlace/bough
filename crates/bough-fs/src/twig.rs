use crate::file::Error;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, facet::Facet)]
pub struct Twig(String);

impl Twig {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if path.is_absolute() {
            return Err(Error::TwigMustBeRelative(path));
        }
        let s = path
            .to_str()
            .ok_or_else(|| Error::TwigNotUtf8(path.clone()))?;
        Ok(Self(s.replace('\\', "/")))
    }

    pub fn path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twig_accepts_relative_path() {
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        assert_eq!(twig.path(), Path::new("src/main.rs"));
    }

    #[test]
    #[cfg(unix)]
    fn twig_rejects_absolute_path() {
        match Twig::new(PathBuf::from("/absolute/path.rs")) {
            Err(Error::TwigMustBeRelative(_)) => {}
            other => panic!("expected TwigMustBeRelative, got {:?}", other),
        }
    }
}
