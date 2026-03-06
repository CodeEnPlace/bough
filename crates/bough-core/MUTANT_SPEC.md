## Mutant

bough[mutant.lang]
Mutant owns LanguageId

bough[mutant.twig]
Mutant owns Twig

bough[mutant.kind]
Mutant owns MutantKind

bough[mutant.span]
Mutant owns Span

bough[span.point]
Span is composed of two Points

bough[point.line]
Point::line is a usize representing the line of the file it points to

bough[point.col]
Point::col is a usize representing the col of the file it points to

bough[point.byte]
Point::byte is a usize representing the byte of the file it points to

bough[mutant.hash.typed-hashable]
Mutant should impl TypedHashable

bough[mutant.hash.lang]
Mutant hash should include lang

bough[mutant.hash.twig]
Mutant hash should include twig

bough[mutant.hash.span]
Mutant hash should include span

bough[mutant.hash.kind]
Mutant hash should include kind

## BasedMutant

bough[mutant.based]
BasedMutant composes Mutant + &Base

bough[mutant.based.base]
BasedMutant holds &Base

bough[mutant.based.mutant]
BasedMutant holds Mutant

bough[mutant.based.hash.typed-hashable]
BasedMutant should impl TypedHashable

bough[mutant.based.hash.base]
BasedMutant hash MUST NOT include base, if two identical files exist in two different bases, their BasedMutant should hash to the same value

bough[mutant.based.hash.file]
BasedMutant hash should include the contents of its base + twig File
