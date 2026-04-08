use crate::base::Base;
use bough_config::SessionConfig;
use bough_core::{Mutant, Mutation};
use bough_fs::{File, Root, Twig};
use bough_typed_hash::TypedHashable;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkId(String);

impl WorkId {
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

impl std::fmt::Display for WorkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Work {
    id: WorkId,
    root: PathBuf,
    base: Arc<Base>,
    active: Option<ActiveMutation>,
}

/// Workspace is meant for acting in a workspace directory, and to act as a handle for it.
/// It's not for storing state or making decisions — that's `Session`'s job.
impl Work {
    pub fn new(
        dir: PathBuf,
        base: Arc<Base>,
        config: &impl SessionConfig,
    ) -> Result<Self, Error> {
        let id = WorkId::generate();
        let root = dir.join("work").join(id.as_str());

        info!(id = %id, root = %root.display(), "creating new workspace");

        if root.exists() {
            warn!(root = %root.display(), "workspace dir already exists");
            return Err(Error::DirAlreadyExists(root));
        }

        std::fs::create_dir_all(&root)?;

        let twigs: Vec<Twig> = base.twigs().collect();
        for twig in &twigs {
            if let Some(parent) = root.join(twig.path()).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.threads().max(1) as usize)
            .build()
            .expect("build rayon pool");
        pool.install(|| -> Result<(), Error> {
            use rayon::prelude::*;
            twigs.par_iter().try_for_each(|twig| -> Result<(), Error> {
                let src = File::new(base.as_ref(), twig).resolve();
                let dst = root.join(twig.path());
                std::fs::copy(&src, &dst)?;
                Ok(())
            })
        })?;

        Ok(Self {
            id,
            root,
            base,
            active: None,
        })
    }

    pub fn bind(dir: PathBuf, id: &WorkId, base: Arc<Base>) -> Result<Self, Error> {
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

    pub fn bind_dirty(dir: PathBuf, id: &WorkId, base: Arc<Base>) -> Self {
        debug!(id = %id, "binding to existing workspace (dirty)");
        let root = dir.join("work").join(id.as_str());
        Self {
            id: id.clone(),
            root,
            base,
            active: None,
        }
    }

    pub fn id(&self) -> &WorkId {
        &self.id
    }

    pub fn base(&self) -> &Base {
        &self.base
    }

    pub fn write_mutant(&mut self, mutation: &Mutation) -> Result<(), Error> {
        if self.active.is_some() {
            warn!(id = %self.id, "attempted to write mutant while one is already active");
            return Err(Error::AlreadyActive);
        }
        debug!(
            workspace = %self.id,
            mutant = format!("{}", mutation.hash().unwrap()),
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

    pub fn files(&self) -> impl Iterator<Item = Twig> + use<'_> {
        self.base.twigs()
    }

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

impl Root for Work {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_core::TwigsIterBuilder;

    struct TestConfig;
    impl SessionConfig for TestConfig {
        fn get_workers_count(&self) -> u64 { 1 }
        fn threads(&self) -> u32 { 2 }
        fn get_bough_state_dir(&self) -> PathBuf { PathBuf::new() }
        fn get_base_root_path(&self) -> PathBuf { PathBuf::new() }
        fn get_base_include_globs(&self) -> impl Iterator<Item = String> { std::iter::empty() }
        fn get_base_exclude_globs(&self) -> impl Iterator<Item = String> { std::iter::empty() }
        fn get_langs(&self) -> impl Iterator<Item = bough_core::LanguageId> { std::iter::empty() }
        fn get_lang_include_globs(&self, _: bough_core::LanguageId) -> impl Iterator<Item = String> { std::iter::empty() }
        fn get_lang_exclude_globs(&self, _: bough_core::LanguageId) -> impl Iterator<Item = String> { std::iter::empty() }
        fn get_lang_skip_queries(&self, _: bough_core::LanguageId) -> impl Iterator<Item = String> { std::iter::empty() }
        fn get_test_cmd(&self) -> String { String::new() }
        fn get_test_pwd(&self) -> PathBuf { PathBuf::new() }
        fn get_test_env(&self) -> std::collections::HashMap<String, String> { Default::default() }
        fn get_test_timeout(&self, _: Option<chrono::Duration>) -> chrono::Duration { chrono::Duration::seconds(0) }
        fn get_init_cmd(&self) -> Option<String> { None }
        fn get_init_pwd(&self) -> PathBuf { PathBuf::new() }
        fn get_init_env(&self) -> std::collections::HashMap<String, String> { Default::default() }
        fn get_init_timeout(&self, _: Option<chrono::Duration>) -> chrono::Duration { chrono::Duration::seconds(0) }
        fn get_reset_cmd(&self) -> Option<String> { None }
        fn get_reset_pwd(&self) -> PathBuf { PathBuf::new() }
        fn get_reset_env(&self) -> std::collections::HashMap<String, String> { Default::default() }
        fn get_reset_timeout(&self, _: Option<chrono::Duration>) -> chrono::Duration { chrono::Duration::seconds(0) }
        fn get_find_number(&self) -> usize { 0 }
        fn get_find_number_per_file(&self) -> usize { 0 }
        fn get_find_factors(&self) -> Vec<bough_config::Factor> { Vec::new() }
    }
    fn test_config() -> TestConfig { TestConfig }

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

    #[test]
    fn workspace_id_is_8_hex_chars() {
        let id = WorkId::generate();
        assert_eq!(id.as_str().len(), 8);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn workspace_id_parse_valid() {
        let id = WorkId::parse("abcd1234").unwrap();
        assert_eq!(id.as_str(), "abcd1234");
    }

    #[test]
    fn workspace_id_parse_rejects_invalid() {
        assert!(WorkId::parse("short").is_err());
        assert!(WorkId::parse("toolongstring").is_err());
        assert!(WorkId::parse("ghijklmn").is_err());
    }

    #[test]
    fn workspace_id_get() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        assert_eq!(ws.id().as_str().len(), 8);
    }

    #[test]
    fn workspace_is_directory_handle() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        assert!(ws.path().exists());
        assert!(ws.path().is_dir());
    }

    #[test]
    fn workspace_new_creates_work_subdir() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let expected_prefix = ws_dir.path().join("work");
        assert!(ws.path().starts_with(&expected_prefix));
    }

    #[test]
    fn workspace_new_errors_if_dir_exists() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let id = ws.id().clone();
        std::fs::create_dir_all(ws_dir.path().join("work").join(id.as_str())).ok();
        let result = Work::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(result.is_ok());
    }

    #[test]
    fn workspace_new_copies_source_files() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let a = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        let b = std::fs::read_to_string(ws.path().join("src/b.js")).unwrap();
        assert_eq!(a, "const a = 1;");
        assert_eq!(b, "const b = 2;");
    }

    #[test]
    fn workspace_bind_attaches_to_existing() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let id = ws.id().clone();
        let bound = Work::bind(ws_dir.path().to_path_buf(), &id, base.clone()).unwrap();
        assert_eq!(bound.path(), ws.path());
        assert_eq!(bound.id(), ws.id());
    }

    #[test]
    fn validate_unchanged_passes_when_identical() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        ws.validate_unchanged().unwrap();
    }

    #[test]
    fn validate_unchanged_fails_when_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        assert!(ws.validate_unchanged().is_err());
    }

    #[test]
    fn validate_unchanged_ignores_extra_files() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        std::fs::write(ws.path().join("extra.txt"), "not tracked").unwrap();
        std::fs::create_dir_all(ws.path().join("other")).unwrap();
        std::fs::write(ws.path().join("other/file.js"), "also not tracked").unwrap();
        ws.validate_unchanged().unwrap();
    }

    #[test]
    fn bind_validates_unchanged_on_creation() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let id = ws.id().clone();
        let bound = Work::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(bound.is_ok());
    }

    #[test]
    fn bind_fails_when_workspace_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let id = ws.id().clone();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        let result = Work::bind(ws_dir.path().to_path_buf(), &id, base.clone());
        assert!(result.is_err());
    }

    #[test]
    fn workspace_impls_root() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        assert!(ws.path().is_absolute());
    }

    #[test]
    fn workspace_holds_base() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        assert_eq!(ws.base().path(), base.path());
    }

    use bough_core::LanguageId;
    use bough_core::Mutation;
    use bough_core::mutant::{BinaryOpMutationKind, Mutant, MutantKind, Point, Span};
    use bough_fs::Twig;

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

    #[test]
    fn write_mutant_applies_substitution_to_workspace_file() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
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

    #[test]
    fn revert_mutant_clears_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
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

    #[test]
    fn revert_mutant_restores_original_file() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
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

    #[test]
    fn write_mutant_errors_if_already_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
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

    #[test]
    fn workspace_active_is_none_initially() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        assert!(ws.active().is_none());
    }

    #[test]
    fn write_mutant_sets_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
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

    #[test]
    fn workspace_files_returns_iter() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Work::new(ws_dir.path().to_path_buf(), base.clone(), &test_config()).unwrap();
        let mut twigs: Vec<_> = ws.files().map(|t| t.path().to_path_buf()).collect();
        twigs.sort();
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/a.js"), PathBuf::from("src/b.js")]
        );
    }
}
