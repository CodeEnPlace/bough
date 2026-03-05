## Mutation

core[mutation.mutant]
Mutation holds &Mutant

core[mutation.subst]
Mutation owns a string that the span of it's Mutant could be replaced with in the pointed twig to produce a different syntactically valid program

core[mutation.subst.js.add.sub]
If the js Mutant was for '+', there should be a Mutation to replace it with '-'

core[mutation.subst.js.add.mul]
If the js Mutant was for '+', there should be a Mutation to replace it with '\*'

core[mutation.subst.js.statement]
If the js Mutant was for a statement block, there should be a Mutation to replace it with '{}'

core[mutation.subst.js.cond.true]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'true'

core[mutation.subst.js.cond.false]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'false'

core[mutation.hash.typed-hashable]
Mutant should impl TypedHashable

core[mutation.hash.mutant]
Mutation hash should include Mutant

core[mutation.hash.subst]
Mutation hash should include subst
