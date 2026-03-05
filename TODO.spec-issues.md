# Spec Issues to Resolve

## Return types

### `base.mutants` / `base.mutations`
Spec says `Base::mutants(language_id: &LanguageId) -> MutantsIter` but `MutantsIter` is per-file (holds single `&Twig`). `Base::mutants` needs to iterate across all files for a language.

Options:
- a) Return `impl Iterator<Item = Mutant<'_>>` — simple, but doesn't match spec signature
- b) Rework `MutantsIter` to support multiple twigs — changes existing type
- c) Create a new `BaseMutantsIter` wrapper — matches spirit but not letter of spec

Same issue applies to `Base::mutations` / `MutationIter`.

## Skip config

### `mutant.iter.skip.kind` / `mutant.iter.skip.kind.multiple`
Where does the skip config live? Options:
- a) Additional params to `MutantsIter::new()`
- b) Builder pattern on `MutantsIter`
- c) Config struct passed in

### `mutant.iter.skip.query` / `mutant.iter.skip.query.multiple`
"ts lisp-style query" — assumed to be tree-sitter S-expression queries. Clarify:
- Does the query match against the mutant's own node?
- Or against the mutant's parent/ancestor context?
- How does it interact with the tree — do we need to retain the parsed tree in `MutantsIter` (currently discarded after `walk_tree`)?
