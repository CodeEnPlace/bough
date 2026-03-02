# Spec

## Core

### Config

r[core.config.partials]
Config should be constable from multiple partials, checking for required values an invariants once all are applied

r[core.config.source-dir]
Config should retain the root of the source code as source_dir

r[core.config.pwd.root]
Config should set the pwd for commands

r[core.config.pwd.phase]
A phase should be able to override pwd

### Session

r[core.session.creation]
A session should be created by passing it a Config

r[core.session.entry-point]
All IO and actions must be performed starting by using the Session to create Structs (eg, Session::get_workspace, Session::get_source_dir)

r[core.session.workspace.discovery]
Session should find all pre-existing workspaces at creation time

r[core.session.bough-dir.in]
The bough dir may be inside the source dir

r[core.session.bough-dir.out]
The bough dir may be outside the source dir

r[core.session.bough-dir.impure]
The bough dir may be touched or altered, even if it exists inside the source dir

### Source

r[core.source.pure]
The source directory must never be touched or altered

r[core.source.files.include]
A file should be included if it matches any of the include globs

r[core.source.files.exclude]
A file should be excluded if it matches any of the exclude globs

r[core.source.files.vcs-ignore]
A file should be excluded if it matches any of the globs in a vcs ignore file

r[core.source.files.iter]
Source::all_files returns an iterator overall files matched

### Workspace

r[core.workspace]
Workspace struct exists as a handle for a directory

r[core.workspace.relationship]
Workspace struct has a 1-to-1 relationship with a workspace directory

r[core.workspace.id]
WorkspaceId is a randomly generated 8 char hex identifier

r[core.workspace.create]
`Workplace::create -> Result<Self, _>` makes a new dir

r[core.workspace.create.dir]
workspace should be created inside the configured bough dir, in a `work` sub dir

r[core.workspace.create.dir.previous]
if the dir previously existed, that's an error

r[core.workspace.create.from-source-files]
Workspace should be created by copying the matched files of Source::all_files

r[core.workspace.create.validate-unchanged]
called after creation

r[core.workspace.attach]
`Workplace::attach -> Result<Self, _>` creates a new struct associated with an existing directory

r[core.workspace.attach.validate-unchanged]
called after attach

r[core.workspace.validate-unchanged]
Workspace::validate_unchanged checks that the files from Source::all_files are bitwise equal in source and its dir.

### Phase

r[core.phase.in-workspace]
A phase should only ever run inside a workspace dir, never in the source dir

### Mutation

r[core.mut.apply.not-in-source]
A Mutation should never be applied to a file in the source dir

r[core.mut.apply.in-workspace]
A Mutation can only be applied to a file in a workspace dir

### MutationResult

r[core.mut-res.role]
`MutationResult`s store the most recent outcoming of running a Test Phase against the specified Mutation

r[core.mut-res.store]
`MutationResult` are stored and managed via a DiskHashStore bound to `$BOUGH_DIR/state`

r[core.mut-res.hash]
`MutationResult` identified by the hash of their mutation, not any other properties. Updating other properties should not alter its hash

### Testing

r[core.testing.source]
All tests that involve file IO should start by creating a temp dir, copying the contents of examples/vitest-js in, and operating over that temp dir

r[core.testing.config]
tests should define their config via a TOML string, tests can share config strings.
