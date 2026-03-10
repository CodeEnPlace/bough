# Bough

An Incremental Polyglot Mutation Tester

> [!WARNING]
> Pre-Alpha software, highly likely to change and no guarantee of correctness as of yet

## Foundational Ideas

### Incremental

Whenever `bough` learns new information about you coverage, it should be written to disk; and bough should prefer reading information directly from disk. This means that you can pause a mutation testing run midway through, and seamlessly resume it.

You can also check your coverage metadata directly into git, or share it between developers easily.

### Config, Not Plugins

A mutation testing system that tightly integrates with your language ecosystem is great until you deviate from the ecosystem. Plugins create a 2-tier experience where the core maintainer owns a list of "blessed" plugins, and anything outside of that set is subject to breakage.

By instead creating an API surface with config that is as generalized as possible, I hope to make software that is more flexible.
