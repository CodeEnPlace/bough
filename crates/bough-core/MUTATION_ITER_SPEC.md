## MutationIter

core[mutation.iter.mutant]
MutationIter holds &Mutant

core[mutation.iter.mutation]
MutationIter impls Iter<Item = Mutation>

core[mutation.iter.language_driver]
MutationIter delegates to LangaugeDriver to produce valid strings to replace Mutant with

core[mutation.iter.invalid]
If the Mutant is not syntactically valid for a language (eg, StrictEq '===' in rust), it should produce no Mutations

## TwigMutationsIter

core[mutation.twig-iter]
TwigMutationsIter impls Iterator<Item = Mutation>

core[mutation.twig-iter.twig-mutants-iter]
TwigMutationsIter holds TwigMutantsIter

core[mutation.twig-iter.delegates]
TwigMutationsIter constructs a MutationIter for each Mutant yielded by its TwigMutantsIter, and delegates to that sub iter

## TwigsMutationsIter

core[mutation.twigs-iter]
TwigsMutationsIter impls Iterator<Item = Mutation>

core[mutation.twigs-iter.twigs-mutants-iter]
TwigsMutationsIter holds TwigsMutantsIter

core[mutation.twigs-iter.delegates]
TwigsMutationsIter constructs a MutationIter for each Mutant yielded by its TwigsMutantsIter, and delegates to that sub iter

