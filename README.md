# Bough

Cross-language mutation testing.

## Config

Create `bough.toml` in your project root:

```toml
language = "js"
vcs = "jj"
working_dir = "/tmp/bough-work"
parallelism = 2
report_dir = "/tmp/bough-report"
ordering = "random"
files = "src/**/*.js"
sub_dir = "."

[timeout]
absolute = 30
relative = 1.5

[commands]
install = "npm install"
test = "npx vitest run"
```

Also supported: `bough.json`, `.bough.toml`, `.config/bough.toml`, etc. Use `--config <path>` to specify explicitly.

All config values can be overridden via CLI flags (e.g. `--parallelism 4`).

## Manual step-by-step usage

### 1. Plan

Generate all possible mutations:

```
bough step plan
```

Writes a `$HASH.mutants.plan.json` to `working_dir`.

### 2. Create workspaces

```
bough step create
```

Creates one jj workspace per `parallelism` setting. Writes a `$HASH.workspaces.json` manifest.

### 3. Install dependencies

```
bough step install -w <workspace>
```

Runs `commands.install` in the workspace's `sub_dir`.

### 4. Apply a mutation

```
bough step apply -w <workspace> --hash <mutation-hash>
```

Applies a specific mutation from the plan to a file in the workspace.

### 5. Test

```
bough step test -w <workspace>
```

Runs `commands.test`. Exit code 0 = mutant survived, non-zero = mutant killed.

### 6. Reset

```
bough step reset -w <workspace> -r <rev>
```

Resets the workspace back to a clean revision via `jj edit`. Use this between mutation test runs.

### 7. Cleanup

```
bough step cleanup
```

Forgets all jj workspaces and removes working directories.

### Full example

```sh
bough step plan
bough step create
bough step install -w bough-abc12345-0

# test a mutation
bough step apply -w bough-abc12345-0 --hash e463d93771037e15
bough step test -w bough-abc12345-0

# reset and try another
bough step reset -w bough-abc12345-0 -r trunk()
bough step apply -w bough-abc12345-0 --hash 3b5d343b076d654a
bough step test -w bough-abc12345-0

# done
bough step cleanup
```

## Low-level mutation commands

```sh
# list all mutations for a file
bough mutate generate -f src/foo.js

# view a mutation diff
bough mutate view -f src/foo.js --hash <hex>

# apply a mutation in place (requires --force-on-dirty-repo if repo is dirty)
bough mutate apply -f src/foo.js --hash <hex>
```

## Flags

| Flag | Description |
|------|-------------|
| `-l` / `--language` | `js` or `ts` (or set in config) |
| `-v` / `-vv` / `-vvv` | Log verbosity |
| `--style` | `plain`, `pretty`, or `json` |
| `--diff` | `unified` or `side-by-side` |
| `--force-on-dirty-repo` | Allow file-writing actions on dirty repos |
| `--config` | Path to config file |
