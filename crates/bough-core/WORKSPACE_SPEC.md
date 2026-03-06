## Workspace

bough[workspace.root]
Workspace impls Root

bough[workspace.base]
Workspace holds &Base

bough[workspace.files]
Workspace::files -> TwigsIter

bough[workspace.is-handle]
Workspace struct exists as a handle for a directory

bough[workspace.relationship]
Workspace struct has a 1-to-1 relationship with a workspace directory

bough[workspace.id]
WorkspaceId is a randomly generated 8 char hex identifier

bough[workspace.id.is-dir-name]
The directory a Workspace points to must be a valid WorkspaceId

bough[workspace.id.get]
Workspace::id() -> WorkspaceId

bough[workspace.new]
`Workspace::new(dir: PathBuf) -> Result<Self, worspace::Error>` makes a new dir

bough[workspace.new.dir]
workspace should be created inside the provided dir, in a `work` sub dir

bough[workspace.new.dir.previous]
if the dir previously existed, that's an error

bough[workspace.new.from-base-files]
Workspace should be created by copying the matched files of Base::files

bough[workspace.bind]
`Workplace::bind(dir: PathBuf, id: &WorkspaceId) -> Result<Self, _>` creates a new struct associated with an existing directory

bough[workspace.bind.validate-unchanged]
Workspace::validate_unchanged() is called after bind to ensure it has not changed

bough[workspace.validate-unchanged]
Workspace::validate_unchanged() checks that list and file contents of Base::files and Workspace::files are identicall

bough[workspace.validate-unchanged.untracked]
Workspace::validate_unchanged() doesn't check for equality of files that are not in Base::files; they are allowed to differ, or not be present, in workspaces

bough[workspace.active]
The stored Workspace active Mutation should be of type Mutation

bough[workspace.write_mutant]
`Workspace::write(&mut self, ws: &Mutant) -> Result<(), _>` writes the mutated file to the coresponding file in the specified workspace

bough[workspace.write_mutant.set-active]
`Workspace::write` sets the provided mutant as it's "active" mutant

bough[workspace.write_mutant.set-active.only-one]
Workspace can only have one active mutant, trying to set multiple results in an error

bough[workspace.revert_mutant]
`Workspace::revert(&mut self, ws: &Mutant) -> Result<(), _>` reverts the mutated file in the workspace so it is identical to the file in the Base

bough[workspace.revert_mutant.active]
`Workspace::revert` clears the active mutant
