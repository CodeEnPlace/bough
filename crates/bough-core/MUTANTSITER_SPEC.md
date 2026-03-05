## TwigMutantsIter

core[mutant.twig-iter]
TwigMutantsIter impls Iterator<Item = Mutant>

core[mutant.twig-iter.base]
TwigMutantsIter holds &Base

core[mutant.twig-iter.twig]
TwigMutantsIter holds &Twig

core[mutant.twig-iter.lang]
TwigMutantsIter owns LanguageId

core[mutant.twig-iter.file]
TwigMutantsIter uses base and twig to resolve the absolute path of the file to parse

core[mutant.twig-iter.find]
TwigMutantsIter uses its LanguageDriver to walk the parsed tree-sitter AST and find all supported mutants

core[mutant.twig-iter.find.js.statement]
TwigMutantsIter finds javascript statement blocks

core[mutant.twig-iter.find.js.condition.if]
TwigMutantsIter finds javascript conditions of if statements

core[mutant.twig-iter.find.js.condition.while]
TwigMutantsIter finds javascript conditions of while statements

core[mutant.twig-iter.find.js.condition.for]
TwigMutantsIter finds javascript conditions of for statements

core[mutant.twig-iter.find.js.binary.add]
TwigMutantsIter finds javascript add binary ops

core[mutant.twig-iter.find.js.binary.sub]
TwigMutantsIter finds javascript subtract binary ops

core[mutant.twig-iter.skip.kind]
TwigMutantsIter::with_skip_kind(self, kind: MutantKind) -> Self skips all mutants of the specified kind

core[mutant.twig-iter.skip.kind.multiple]
withskip_kind can be called multiple times; a mutant is skipped if its kind matches ANY of the configured skip kinds

core[mutant.twig-iter.skip.query]
TwigMutantsIter::with_skip_query(self, query: &str) -> Self skips all mutants whose tree-sitter node matches the provided S-expression query

core[mutant.twig-iter.skip.query.multiple]
withskip_query can be called multiple times; a mutant is skipped if its node matches ANY of the configured skip queries

<!-- ## TwigsMutantsIter -->

<!-- core[mutant.twigs-iter.twigs] -->
<!-- TwigsMutantsIter holds a TwigsIter -->

<!-- core[mutant.twigs-iter.base] -->
<!-- TwigsMutantsIter holds &Base -->

<!-- core[mutant.twigs-iter.lang] -->
<!-- TwigsMutantsIter owns LanguageId -->

<!-- core[mutant.twigs-iter.item] -->
<!-- TwigsMutantsIter impls Iterator<Item = Mutant> -->

<!-- core[mutant.twigs-iter.find] -->
<!-- TwigsMutantsIter lazily creates a TwigMutantsIter for each twig, yielding all Mutants across all files -->
