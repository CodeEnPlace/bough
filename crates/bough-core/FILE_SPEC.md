## File

bough[file.root]
Root Path must be created with an absolute path

bough[file.twig]
Twig(PathBuf) must be created with a relative path

bough[file.file]
`pub struct File<'a> { root: &'a Root, twig: &'a Twig, }`

bough[file.file.resolve]
File::resolve joins root and twig to create the fully resolved path

bough[file.transplant]
`File::transplant(&self, root: &Root) -> Self` replace root

bough[twig.iter.root]
TwigsIter holds a ref to a Root

bough[twig.iter]
TwigsIter impls Iterator<Item = Twig>

bough[twig.iter.new]
TwigsIter::new(root: &impl Root) -> Self, uses walkdir to recursively walk root

bough[twig.iter.include.match]
A file should be yielded only if it matches any of the include patterns

bough[twig.iter.include.empty]
If no include patterns are configured, no files are yielded

bough[twig.iter.exclude.match]
A file should be excluded if it matches any of the exclude patterns
