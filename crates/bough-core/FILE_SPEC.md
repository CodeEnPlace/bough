## File

core[file.root]
Root Path must be created with an absolute path

core[file.twig]
Twig(PathBuf) must be created with a relative path

core[file.file]
`pub struct File<'a> { root: &'a Root, twig: &'a Twig, }`

core[file.file.resolve]
File::resolve joins root and twig to create the fully resolved path

core[file.transplant]
`File::transplant(&self, root: &Root) -> Self` replace root

core[twig.iter.root]
TwigsIter holds a ref to a Root

core[twig.iter]
TwigsIter impls Iterator<Item = Twig>

core[twig.iter.new]
TwigsIter::new(root: &impl Root) -> Self, uses walkdir to recursively walk root

core[twig.iter.include]
TwigsIter::with_include_glob(self, pattern: &str) -> Self adds an include glob::Pattern

core[twig.iter.exclude]
TwigsIter::with_exclude_glob(self, pattern: &str) -> Self adds an exclude glob::Pattern

core[twig.iter.include.match]
A file should be yielded only if it matches any of the include patterns

core[twig.iter.include.empty]
If no include patterns are configured, no files are yielded

core[twig.iter.exclude.match]
A file should be excluded if it matches any of the exclude patterns

