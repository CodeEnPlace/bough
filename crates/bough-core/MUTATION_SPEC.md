## Mutation

bough[mutation.mutant]
Mutation holds &Mutant

bough[mutation.subst]
Mutation owns a string that the span of it's Mutant could be replaced with in the pointed twig to produce a different syntactically valid program

bough[mutation.subst.js.add.sub]
If the js Mutant was for '+', there should be a Mutation to replace it with '-'

bough[mutation.subst.js.add.mul]
If the js Mutant was for '+', there should be a Mutation to replace it with '\*'

bough[mutation.subst.js.statement]
If the js Mutant was for a statement block, there should be a Mutation to replace it with '{}'

bough[mutation.subst.js.cond.true]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'true'

bough[mutation.subst.js.cond.false]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'false'

bough[mutation.hash.typed-hashable]
Mutant should impl TypedHashable

bough[mutation.hash.mutant]
Mutation hash should include Mutant

bough[mutation.hash.subst]
Mutation hash should include subst
