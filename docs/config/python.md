+++
title = "Configuring for Python"
+++

Include a `[lang.py]` section in your config with `include` and `exclude` globs. These control which files bough will attempt to mutate.

```toml
[lang.py]
include = [ "src/**/*.py", ]
exclude = [ "**/test_*.py", "**/*_test.py", ]
```

## Doctests

Python's [doctest] module allows inline tests inside docstrings, which we don't want to mutate. The following config snippet tells bough to skip any string literal containing `>>>` (the doctest prompt marker).

```toml
[lang.py.skip]
query = [
  """
(string
  (string_content) @_content
  (#match? @_content ">>>")) @skip
""",
]
```
