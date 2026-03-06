## Mutant

core[mutant.lang]
Mutant owns LanguageId

core[mutant.twig]
Mutant owns Twig

core[mutant.kind]
Mutant owns MutantKind

core[mutant.span]
Mutant owns Span

core[span.point]
Span is composed of two Points

core[point.line]
Point::line is a usize representing the line of the file it points to

core[point.col]
Point::col is a usize representing the col of the file it points to

core[point.byte]
Point::byte is a usize representing the byte of the file it points to

core[mutant.hash.typed-hashable]
Mutant should impl TypedHashable

core[mutant.hash.lang]
Mutant hash should include lang

core[mutant.hash.twig]
Mutant hash should include twig

core[mutant.hash.span]
Mutant hash should include span

core[mutant.hash.kind]
Mutant hash should include kind

## BasedMutant

core[mutant.based]
BasedMutant composes Mutant + &Base

core[mutant.based.base]
BasedMutant holds &Base

core[mutant.based.mutant]
BasedMutant holds Mutant

core[mutant.based.hash.typed-hashable]
BasedMutant should impl TypedHashable

core[mutant.based.hash.base]
BasedMutant hash MUST NOT include base, if two identical files exist in two different bases, their BasedMutant should hash to the same value

core[mutant.based.hash.file]
BasedMutant hash should include the contents of its base + twig File
