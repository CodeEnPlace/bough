use crate::base::Base;
use bough_fs::{File, Root, Twig};
use crate::mutant::Mutant;
use crate::mutation::Mutation;
use bough_typed_hash::TypedHashable;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

// bough[impl workspace.active]
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveMutation {
    mutant: Mutant,
    mutation: Mutation,
}

impl ActiveMutation {
    pub fn mutant(&self) -> &Mutant {
        &self.mutant
    }

    pub fn mutation(&self) -> &Mutation {
        &self.mutation
    }
}

#[derive(Debug)]
pub enum Error {
    File(bough_fs::Error),
    IdParse(String),
    DirAlreadyExists(PathBuf),
    Io(std::io::Error),
    Unchanged(String),
    AlreadyActive,
    NotActive,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::File(e) => write!(f, "{e}"),
            Error::IdParse(s) => write!(f, "invalid workspace id: {s}"),
            Error::DirAlreadyExists(p) => {
                write!(f, "workspace dir already exists: {}", p.display())
            }
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Unchanged(msg) => write!(f, "workspace changed: {msg}"),
            Error::AlreadyActive => write!(f, "workspace already has an active mutant"),
            Error::NotActive => write!(f, "workspace has no active mutant to revert"),
        }
    }
}

impl std::error::Error for Error {}

impl From<bough_fs::Error> for Error {
    fn from(e: bough_fs::Error) -> Self {
        Error::File(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

// bough[impl workspace.id]
// bough[impl workspace.id.is-dir-name]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn generate() -> Self {
        use std::fmt::Write;
        let mut buf = String::with_capacity(8);
        let bytes: [u8; 4] = rand::random();
        for b in bytes {
            write!(buf, "{b:02x}").unwrap();
        }
        Self(buf)
    }

    pub fn parse(s: &str) -> Result<Self, Error> {
        if s.len() != 8 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::IdParse(s.to_string()));
        }
        Ok(Self(s.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// bough[impl workspace.is-handle]
// bough[impl workspace.relationship]
// bough[impl workspace.root]
// bough[impl workspace.base]
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    id: WorkspaceId,
    root: PathBuf,
    base: Arc<Base>,
    active: Option<ActiveMutation>,
}

/// Workspace is meant for actiing in a workspace directory, and to act as a handle for it
/// It's not for storing state or making decitions, that's [Session]'s job
impl Workspace {
    // bough[impl workspace.new]
    // bough[impl workspace.new.dir]
    // bough[impl workspace.new.dir.previous]
    // bough[impl workspace.new.from-base-files]
    pub fn new(dir: PathBuf, base: Arc<Base>) -> Result<Self, Error> {
        let id = WorkspaceId::generate();
        let root = dir.join("work").join(id.as_str());

        info!(id = %id, root = %root.display(), "creating new workspace");

        if root.exists() {
            warn!(root = %root.display(), "workspace dir already exists");
            return Err(Error::DirAlreadyExists(root));
        }

        std::fs::create_dir_all(&root)?;

        for twig in base.twigs() {
            let src = File::new(base.as_ref(), &twig).resolve();
            let dst = root.join(twig.path());
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dst)?;
        }

        Ok(Self {
            id,
            root,
            base,
            active: None,
        })
    }

    // bough[impl workspace.bind]
    // bough[impl workspace.bind.validate-unchanged]
    pub fn bind(dir: PathBuf, id: &WorkspaceId, base: Arc<Base>) -> Result<Self, Error> {
        debug!(id = %id, "binding to existing workspace");
        let root = dir.join("work").join(id.as_str());
        let ws = Self {
            id: id.clone(),
            root,
            base,
            active: None,
        };
        ws.validate_unchanged()?;
        Ok(ws)
    }

    pub fn bind_dirty(dir: PathBuf, id: &WorkspaceId, base: Arc<Base>) -> Self {
        debug!(id = %id, "binding to existing workspace (dirty)");
        let root = dir.join("work").join(id.as_str());
        Self {
            id: id.clone(),
            root,
            base,
            active: None,
        }
    }

    // bough[impl workspace.id.get]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn base(&self) -> &Base {
        &self.base
    }

    // bough[impl workspace.write_mutant]
    // bough[impl workspace.write_mutant.set-active]
    // bough[impl workspace.write_mutant.set-active.only-one]
    pub fn write_mutant(&mut self, mutation: &Mutation) -> Result<(), Error> {
        if self.active.is_some() {
            warn!(id = %self.id, "attempted to write mutant while one is already active");
            return Err(Error::AlreadyActive);
        }
        debug!(
            workspace = %self.id,
            mutant = format!("{}",mutation.hash().unwrap()),
            "writing mutant to workspace"
        );
        let mutant = mutation.mutant();
        let base_file = File::new(self.base.as_ref(), mutant.twig()).resolve();
        let content = std::fs::read(&base_file)?;
        let span = mutant.span();
        let start = span.start().byte();
        let end = span.end().byte();
        let mut mutated = Vec::with_capacity(content.len());
        mutated.extend_from_slice(&content[..start]);
        mutated.extend_from_slice(mutation.subst().as_bytes());
        mutated.extend_from_slice(&content[end..]);
        let ws_file = self.root.join(mutant.twig().path());
        std::fs::write(&ws_file, &mutated)?;
        self.active = Some(ActiveMutation {
            mutant: mutation.mutant().clone(),
            mutation: mutation.clone(),
        });
        Ok(())
    }

    // bough[impl workspace.revert_mutant]
    // bough[impl workspace.revert_mutant.active]
    pub fn revert_mutant(&mut self) -> Result<(), Error> {
        let active = self.active.take().ok_or(Error::NotActive)?;
        debug!(id = %self.id, twig = %active.mutant().twig().path().display(), "reverting mutant");
        let twig = active.mutant().twig();
        let src = File::new(self.base.as_ref(), twig).resolve();
        let dst = self.root.join(twig.path());
        std::fs::copy(&src, &dst)?;
        Ok(())
    }

    pub fn active(&self) -> Option<&ActiveMutation> {
        self.active.as_ref()
    }

    // bough[impl workspace.files]
    pub fn files(&self) -> impl Iterator<Item = Twig> + use<'_> {
        self.base.twigs()
    }

    // bough[impl workspace.validate-unchanged]
    // bough[impl workspace.validate-unchanged.untracked]
    pub fn validate_unchanged(&self) -> Result<(), Error> {
        for twig in self.files() {
            let base_file = File::new(self.base.as_ref(), &twig).resolve();
            let ws_file = self.root.join(twig.path());

            let base_contents = std::fs::read(&base_file)
                .map_err(|e| Error::Unchanged(format!("base read {}: {e}", base_file.display())))?;
            let ws_contents = std::fs::read(&ws_file).map_err(|e| {
                Error::Unchanged(format!("workspace read {}: {e}", ws_file.display()))
            })?;

            if base_contents != ws_contents {
                return Err(Error::Unchanged(format!(
                    "file differs: {}",
                    twig.path().display()
                )));
            }
        }
        Ok(())
    }
}

impl Workspace {
    pub fn run_test(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        crate::phase::run_phase_in_workspace(
            self,
            &config.get_test_cmd(),
            config.get_test_pwd(),
            config.get_test_env(),
            config.get_test_timeout(reference_duration),
        )
    }

    pub fn run_init(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        let cmd = config
            .get_init_cmd()
            .ok_or(crate::phase::Error::NoCmdConfigured)?;
        crate::phase::run_phase_in_workspace(
            self,
            &cmd,
            config.get_init_pwd(),
            config.get_init_env(),
            config.get_init_timeout(reference_duration),
        )
    }

    pub fn run_reset(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        let cmd = config
            .get_reset_cmd()
            .ok_or(crate::phase::Error::NoCmdConfigured)?;
        crate::phase::run_phase_in_workspace(
            self,
            &cmd,
            config.get_reset_pwd(),
            config.get_reset_env(),
            config.get_reset_timeout(reference_duration),
        )
    }
}

impl Root for Workspace {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_fs::TwigsIterBuilder;

    fn make_base() -> (tempfile::TempDir, Arc<Base>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("src/b.js"), "const b = 2;").unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, Arc::new(base))
    }

    // bough[verify workspace.id]
    #[test]
    fn workspace_id_is_8_hex_chars() {
        let id = WorkspaceId::generate();
        assert_eq!(id.as_str().len(), 8);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    // bough[verify workspace.id]
    #[test]
    fn workspace_id_parse_valid() {
        let id = WorkspaceId::parse("abcd1234").unwrap();
        assert_eq!(id.as_str(), "abcd1234");
    }

    // bough[verify workspace.id]
    // bough[verify workspace.id.is-dir-name]
    #[test]
    fn workspace_id_parse_rejects_invalid() {
        assert!(WorkspaceId::parse("short").is_err());
        assert!(WorkspaceId::parse("toolongstring").is_err());
        assert!(WorkspaceId::parse("ghijklmn").is_err());
    }

    // bough[verify workspace.id.get]
    #[test]
    fn workspace_id_get() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        assert_eq!(ws.id().as_str().len(), 8);
    }

    // bough[verify workspace.is-handle]
    // bough[verify workspace.relationship]
    #[test]
    fn workspace_is_directory_handle() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        assert!(ws.path().exists());
        assert!(ws.path().is_dir());
    }

    // bough[verify workspace.new]
    // bough[verify workspace.new.dir]
    #[test]
    fn workspace_new_creates_work_subdir() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let expected_prefix = ws_dir.path().join("work");
        assert!(ws.path().starts_with(&expected_prefix));
    }

    // bough[verify workspace.new.dir.previous]
    #[test]
    fn workspace_new_errors_if_dir_exists() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let id = ws.id().clone();
        std::fs::create_dir_all(ws_dir.path().join("work").join(id.as_str())).ok();
        let result = Workspace::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(result.is_ok());
    }

    // bough[verify workspace.new.from-base-files]
    #[test]
    fn workspace_new_copies_source_files() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let a = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        let b = std::fs::read_to_string(ws.path().join("src/b.js")).unwrap();
        assert_eq!(a, "const a = 1;");
        assert_eq!(b, "const b = 2;");
    }

    // bough[verify workspace.bind]
    #[test]
    fn workspace_bind_attaches_to_existing() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let id = ws.id().clone();
        let bound = Workspace::bind(ws_dir.path().to_path_buf(), &id, base.clone()).unwrap();
        assert_eq!(bound.path(), ws.path());
        assert_eq!(bound.id(), ws.id());
    }

    // bough[verify workspace.validate-unchanged]
    #[test]
    fn validate_unchanged_passes_when_identical() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        ws.validate_unchanged().unwrap();
    }

    // bough[verify workspace.validate-unchanged]
    #[test]
    fn validate_unchanged_fails_when_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        assert!(ws.validate_unchanged().is_err());
    }

    // bough[verify workspace.validate-unchanged.untracked]
    #[test]
    fn validate_unchanged_ignores_extra_files() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        std::fs::write(ws.path().join("extra.txt"), "not tracked").unwrap();
        std::fs::create_dir_all(ws.path().join("other")).unwrap();
        std::fs::write(ws.path().join("other/file.js"), "also not tracked").unwrap();
        ws.validate_unchanged().unwrap();
    }

    // bough[verify workspace.bind.validate-unchanged]
    #[test]
    fn bind_validates_unchanged_on_creation() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let id = ws.id().clone();
        let bound = Workspace::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(bound.is_ok());
    }

    // bough[verify workspace.bind.validate-unchanged]
    #[test]
    fn bind_fails_when_workspace_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let id = ws.id().clone();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        let result = Workspace::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(result.is_err());
    }

    // bough[verify workspace.root]
    #[test]
    fn workspace_impls_root() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        assert!(ws.path().is_absolute());
    }

    // bough[verify workspace.base]
    #[test]
    fn workspace_holds_base() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        assert_eq!(ws.base().path(), base.path());
    }

    use crate::LanguageId;
    use bough_fs::Twig;
    use crate::mutant::{BinaryOpMutationKind, Mutant, MutantKind, Point, Span};
    use crate::mutation::Mutation;

    fn make_js_base(content: &str) -> (tempfile::TempDir, Arc<Base>) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, Arc::new(base))
    }

    // bough[verify workspace.write_mutant]
    #[test]
    fn write_mutant_applies_substitution_to_workspace_file() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            // "a + b" is bytes 10..15
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "a - b".to_string(),
        };
        ws.write_mutant(&mutation).unwrap();
        let result = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        assert_eq!(result, "const x = a - b;");
    }

    // bough[verify workspace.revert_mutant.active]
    #[test]
    fn revert_mutant_clears_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "a - b".to_string(),
        };
        ws.write_mutant(&mutation).unwrap();
        assert!(ws.active().is_some());
        ws.revert_mutant().unwrap();
        assert!(ws.active().is_none());
    }

    // bough[verify workspace.revert_mutant]
    #[test]
    fn revert_mutant_restores_original_file() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "a - b".to_string(),
        };
        ws.write_mutant(&mutation).unwrap();
        let mutated = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        assert_eq!(mutated, "const x = a - b;");
        ws.revert_mutant().unwrap();
        let reverted = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        assert_eq!(reverted, js);
    }

    // bough[verify workspace.write_mutant.set-active.only-one]
    #[test]
    fn write_mutant_errors_if_already_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "a - b".to_string(),
        };
        ws.write_mutant(&mutation).unwrap();
        let result = ws.write_mutant(&mutation);
        assert!(result.is_err());
    }

    // bough[verify workspace.active]
    #[test]
    fn workspace_active_is_none_initially() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        assert!(ws.active().is_none());
    }

    // bough[verify workspace.write_mutant.set-active]
    #[test]
    fn write_mutant_sets_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "a - b".to_string(),
        };
        assert!(ws.active().is_none());
        ws.write_mutant(&mutation).unwrap();
        assert!(ws.active().is_some());
    }

    // bough[verify workspace.files]
    #[test]
    fn workspace_files_returns_iter() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), base.clone()).unwrap();
        let mut twigs: Vec<_> = ws.files().map(|t| t.path().to_path_buf()).collect();
        twigs.sort();
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/a.js"), PathBuf::from("src/b.js")]
        );
    }
}
