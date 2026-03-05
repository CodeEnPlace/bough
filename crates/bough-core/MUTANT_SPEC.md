## Mutant

core[mutant.lang]
Mutant owns LanguageId

core[mutant.base]
Mutant holds &Base

core[mutant.twig]
Mutant holds &Twig

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

core[mutant.hash.base]
Mutant hash MUST NOT include base, if two identical files exist in two different bases, their mutant should hash to the same value

core[mutant.hash.lang]
Mutant hash should include lang

core[mutant.hash.twig]
Mutant hash should include twig

core[mutant.hash.file]
Mutant hash should include the contents of its base + twig File

core[mutant.hash.span]
Mutant hash should include span

core[mutant.hash.kind]
Mutant hash should include kind

