## Multi lang config

should be like

```toml
[language.js] #must be tree-sitter id
files.include  = [ "**/*.js", "**/*.jsx", ]
files.exclude = [ "**/*__mocks__*" ]
mutants.skip = [
  "((fn 'describe') @ignore)",
  { "BinaryOp" = "Add" }
]

[language.rust] #must be tree-sitter id
files.include  = [ "**/*.rs",  ]
mutants.skip = [
  "((#[cfg(test)]) @ignore)",
  { "BinaryOp" = "Div" }
]
```

## Break appart commands

```toml
[vitest.test]
pwd = "./examples/vitest"
timeout.absolute = 30
timeout.relative = 3
commands = [
  "npx run build",
  "npx run test"
]

[cargo.test]
pwd = "./examples/cargo"
timeout.absolute = 30
timeout.relative = 3
commands = [
  "cargo test",
]
```
