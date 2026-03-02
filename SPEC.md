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
