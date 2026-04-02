use bough_fs::{Root, Twig};
use crate::Glob;
use std::collections::VecDeque;
use std::path::PathBuf;

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
        let root_path = self.root.path().to_path_buf();
        TwigWalkerIter {
            root_path: root_path.clone(),
            includes: self.includes,
            excludes: self.excludes,
            queue: VecDeque::from([root_path]),
        }
    }
}

pub struct TwigWalkerIter {
    root_path: PathBuf,
    includes: Vec<Glob>,
    excludes: Vec<Glob>,
    queue: VecDeque<PathBuf>,
}

impl Iterator for TwigWalkerIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let path = self.queue.pop_front()?;

            if path.is_symlink() || path.is_dir() {
                let Ok(entries) = std::fs::read_dir(&path) else {
                    continue;
                };
                let mut children: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .collect();
                children.sort();
                for child in children.into_iter().rev() {
                    self.queue.push_front(child);
                }
                continue;
            }

            if !path.is_file() {
                continue;
            }

            let rel = path.strip_prefix(&self.root_path).ok()?;
            let matched_include = self.includes.iter().any(|g| g.is_match(rel));
            let matched_exclude = self.excludes.iter().any(|g| g.is_match(rel));

            if matched_include && !matched_exclude {
                return Twig::new(rel.to_path_buf()).ok();
            }
        }
    }
}
