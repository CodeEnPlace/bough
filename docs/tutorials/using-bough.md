+++
title= "Using Bough"
+++

This tutorial will walk through setting up bough for a new JavaScript project, running the mutation test suite, and getting actionable information from that test run.A

We'll be using the code in `./examples/vitest-js`, so you can clone the bough repo and follow along.

## Setup

### Installation

First, install bough

```bash
cargo install --locked --git https://github.com/CodeEnPlace/bough --bin bough
```

verify it installed by running

```bash
bough
```

```output
Bough — a polyglot mutation testing tool.
```

### Configuration

Insert the following into `bough.config.toml`:

```toml
# Tells bough what the root of your project is, resolved relative to `bough.config.toml'
base_root_dir = "."

# Which files are required to run a test
include =["**/*"]

# Which files should not be coppied over to test workspaces.
# .gitignore contents are included here automatically, so we don't need to specify `node_modules`
exclude  = []

# The command to run when setting up a bough workspace to get it ready for testing
init.cmd = "npm install"

# The command to run the tests
test.cmd = "npm run test"

# Per language configuration, defines how bough will mutate your project
[lang.js]

# Which files to run the javascript mutator over
include = [ "src/**/*.js", "src/**/*.jsx", ]

# Which files NOT to mutate. It doesn't make sense to mutate test files
exclude = ["**/*.test.*"]

# Specify which parts of the source files not to mutate. This uses tree-sitter's grammar to match nodes
# Here we're skipping the in-source vitest, but this could also be used to skip comments like // bough-skip
[lang.js.skip]
query = [
  """
(if_statement
  condition: (parenthesized_expression
    (member_expression
      object: (meta_property)
      property: (property_identifier) @_prop))
  (#eq? @_prop "vitest")) @skip
""",
]
```

### Confirm Configuration

```bash
bough show config # outputs the resolved config values
bough show files # shows all the files that will be cloned into workspace dirs
bough show files js # shows all the files that will be mutated
bough show mutations # shows all the mutations that will be applied
bough show mutations js # shows all the javascript mutations that will be applied
bough show mutations js src/index.js # shows all the mutations in a single file
```

All commands accept a `--format` with 1 on the following options:

- `terse` (default) Short, usually 1 line output
- `verbose` Longer, usually multiline where relevant
- `markdown` Markdown format, good for providing to LLMs
- `json` JSON format, good for scripting and programmatic interfaces

The `mutations` commands should all report 'not run`, showing that bough currently doesn't know if these mutations would be caught or not

## Run Mutation Test Suite

We can now use bough to mutation test. Running this command:

`bough run --config.workers=2`

- Re-scan your included files to make sure the mutation state is up to date
- Remove any stale workspaces from previous runs
- Create 2 new workspaces (workers = 2)
- Run `init.cmd` in each of those workspace
- While there are mutations that are either `Missed` or `Not Run`, for each workspace:
  1. Run `reset.cmd`
  1. Apply a mutation
  1. Run `test.cmd`
  1. See if test fails or passes
  1. Record test status for that mutation
  1. Un-apply the mutation in that workspace

bough shows an interactive process bar during this process

### Files

By default bough writes workspaces and state information to `.bough/workspaces` and `.bough/state`. The current intended workflow is to gitignore `.bough/workspaces`, and check `.bough/state` into git; so that current mutation coverage state is tracked and distributed to all developers. You don't have to do this, and it may change in future.

## View Mutations Status

Viewing the mutations again with

```bash
bough show mutations
```

will now show them either as `caught` or `missed`. It's the `missed` ones we care about: those are areas of your codebase that bough was able to change without breaking tests, suggesting that they're not properly covered by testing.

### Which Mutations to Test First

In a large codebase it can be hard to figure out which mutation to test first. bough supports searching your missed mutations and sorting them via various heuristics to find the ones you should test next:

```bash
bough find
```

```output
1.00 986aa807b8d004aaf8e1e0dfb307d111bac35cf10e99a6a666676ccc5be27623 js src/index.js 9:15 - 9:16 missed Literal(Number) -> 0
```

You can get more detailed information on that mutation using `bough show mutation`; and you can even put it in markdown format to make it easy for LLMs to understand:

```bash
bough show mutation 986aa807b8d004aaf8e1e0dfb307d111bac35cf10e99a6a666676ccc5be27623 --format markdown
```

````markdown
# Mutation

986aa807b8d004aaf8e1e0dfb307d111bac35cf10e99a6a666676ccc5be27623 JavaScript src/index.js 9:15 - 9:16 missed Literal(Number) -> 0

## Before

```javascript
function childsDay(date) {
  const day = date.getDay();

  if (day === 0) return "bonny and blithe and good and gay";
  if (day === 1) return "fair of face";
  if (day === 2) return "full of grace";
  if (day === 3) return "full of woe";
  if (day === 4) return "has far to go";
  if (day === 5) return "loving and giving";
  if (day === 6) return "works hard for a living";
}
```

## After

```javascript
function childsDay(date) {
  const day = date.getDay();

  if (day === 0) return "bonny and blithe and good and gay";
  if (day === 1) return "fair of face";
  if (day === 2) return "full of grace";
  if (day === 3) return "full of woe";
  if (day === 4) return "has far to go";
  if (day === 0) return "loving and giving";
  if (day === 6) return "works hard for a living";
}
```
````
