+++
title = "Bough"
idx = 0
+++

Bough is a polyglot incremental mutation tester

## Mutation Testing

Bough takes your project and finds ways to modify the source code to produce different, syntactically valid, programs. It then runs your test suite. It makes sense that modifying your source code should cause at least one test to break, so if none do we've found a "missed mutation".

## Incremental

Mutation testing can take a _very_ long time. Bough stores all information to disk, so you can stop a mutation testing run midway through, and resume later. It also records information to make sure it only has to run the minimum number of tests.

## Polyglot

Bough is based on [tree sitter], so it's easy to add new languages and reused most of the mutation testing process. Is your language not supported? Open an issue or PR on [github]!

[tree sitter]: https://tree-sitter.github.io/tree-sitter/
[github]: https://github.com/CodeEnPlace/bough
