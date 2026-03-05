use crate::LanguageId;
use crate::file::{Error, Root, Twig};
use crate::twig::TwigsIter;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: Vec<Twig>,
    mutant_files: HashMap<LanguageId, Vec<Twig>>,
}

impl Base {
    pub fn new(root: PathBuf, files: TwigsIter) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            files: files.collect(),
            mutant_files: HashMap::new(),
        })
    }

    pub fn add_mutator(&mut self, language_id: LanguageId, files: TwigsIter) {
        self.mutant_files.insert(language_id, files.collect());
    }

    pub fn twigs(&self) -> impl Iterator<Item = &Twig> + '_ {
        self.files.iter()
    }

    // pub fn mutants(&self) -> impl Iterator<Item = &Mutant> + '_ {
    //     self.files.iter().flat_map(|twig)
    // }

    // pub fn mutations(&self) -> impl Iterator<Item = &Mutation> + '_ {
    //     todo!()
    // }
}

impl Root for Base {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::TestRoot;
    use crate::twig::TwigsIterBuilder;

    fn files_for(root: &impl Root, include: &[&str]) -> TwigsIter {
        let mut builder = TwigsIterBuilder::new();
        for glob in include {
            builder = builder.with_include_glob(glob);
        }
        builder.build(root)
    }

    // core[verify base.root]
    #[test]
    fn base_impls_root() {
        let root = TestRoot::new("/tmp/project");
        let base = Base::new(
            PathBuf::from("/tmp/project"),
            files_for(&root, &[]),
        )
        .unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    // core[verify base.root]
    #[test]
    fn base_rejects_relative_path() {
        let root = TestRoot::new("relative");
        assert!(matches!(
            Base::new(
                PathBuf::from("relative"),
                files_for(&root, &[])
            ),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }
}
