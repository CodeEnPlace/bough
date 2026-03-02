# Spec

## Core

### Config

core[config.partials]
Config should be constable from multiple partials, checking for required values an invariants once all are applied

core[config.source-dir]
Config should retain the root of the source code as source_dir

core[config.pwd.root]
Config should set the pwd for commands

core[config.pwd.phase]
A phase should be able to override pwd

### Session

core[session.new]
Session::new(config: Config) -> Result<Self, session::Error>

core[session.is-entry-point]
All IO and actions must be performed starting by using the Session to create Structs (eg, Session::get_workspace, Session::get_source_dir)

core[session.workspace.discovery]
Session should find all pre-existing workspaces at creation time

core[session.workspace.discovery.changed]
If during discovery, one of the workspaces failed to validate_unchanged, the directory should be removed.

core[session.bough-dir.in]
The bough dir may be inside the source dir

core[session.bough-dir.out]
The bough dir may be outside the source dir

core[session.bough-dir.impure]
The bough dir may be touched or altered, even if it exists inside the source dir

### Source

core[source]
SourceDir struct exists as a handle for a directory

core[source.new]
`SourceDir::new(config: &Config) -> Result<Self, _>`

core[workspace.relationship]
SourceDir struct has a 1-to-1 relationship with the source directory

core[source.pure]
The source directory on disk must never be touched or altered

core[source.files.include]
A file should be included if it matches any of the include globs

core[source.files.exclude]
A file should be excluded if it matches any of the exclude globs

core[source.files.vcs-ignore]
A file should be excluded if it matches any of the globs in a vcs ignore file

core[source.files.iter]
`SourceDir::all_files -> Iter<SourceFile>` returns an iterator overall files matched

### Workspace

core[workspace]
Workspace struct exists as a handle for a directory

core[workspace.relationship]
Workspace struct has a 1-to-1 relationship with a workspace directory

core[workspace.id]
WorkspaceId is a randomly generated 8 char hex identifier

core[workspace.new]
`Workspace::new -> Result<Self, worspace::Error>` makes a new dir

core[workspace.new.dir]
workspace should be created inside the configured bough dir, in a `work` sub dir

core[workspace.new.dir.previous]
if the dir previously existed, that's an error

core[workspace.new.from-source-files]
Workspace should be reated by copying the matched files of Source::all_files

core[workspace.bind]
`Workplace::bind(id: &WorkspaceId) -> Result<Self, _>` creates a new struct associated with an existing directory

core[workspacen.bind.validate-unchanged]
Workspace::validate_unchanged() is called after bind to ensure it has not changed

core[workspace.validate-unchanged]
Workspace::validate_unchanged() checks that the files from Source::all_files are bitwise equal in source and its dir.

### Phase

core[phase.in-source.timeout]
A phase should be runnable in the Source dir, producing an InSourceDuration

core[phase.in-workspace.timeout]
A phase running in a workspace should be provided with a InSourceDuration struct that says how long the phase took to execute when run in the Source Dir

core[phase.setup.pwd]
A PhaseRunner should be created with a pwd, resolved from the PhaseConfig, Config, or process pwd, in that order

core[phase.setup.timeout]
A PhaseRunner should be created with a timeout, resolved from the PhaseConfig, or Config, in that order

core[phase.setup.env]
A PhaseRunner should be created with an env var map, resolved by merging from the PhaseConfig, & Config & process environment varialbes,

core[phase.setup.env.unset]
If a config sets an env var to `""`, that should remove it from the env map.

### Mutation

core[mut.apply.not-in-source]
A Mutation should never be applied to a file in the source dir

core[mut.apply.in-workspace]
A Mutation can only be applied to a file in a workspace dir

core[mut.undo]
Mutation::undo should reset the workspace to the pre-mutation state, so Workspace::validate_unchanged() succeeds again.

### MutationResult

core[mut-res.role]
`MutationResult`s store the most recent outcoming of running a Test Phase against the specified Mutation

core[mut-res.store]
`MutationResult` are stored and managed via a DiskHashStore bound to `$BOUGH_DIR/state`

core[mut-res.init]
`MutationResult` is created on disk once a mutation is identified, even if the test suite has not been identified so there is no `outcome`

core[mut-res.hash]
`MutationResult` identified by the hash of their mutation, not any other properties. Updating other properties should not alter its hash

core[mut-res.missed]
MutationResult::outcome should be set to missed if the test phase exits zero when run on a workspace that has the coresponding mutant applied

core[mut-res.caught]
MutationResult::outcome should be set to caught if the test phase exits non-zero when run on a workspace that has the coresponding mutant applied

core[mut-res.mod-at]
MutationResult::modified_at should be updated every time the mutation result changes.

core[mut-res.mod-at.not-changed]
MutationResult::modified_at should not be updated if the mutation result has not changed.

### Testing

core[testing.source]
All tests that involve file IO should start by creating a temp dir, copying the contents of examples/vitest-js in, and operating over that temp dir

core[testing.config]
tests should define their config via a TOML string, tests can share config strings.
