## MutationIter

core[mutation.iter.mutant]
MutationIter holds &Mutant

core[mutation.iter.mutation]
MutationIter impls Iter<Item = Mutation>

core[mutation.iter.language_driver]
MutationIter delegates to LangaugeDriver to produce valid strings to replace Mutant with

core[mutation.iter.invalid]
If the Mutant is not syntactically valid for a language (eg, StrictEq '===' in rust), it should produce no Mutations

