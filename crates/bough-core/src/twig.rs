use crate::file::{Error, Root};
use ignore::overrides::OverrideBuilder;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

// bough[impl file.twig]
#[derive(Debug, Clone, PartialEq, Eq, Hash, facet::Facet)]
pub struct Twig(String);

impl Twig {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if path.is_absolute() {
            return Err(Error::TwigMustBeRelative(path));
        }
        let s = path
            .to_str()
            .ok_or_else(|| Error::TwigNotUtf8(path.clone()))?;
        Ok(Self(s.to_owned()))
    }

    pub fn path(&self) -> &Path {
        Path::new(&self.0)
    }
}

// bough[impl twig.iter.root]
// bough[impl twig.iter]
#[derive(Clone, PartialEq, Debug)]
pub struct TwigsIterBuilder {
    // bough[impl twig.iter.include]
    include: Vec<String>,
    // bough[impl twig.iter.exclude]
    exclude: Vec<String>,
}

impl TwigsIterBuilder {
    pub fn new() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn with_include_glob(mut self, pattern: &str) -> Self {
        self.include.push(pattern.to_string());
        self
    }

    pub fn with_exclude_glob(mut self, pattern: &str) -> Self {
        self.exclude.push(pattern.to_string());
        self
    }

    // bough[impl twig.iter.new]
    pub fn build<'a, R: Root>(self, root: &'a R) -> TwigsIter<'a, R> {
        let root_path = root.path();
        debug!(
            root = %root_path.display(),
            includes = ?self.include,
            excludes = ?self.exclude,
            "building twigs iterator"
        );

        // bough[impl twig.iter.include.empty]
        if self.include.is_empty() {
            return TwigsIter {
                root,
                walker: None,
            };
        }

        let mut overrides = OverrideBuilder::new(root_path);
        // bough[impl twig.iter.include.match]
        for pat in &self.include {
            overrides.add(pat).expect("invalid include glob");
        }
        // bough[impl twig.iter.exclude.match]
        for pat in &self.exclude {
            overrides
                .add(&format!("!{pat}"))
                .expect("invalid exclude glob");
        }
        let overrides = overrides.build().expect("failed to build overrides");

        let walker = {
            let mut builder = ignore::WalkBuilder::new(root_path);
            builder
                .standard_filters(false)
                .overrides(overrides)
                .sort_by_file_path(|a, b| a.cmp(b));
            builder.build()
        };

        TwigsIter {
            root,
            walker: Some(walker),
        }
    }
}

pub struct TwigsIter<'a, R: Root> {
    root: &'a R,
    walker: Option<ignore::Walk>,
}

impl<R: Root> Iterator for TwigsIter<'_, R> {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        let walker = self.walker.as_mut()?;
        loop {
            let entry = walker.next()?.ok()?;
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
            let rel = entry.path().strip_prefix(self.root.path()).ok()?;
            trace!(twig = %rel.display(), "yielding twig");
            return Twig::new(rel.to_path_buf()).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::TestRoot;

    // bough[verify file.twig]
    #[test]
    fn twig_accepts_relative_path() {
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        assert_eq!(twig.path(), Path::new("src/main.rs"));
    }

    // bough[verify file.twig]
    #[test]
    fn twig_rejects_absolute_path() {
        assert!(matches!(
            Twig::new(PathBuf::from("/absolute/path.rs")),
            Err(Error::TwigMustBeRelative(_))
        ));
    }

    // bough[verify twig.iter.root]
    // bough[verify twig.iter.new]
    // bough[verify twig.iter]
    #[test]
    fn iter_takes_root_and_yields_twigs() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        let twigs: Vec<_> = TwigsIterBuilder::new()
            .with_include_glob("*.txt")
            .build(&root)
            .collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }

    fn make_test_tree(dir: &std::path::Path) {
        for p in &[
            "src/index.js",
            "src/utils.js",
            "src/style.css",
            "test/index.test.js",
            "build/output.js",
            "README.md",
        ] {
            let full = dir.join(p);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, "content").unwrap();
        }
    }

    fn sorted_twigs<'a, R: Root>(iter: TwigsIter<'a, R>) -> Vec<PathBuf> {
        let mut twigs: Vec<PathBuf> = iter.map(|t| t.path().to_path_buf()).collect();
        // twigs.sort();
        twigs
    }

    // bough[verify twig.iter]
    // bough[verify twig.iter.include.match]
    #[test]
    fn iter_includes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("src/**/*.js")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/index.js"), PathBuf::from("src/utils.js")]
        );
    }

    // bough[verify twig.iter.include.match]
    #[test]
    fn iter_includes_multiple_globs() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("src/**/*.js")
                .with_include_glob("**/*.md")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
            ]
        );
    }

    // bough[verify twig.iter.include.empty]
    #[test]
    fn iter_empty_include_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<_> = TwigsIterBuilder::new().build(&root).collect();
        assert!(twigs.is_empty());
    }

    // bough[verify twig.iter.exclude.match]
    #[test]
    fn iter_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("**/*.js")
                .with_exclude_glob("build/**")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
                PathBuf::from("test/index.test.js"),
            ]
        );
    }
}
