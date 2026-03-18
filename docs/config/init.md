+++
title = "Initalizing Config"
+++

`bough` reads config formatted as `toml`, `yaml`, and `json`. We will use `toml` for the rest of this tutorial

`bough` looks for config files in the current dir, and parent dirs, in the locations:

- `bough.config.toml`
- `.bough.toml`
- `.config/bough.toml`

run

```bash
bough show config
```

in an uninitialized project to see the full ordered list of config search paths.

## Minimal Config

```toml
base_root_dir = "."
include = ["**/*"]
exclude = ["target/**/*"]
test.cmd = "echo 'run test command'"
[lang.js]
```

- `base_root_dir`: Your project root, probably where your `.git` directory is. **Resolved relative to the directory containing your config file!**. Bough requires this explicitly to avoid "magically" detecting your project root and getting it wrong. Example
  - `/foo/bar/baz/bough.config.toml`
  - contains `base_root_dir = "../qux"`
  - resolves: `/foo/bar/quz`
- `include`: Multiple globs matching **ALL** files that are part of your mutation test. These will be coppied into each test workspace. You can check matched files by running `bough show files`
- `exclude`: Multiple globs matching **ALL** files that are **NOT** part of your mutation test. These will **NOT** be coppied into each test workspace. You can check matched files by running `bough show files`
- `test.cmd`: The test command to run
- `[lang.js]`: At least one config section to describe which mutations systems to use. This example is for JavaScript, but any supported language can be used.
