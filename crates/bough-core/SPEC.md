# Core

## File

core[file.root]
Root Path must be created with an absolute path

core[file.twig]
Twig(PathBuf) must be created with a relative path

core[file.file]
`pub struct File<'a> { root: &'a Root, twig: &'a Twig, }`

core[file.file.resolve]
File::resolve joins root and twig to create the fully resolved path

core[file.transplant]
`File::transplant(&self, root: &Root) -> Self` replace root

core[file.files.config]
FilesIter holds a ref to FileSourceConfig

core[file.files.root]
FilesIter holds a ref to a Root

core[file.files.iter]
FilesIter iterates Twigs

core[file.files.iter.include]
A file should be included if it matches any of the include globs

core[file.files.iter.exclude]
A file should be excluded if it matches any of the exclude globs

core[file.files.iter.vcs-ignore]
A file should be excluded if it matches any of the globs in a vcs ignore file

## Base

core[base.root]
Base impls Root

core[base.files]
Base::files -> FilesIter

core[base.mutant_files]
Base::mutant_files(language_id: &LanguageId) -> FilesIter

## Workspace

core[workspace.root]
Workspace impls Root

core[workspace.base]
Workspace holds &Base

core[workspace.files]
Workspace::files -> FilesIter

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

core[workspace.bind.validate-unchanged.set_active]
if Workspace::validate_unchanged() finds 1 single active Mutation, bind should set it as the Workspace's active Mutation

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

## Phase

core[phase.root]
Phase holds &Root

core[phase.pwd]
Phase holds a twig called pwd

core[phase.env]
Phase holds a HashMap<String,String> of env vars to apply

core[phase.cmd]
Phase::cmd is a Vec<String> that will be run as a sub process

core[phase.timeout]
Phase::timeout is a TimeoutConfig

core[phase.run]
`Phase::run() -> Result<_,_>` runs Phase::cmd

core[phase.run.pwd]
Phase::run runs the command in File { root, pwd }

core[phase.run.env]
Phase::run runs the command with the specified env vars

core[phase.run.timeout]
Phase::run stops the command if it extends the provided timeout

core[phase.run.timeout.absolute]
Phase::run stops the command if it extends the provided absolute timeout

core[phase.run.timeout.relative]
Phase::run takes an optional Duration. it stops the command if this Duration is defined, and it extends the provided relative `timeout * provided_duration`

core[phase.out]
`Phase::run -> Result<PhaseOutcome , _>`

core[phase.out.stdio]
PhaseOutcome should retain all stdout and stderr from the command

core[phase.out.exit]
PhaseOutcome should contain the exit code of the command, non-zero should return Ok(), not Err

core[phase.out.duration]
PhaseOutcome should contain the duration time of the command

## MutantsIter

core[mutant.iter.twig]
MutantsIter holds &Twig

core[mutant.iter.base]
MutantIter holds &Base

core[mutant.iter.file]
MutantIter uses twig and base to create an absolute path for the file it will generate mutants for

core[mutant.iter.lang]
MutantIter owns LanguageId

core[mutant.iter.item]
MutantIter impls Iter<Item=Mutant>

core[mutant.iter.find]
MutantIter uses its driver to walk its file and find all mutants we support

core[mutant.iter.find.js.statement]
MutantIter finds javascript statement blocks

core[mutant.iter.find.js.condition.if]
MutantIter finds javascript conditions of if statements

core[mutant.iter.find.js.condition.while]
MutantIter finds javascript conditions of while statements

core[mutant.iter.find.js.condition.for]
MutantIter finds javascript conditions of for statements

core[mutant.iter.find.js.binary.add]
MutantIter finds javascript add binary ops

core[mutant.iter.find.js.binary.sub]
MutantIter finds javascript subtract binary ops

## Mutant

core[mutant.lang]
Mutant owns LanguageId

core[mutant.base]
Mutant holds &Base

core[mutant.twig]
Mutant holds &Twig

core[mutant.kind]
Mutant owns MutantKind

core[mutant.span]
Mutant owns Span

core[span.point]
Span is composed of two Points

core[point.line]
Point::line is a usize representing the line of the file it points to

core[point.col]
Point::col is a usize representing the col of the file it points to

core[point.byte]
Point::byte is a usize representing the byte of the file it points to

core[mutant.hash.typed-hashable]
Mutant should impl TypedHashable

core[mutant.hash.base]
Mutant hash MUST NOT include base, if two identical files exist in two different bases, their mutant should hash to the same value

core[mutant.hash.lang]
Mutant hash should include lang

core[mutant.hash.twig]
Mutant hash should include twig

core[mutant.hash.file]
Mutant hash should include the contents of its base + twig File

core[mutant.hash.span]
Mutant hash should include span

core[mutant.hash.kind]
Mutant hash should include kind

## MutationIter

core[mutation.iter.mutant]
MutationIter holds &Mutant

core[mutation.iter.mutation]
MutationIter impls Iter<Item = Mutation>

core[mutation.iter.language_driver]
MutationIter delegates to LangaugeDriver to produce valid strings to replace Mutant with

core[mutation.iter.invalid]
If the Mutant is not syntactically valid for a language (eg, StrictEq '===' in rust), it should produce no Mutations

## Mutation

core[mutation.mutant]
Mutation holds &Mutant

core[mutation.subst]
Mutation owns a string that the span of it's Mutant could be replaced with in the pointed twig to produce a different syntactically valid program

core[mutation.subst.js.add.sub]
If the js Mutant was for '+', there should be a Mutation to replace it with '-'

core[mutation.subst.js.add.mul]
If the js Mutant was for '+', there should be a Mutation to replace it with '\*'

core[mutation.subst.js.statement]
If the js Mutant was for a statement block, there should be a Mutation to replace it with '{}'

core[mutation.subst.js.cond.true]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'true'

core[mutation.subst.js.cond.false]
If the js Mutant was for a condition, there should be a Mutation to replace it with 'false'

core[mutation.hash.typed-hashable]
Mutant should impl TypedHashable

core[mutation.hash.mutant]
Mutation hash should include Mutant

core[mutation.hash.subst]
Mutation hash should include subst

<!-- ### Config -->

<!-- core[config.partials] -->
<!-- Config should be constable from multiple partials, checking for required values an invariants once all are applied -->

<!-- core[config.source-dir] -->
<!-- Config should retain the root of the source code as source_dir -->

<!-- core[config.pwd.root] -->
<!-- Config should set the pwd for commands -->

<!-- core[config.pwd.phase] -->
<!-- A phase should be able to override pwd -->

<!-- ### Session -->

<!-- core[session.new] -->
<!-- Session::new(config: Config) -> Result<Self, session::Error> -->

<!-- core[session.is-entry-point] -->
<!-- All IO and actions must be performed starting by using the Session to create Structs (eg, Session::get_workspace, Session::get_source_dir) -->

<!-- core[session.workspace.discovery] -->
<!-- Session should find all pre-existing workspaces at creation time -->

<!-- core[session.workspace.discovery.changed] -->
<!-- If during discovery, one of the workspaces failed to validate_unchanged, the directory should be removed. -->

<!-- core[session.bough-dir.in] -->
<!-- The bough dir may be inside the source dir -->

<!-- core[session.bough-dir.out] -->
<!-- The bough dir may be outside the source dir -->

<!-- core[session.bough-dir.impure] -->
<!-- The bough dir may be touched or altered, even if it exists inside the source dir -->

<!-- ### Workspace -->

<!-- ### Phase -->

<!-- core[phase.in-source.timeout] -->
<!-- A phase should be runnable in the Source dir, producing an InSourceDuration -->

<!-- core[phase.in-workspace.timeout] -->
<!-- A phase running in a workspace should be provided with a InSourceDuration struct that says how long the phase took to execute when run in the Source Dir -->

<!-- core[phase.setup.pwd] -->
<!-- A PhaseRunner should be created with a pwd, resolved from the PhaseConfig, Config, or process pwd, in that order -->

<!-- core[phase.setup.timeout] -->
<!-- A PhaseRunner should be created with a timeout, resolved from the PhaseConfig, or Config, in that order -->

<!-- core[phase.setup.env] -->
<!-- A PhaseRunner should be created with an env var map, resolved by merging from the PhaseConfig, & Config & process environment varialbes, -->

<!-- core[phase.setup.env.unset] -->
<!-- If a config sets an env var to `""`, that should remove it from the env map. -->

<!-- ### Mutation -->

<!-- core[mut.apply.not-in-source] -->
<!-- A Mutation should never be applied to a file in the source dir -->

<!-- core[mut.apply.in-workspace] -->
<!-- A Mutation can only be applied to a file in a workspace dir -->

<!-- core[mut.undo] -->
<!-- Mutation::undo should reset the workspace to the pre-mutation state, so Workspace::validate_unchanged() succeeds again. -->

<!-- core[mut.by-lang] -->
<!-- A config can specify multiple languages to produce mutations for -->

<!-- core[mut.by-lang.files] -->
<!-- Files sourced are configured using the same schema as overall source files, but do not nescisarily match the same files -->

<!-- core[mut.by-lang.files.only-one] -->
<!-- No file can be matched by multiple languages -->

<!-- core[mut.by-lang.files.in-source] -->
<!-- It is an error for a language config to mutate a file that is not included in SourceDir::all_files -->

<!-- ### MutationResult -->

<!-- core[mut-res.role] -->
<!-- `MutationResult`s store the most recent outcoming of running a Test Phase against the specified Mutation -->

<!-- core[mut-res.store] -->
<!-- `MutationResult` are stored and managed via a DiskHashStore bound to `$BOUGH_DIR/state` -->

<!-- core[mut-res.init] -->
<!-- `MutationResult` is created on disk once a mutation is identified, even if the test suite has not been identified so there is no `outcome` -->

<!-- core[mut-res.hash] -->
<!-- `MutationResult` identified by the hash of their mutation, not any other properties. Updating other properties should not alter its hash -->

<!-- core[mut-res.missed] -->
<!-- MutationResult::outcome should be set to missed if the test phase exits zero when run on a workspace that has the coresponding mutant applied -->

<!-- core[mut-res.caught] -->
<!-- MutationResult::outcome should be set to caught if the test phase exits non-zero when run on a workspace that has the coresponding mutant applied -->

<!-- core[mut-res.mod-at] -->
<!-- MutationResult::modified_at should be updated every time the mutation result changes. -->

<!-- core[mut-res.mod-at.not-changed] -->
<!-- MutationResult::modified_at should not be updated if the mutation result has not changed. -->

<!-- ### Testing -->

<!-- core[testing.source] -->
<!-- All tests that involve file IO should start by creating a temp dir, copying the contents of examples/vitest-js in, and operating over that temp dir -->

<!-- core[testing.config] -->
<!-- tests should define their config via a TOML string, tests can share config strings. -->
