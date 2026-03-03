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

### Source

r[core.source.pure]
The source directory must never be touched or altered

r[core.source.pure.bough-dir]
The bough dir may be touched or altered, even if it exists inside the source dir

r[core.source.files.include]
A file should be included if it matches any of the include globs

r[core.source.files.exclude]
A file should be excluded if it matches any of the exclude globs

r[core.source.files.vcs-ignore]
A file should be excluded if it matches any of the globs in a vcs ignore file

### Workspaces

r[core.workspaces.created]
workspaces should be created inside the configured bough dir, in a `work` sub dir

r[core.workspaces.created.from-source-files]
Workspaces should be created by copying the matched files of r[core.source.files]

### Phase

r[core.phase.in-workspace]
A phase should only ever run inside a workspace dir, never in the source dir

### Testing

r[core.testing.source]
All tests that involve file IO should start by creating a temp dir, copying the contents of examples/vitest-js in, and operating over that temp dir

r[core.testing.config]
tests should define their config via a TOML string, tests can share config strings.
