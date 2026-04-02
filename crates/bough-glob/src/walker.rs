use bough_fs::{Root, Twig};
use crate::Glob;

pub struct TwigWalker<'a, R: Root> {
    root: &'a R,
    includes: Vec<Glob>,
    excludes: Vec<Glob>,
}

impl<'a, R: Root> TwigWalker<'a, R> {
    pub fn new(root: &'a R) -> Self {
        Self {
            root,
            includes: Vec::new(),
            excludes: Vec::new(),
        }
    }

    pub fn include(mut self, glob: Glob) -> Self {
        self.includes.push(glob);
        self
    }

    pub fn exclude(mut self, glob: Glob) -> Self {
        self.excludes.push(glob);
        self
    }

    pub fn iter(self) -> TwigWalkerIter {
        todo!()
    }
}

pub struct TwigWalkerIter {
    _private: (),
}

impl Iterator for TwigWalkerIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
