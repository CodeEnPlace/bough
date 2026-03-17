+++
title = "Configuing for JavaScript"
+++

```toml
[lang.js]
include = [ "src/**/*.js", "src/**/*.jsx", ]
exclude = [ "**/*.test.*" ]
```

## Vitest

[vitest] allows in-src tests, which we don't want to mutate. The following config snippet tells bough not to mutate any code inside a `if(import.meta.vitest){}` block

```toml
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
