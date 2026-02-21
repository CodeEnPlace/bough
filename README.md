# Pollard

Cross-language mutation testing.

## Config

Create `pollard.toml` in your project root:

```toml
language = "js"
vcs = "jj"
working_dir = "/tmp/pollard-work"
parallelism = 2
report_dir = "/tmp/pollard-report"
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

Also supported: `pollard.json`, `.pollard.toml`, `.config/pollard.toml`, etc. Use `--config <path>` to specify explicitly.

All config values can be overridden via CLI flags (e.g. `--parallelism 4`).

## Manual step-by-step usage

### 1. Plan

Generate all possible mutations:

```
pollard step plan
```

Writes a `$HASH.mutants.plan.json` to `working_dir`.

### 2. Create workspaces

```
pollard step create
```

Creates one jj workspace per `parallelism` setting. Writes a `$HASH.workspaces.json` manifest.

### 3. Install dependencies

```
pollard step install -w <workspace>
```

Runs `commands.install` in the workspace's `sub_dir`.

### 4. Apply a mutation

```
pollard step apply -w <workspace> --hash <mutation-hash>
```

Applies a specific mutation from the plan to a file in the workspace.

### 5. Test

```
pollard step test -w <workspace>
```

Runs `commands.test`. Exit code 0 = mutant survived, non-zero = mutant killed.

### 6. Reset

```
pollard step reset -w <workspace> -r <rev>
```

Resets the workspace back to a clean revision via `jj edit`. Use this between mutation test runs.

### 7. Cleanup

```
pollard step cleanup
```

Forgets all jj workspaces and removes working directories.

### Full example

```sh
pollard step plan
pollard step create
pollard step install -w pollard-abc12345-0

# test a mutation
pollard step apply -w pollard-abc12345-0 --hash e463d93771037e15
pollard step test -w pollard-abc12345-0

# reset and try another
pollard step reset -w pollard-abc12345-0 -r trunk()
pollard step apply -w pollard-abc12345-0 --hash 3b5d343b076d654a
pollard step test -w pollard-abc12345-0

# done
pollard step cleanup
```

## Low-level mutation commands

```sh
# list all mutations for a file
pollard mutate generate -f src/foo.js

# view a mutation diff
pollard mutate view -f src/foo.js --hash <hex>

# apply a mutation in place (requires --force-on-dirty-repo if repo is dirty)
pollard mutate apply -f src/foo.js --hash <hex>
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
