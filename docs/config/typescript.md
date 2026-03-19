+++
title = "TypeScript"
+++

Include a `[lang.ts]` section in your config with `include` and `exclude` globs. These control which files bough will attempt to mutate.

```toml
[lang.ts]
include = [ "src/**/*.ts", "src/**/*.tsx", ]
exclude = [ "**/*.test.*", "**/*__mock__*/**", ]
```

## Vitest

[vitest] allows in-src tests, which we don't want to mutate. The following config snippet tells bough not to mutate any code inside a `if(import.meta.vitest){}` block

```toml
[lang.ts.skip]
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
