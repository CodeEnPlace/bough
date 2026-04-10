# Bough

A polyglot incremental mutation tester.

> [!NOTE]
> Beta software — functional and tested on Linux, macOS, and Windows, but the CLI and config format may still change before 1.0.

📖 **[Documentation](https://bough.codeenplace.dev/)**

## Features

- **Incremental** — mutation testing can take a long time. Bough stores all results to disk, so you can stop a run midway through and resume later. Check your mutation state into git and share coverage data across your team.
- **Polyglot** — built on tree-sitter, so adding new languages reuses most of the mutation testing machinery.
- **Config, not plugins** — no plugin ecosystem to navigate. A single config file controls everything: file globs, test commands, skip patterns via tree-sitter queries.

## Installation

```bash
cargo install --locked --git https://github.com/CodeEnPlace/bough --bin bough
```

Or download a prebuilt binary from the [latest release](https://github.com/CodeEnPlace/bough/releases/latest).

## Quick Start

Create a `bough.config.toml` in your project root:

```toml
base_root_dir = "."
include = ["**/*"]
exclude = []

init.cmd = "npm install"
test.cmd = "npm run test"

[lang.js]
include = ["src/**/*.js"]
exclude = ["**/*.test.*"]
```

Then run:

```bash
bough show mutations  # see what bough will mutate
bough run             # run mutation testing
bough find            # find the most important missed mutations
```

See the [tutorial](https://bough.codeenplace.dev/tutorials/using-bough/) for a full walkthrough.

## Supported Languages

| Language   | ID     |
|------------|--------|
| JavaScript | `js`   |
| TypeScript | `ts`   |
| Python     | `py`   |
| Rust       | `rs`   |
| Go         | `go`   |
| C          | `c`    |
| Java       | `java` |
| C#         | `cs`   |

Is your language not supported? [Open an issue or PR on GitHub.](https://github.com/CodeEnPlace/bough/issues)

## License

MIT
