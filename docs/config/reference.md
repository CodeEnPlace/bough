---
title: Configuration Reference
---

## `Config`

Project configuration. Typically loaded from `bough.config.toml` or equivalent. All fields can also be set via `BOUGH_*` env vars or `--config.*` CLI flags.

| Field | Type | Description |
|-------|------|-------------|
| `workers` | `u64` | Number of parallel worker processes. Each gets its own workspace copy. Default: 1. *(optional)* |
| `threads` | `u64` | Number of threads bough uses for analysis and mutation discovery. Default: 1. *(optional)* |
| `base_root_dir` | `String` | Root directory of the project source tree. Relative paths are resolved from the config file's directory. Required. |
| `include` | `String[]` | Glob patterns for files to clone into workspaces and scan for mutations. At least one is required. Example: `["src/**", "lib/**"]`. |
| `exclude` | `String[]` | Glob patterns to exclude from workspace cloning and mutation scanning. `.gitignore` patterns, VCS directories, and the `.bough/` state dir are always excluded automatically. |
| `lang` | `Map<LanguageId, LanguageConfig>` | Per-language configuration keyed by language id. At least one language must be configured. |
| `pwd` | `String?` | Working directory for the phase command. Relative to `base_root_dir`. Default: `.`. *(optional)* |
| `env` | `Map<String, String>?` | Extra environment variables. Phase-level values merge with top-level defaults; set a key to `""` to remove an inherited variable. *(optional)* |
| `timeout` | `TimeoutConfig?` | Timeout limits for the phase command. *(optional)* |
| `test` | `TestPhaseConfig?` | Test phase. Required. The command that runs your test suite. *(optional)* |
| `init` | `PhaseConfig?` | Init phase. Optional command run once per workspace before testing begins. *(optional)* |
| `reset` | `PhaseConfig?` | Reset phase. Optional command run after each mutation test to restore workspace state. *(optional)* |
| `find` | `FindMutationsConfig` | Controls how `bough find` selects and ranks mutations. *(optional)* |

### `LanguageConfig`

Per-language file matching and skip rules.

| Field | Type | Description |
|-------|------|-------------|
| `include` | `String[]` | Glob patterns matching source files for this language. Example: `["**/*.ext"]`. |
| `exclude` | `String[]` | Additional exclude globs for this language, appended after the top-level `exclude` patterns. |
| `skip` | `LanguageSkipConfig?` | Optional rules for skipping certain AST nodes from mutation. *(optional)* |

#### `LanguageSkipConfig`

Rules for excluding specific AST nodes from mutation.

| Field | Type | Description |
|-------|------|-------------|
| `query` | `String[]` | Tree-sitter query patterns. Any AST node matching one of these queries will not be mutated. *(optional)* |

### `TimeoutConfig`

Timeout limits for a phase command. At least one of `absolute` or `relative` must be set when the timeout section is present.

| Field | Type | Description |
|-------|------|-------------|
| `absolute` | `u64?` | Hard cap in seconds. The command is killed after this duration regardless of anything else. *(optional)* |
| `relative` | `f64?` | Multiplier applied to the baseline test duration. E.g. `3.0` means "allow up to 3× the unmutated test time". *(optional)* |

### `TestPhaseConfig`

Test phase configuration. Defines the command bough runs to determine whether a mutation is killed.

| Field | Type | Description |
|-------|------|-------------|
| `cmd` | `String` | Shell command to run the test suite. Required. |
| `pwd` | `String?` | Working directory for the phase command. Relative to `base_root_dir`. Default: `.`. *(optional)* |
| `env` | `Map<String, String>?` | Extra environment variables. Phase-level values merge with top-level defaults; set a key to `""` to remove an inherited variable. *(optional)* |
| `timeout` | `TimeoutConfig?` | Timeout limits for the phase command. *(optional)* |

### `PhaseConfig`

Configuration for an optional phase (init or reset). Omit the section entirely to skip the phase.

| Field | Type | Description |
|-------|------|-------------|
| `cmd` | `String?` | Shell command to run for this phase. *(optional)* |
| `pwd` | `String?` | Working directory for the phase command. Relative to `base_root_dir`. Default: `.`. *(optional)* |
| `env` | `Map<String, String>?` | Extra environment variables. Phase-level values merge with top-level defaults; set a key to `""` to remove an inherited variable. *(optional)* |
| `timeout` | `TimeoutConfig?` | Timeout limits for the phase command. *(optional)* |

### `FindMutationsConfig`

Settings for `bough find` — controls how many mutations are selected and which ranking factors are used to prioritise them.

| Field | Type | Description |
|-------|------|-------------|
| `number` | `usize` | Total number of mutations to return. Default: 1. *(optional)* |
| `number_per_file` | `usize` | Maximum mutations to return per source file. Default: 1. *(optional)* |
| `factors` | `Factor[]` | Ranking factors used to score and sort candidate mutations. Default: `[EncompasingMissedMutationsCount, TSNodeDepth]`. *(optional)* |

#### `Factor`

- **`FileAuthorCount`** — How many authors have touched this file
- **`MutationSeverity`** — Severity of the mutation operator (e.g. removing a null check vs flipping a comparator)
- **`EncompasingMissedMutationsCount`** — How many mutations have a span that includes this mutation
- **`SiblingMissedMutations`** — How many other surviving mutants exist in the same function
- **`SiblingOperatorDiversity`** — How many distinct mutation operator types survive in the same function
- **`TSNodeDepth`** — How deep into the tree-sitter node graph is this mutation
- **`VcsFileChurn`** — How many times has this file been modified in version control
- **`VcsLineBlameRecency`** — How recently was this line touched

