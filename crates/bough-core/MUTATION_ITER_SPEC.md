## MutationIter

bough[mutation.iter.mutant]
MutationIter holds &Mutant

bough[mutation.iter.mutation]
MutationIter impls Iter<Item = Mutation>

bough[mutation.iter.language_driver]
MutationIter delegates to LangaugeDriver to produce valid strings to replace Mutant with

bough[mutation.iter.invalid]
If the Mutant is not syntactically valid for a language (eg, StrictEq '===' in rust), it should produce no Mutations
