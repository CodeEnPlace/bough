## TwigMutantsIter

bough[mutant.twig-iter]
TwigMutantsIter impls Iterator<Item = Mutant>

bough[mutant.twig-iter.base]
TwigMutantsIter holds &Base

bough[mutant.twig-iter.twig]
TwigMutantsIter holds &Twig

bough[mutant.twig-iter.lang]
TwigMutantsIter owns LanguageId

bough[mutant.twig-iter.file]
TwigMutantsIter uses base and twig to resolve the absolute path of the file to parse

bough[mutant.twig-iter.find]
TwigMutantsIter uses its LanguageDriver to walk the parsed tree-sitter AST and find all supported mutants

bough[mutant.twig-iter.find.js.statement]
TwigMutantsIter finds javascript statement blocks

bough[mutant.twig-iter.find.js.condition.if]
TwigMutantsIter finds javascript conditions of if statements

bough[mutant.twig-iter.find.js.condition.while]
TwigMutantsIter finds javascript conditions of while statements

bough[mutant.twig-iter.find.js.condition.for]
TwigMutantsIter finds javascript conditions of for statements

bough[mutant.twig-iter.find.js.binary.add]
TwigMutantsIter finds javascript add binary ops

bough[mutant.twig-iter.find.js.binary.sub]
TwigMutantsIter finds javascript subtract binary ops

bough[mutant.twig-iter.skip.kind]
TwigMutantsIter::with_skip_kind(self, kind: MutantKind) -> Self skips all mutants of the specified kind

bough[mutant.twig-iter.skip.kind.multiple]
with_skip_kind can be called multiple times; a mutant is skipped if its kind matches ANY of the configured skip kinds

bough[mutant.twig-iter.skip.query]
TwigMutantsIter::with_skip_query(self, query: &str) -> Self skips all mutants whose tree-sitter node matches the provided S-expression query

bough[mutant.twig-iter.skip.query.multiple]
with_skip_query can be called multiple times; a mutant is skipped if its node matches ANY of the configured skip queries
