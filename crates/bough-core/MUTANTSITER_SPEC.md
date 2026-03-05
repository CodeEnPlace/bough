## MutantsIter

core[mutant.iter.twig]
MutantsIter holds &Twig

core[mutant.iter.base]
MutantIter holds &Base

core[mutant.iter.file]
MutantIter uses twig and base to create an absolute path for the file it will generate mutants for

core[mutant.iter.lang]
MutantIter owns LanguageId

core[mutant.iter.item]
MutantIter impls Iter<Item=Mutant>

core[mutant.iter.find]
MutantIter uses its driver to walk its file and find all mutants we support

core[mutant.iter.skip.kind]
MutantIter should be configureable to skip all mutants of specific kinds

core[mutant.iter.skip.kind.multiple]
it should be possible to skip multiple kinds of mutant

core[mutant.iter.skip.query]
MutantIter should be configureable to skip all mutants that match a specified ts lisp-style query

core[mutant.iter.skip.query.multiple]
it should be possible provide multiple skip ts queries

core[mutant.iter.find.js.statement]
MutantIter finds javascript statement blocks

core[mutant.iter.find.js.condition.if]
MutantIter finds javascript conditions of if statements

core[mutant.iter.find.js.condition.while]
MutantIter finds javascript conditions of while statements

core[mutant.iter.find.js.condition.for]
MutantIter finds javascript conditions of for statements

core[mutant.iter.find.js.binary.add]
MutantIter finds javascript add binary ops

core[mutant.iter.find.js.binary.sub]
MutantIter finds javascript subtract binary ops

