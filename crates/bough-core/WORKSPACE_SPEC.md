## Workspace

core[workspace.root]
Workspace impls Root

core[workspace.base]
Workspace holds &Base

core[workspace.files]
Workspace::files -> TwigsIter

core[workspace.is-handle]
Workspace struct exists as a handle for a directory

core[workspace.relationship]
Workspace struct has a 1-to-1 relationship with a workspace directory

core[workspace.id]
WorkspaceId is a randomly generated 8 char hex identifier

core[workspace.id.is-dir-name]
The directory a Workspace points to must be a valid WorkspaceId

core[workspace.id.get]
Workspace::id() -> WorkspaceId

core[workspace.new]
`Workspace::new(dir: PathBuf) -> Result<Self, worspace::Error>` makes a new dir

core[workspace.new.dir]
workspace should be created inside the provided dir, in a `work` sub dir

core[workspace.new.dir.previous]
if the dir previously existed, that's an error

core[workspace.new.from-source-files]
Workspace should be reated by copying the matched files of Source::all_files

core[workspace.bind]
`Workplace::bind(dir: PathBuf, id: &WorkspaceId) -> Result<Self, _>` creates a new struct associated with an existing directory

core[workspace.bind.validate-unchanged]
Workspace::validate_unchanged() is called after bind to ensure it has not changed

core[workspace.validate-unchanged]
Workspace::validate_unchanged() checks that list and file contents of Base::files and Workspace::files are identicall

core[workspace.active]
The stored Workspace active Mutation should be of type Mutation

core[workspace.write_mutant]
`Workspace::write(&mut self, ws: &Mutant) -> Result<(), _>` writes the mutated file to the coresponding file in the specified workspace

core[workspace.write_mutant.set-active]
`Workspace::write` sets the provided mutant as it's "active" mutant

core[workspace.write_mutant.set-active.only-one]
Workspace can only have one active mutant, trying to set multiple results in an error

core[workspace.revert_mutant]
`Workspace::revert(&mut self, ws: &Mutant) -> Result<(), _>` reverts the mutated file in the workspace so it is identical to the file in the Base

core[workspace.revert_mutant.active]
`Workspace::revert` clears the active mutant

