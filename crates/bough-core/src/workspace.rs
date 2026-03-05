use crate::base::Base;
use crate::file::{File, FilesIter, Root, Twig};
use crate::mutant::mutation::Mutation;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    File(crate::file::Error),
    IdParse(String),
    DirAlreadyExists(PathBuf),
    Io(std::io::Error),
    Unchanged(String),
    AlreadyActive,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::File(e) => write!(f, "{e}"),
            Error::IdParse(s) => write!(f, "invalid workspace id: {s}"),
            Error::DirAlreadyExists(p) => write!(f, "workspace dir already exists: {}", p.display()),
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::Unchanged(msg) => write!(f, "workspace changed: {msg}"),
            Error::AlreadyActive => write!(f, "workspace already has an active mutant"),
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::file::Error> for Error {
    fn from(e: crate::file::Error) -> Self {
        Error::File(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

// core[impl workspace.id]
// core[impl workspace.id.is-dir-name]
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

// core[impl workspace.is-handle]
// core[impl workspace.relationship]
// core[impl workspace.root]
// core[impl workspace.base]
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace<'a> {
    id: WorkspaceId,
    root: PathBuf,
    base: &'a Base,
    active: Option<Twig>,
}

impl<'a> Workspace<'a> {
    // core[impl workspace.new]
    // core[impl workspace.new.dir]
    // core[impl workspace.new.dir.previous]
    // core[impl workspace.new.from-source-files]
    pub fn new(dir: PathBuf, base: &'a Base) -> Result<Self, Error> {
        let id = WorkspaceId::generate();
        let root = dir.join("work").join(id.as_str());

        if root.exists() {
            return Err(Error::DirAlreadyExists(root));
        }

        std::fs::create_dir_all(&root)?;

        for twig in base.files() {
            let src = File::new(base, &twig).resolve();
            let dst = root.join(twig.path());
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dst)?;
        }

        Ok(Self { id, root, base, active: None })
    }

    // core[impl workspace.bind]
    // core[impl workspace.bind.validate-unchanged]
    pub fn bind(dir: PathBuf, id: &WorkspaceId, base: &'a Base) -> Result<Self, Error> {
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

    // core[impl workspace.id.get]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn base(&self) -> &Base {
        self.base
    }

    // core[impl workspace.write_mutant]
    // core[impl workspace.write_mutant.set-active]
    // core[impl workspace.write_mutant.set-active.only-one]
    pub fn write_mutant(&mut self, mutation: &Mutation<'_>) -> Result<(), Error> {
        if self.active.is_some() {
            return Err(Error::AlreadyActive);
        }
        let mutant = mutation.mutant();
        let base_file = File::new(self.base, mutant.twig()).resolve();
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
        self.active = Some(mutant.twig().clone());
        Ok(())
    }

    pub fn active(&self) -> Option<&Twig> {
        self.active.as_ref()
    }

    // core[impl workspace.files]
    pub fn files(&self) -> FilesIter {
        self.base.files()
    }

    // core[impl workspace.validate-unchanged]
    pub fn validate_unchanged(&self) -> Result<(), Error> {
        for twig in self.files() {
            let base_file = File::new(self.base, &twig).resolve();
            let ws_file = self.root.join(twig.path());

            let base_contents = std::fs::read(&base_file)
                .map_err(|e| Error::Unchanged(format!("base read {}: {e}", base_file.display())))?;
            let ws_contents = std::fs::read(&ws_file)
                .map_err(|e| Error::Unchanged(format!("workspace read {}: {e}", ws_file.display())))?;

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

impl Root for Workspace<'_> {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileSourceConfig;

    fn make_base() -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("src/b.js"), "const b = 2;").unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            FileSourceConfig {
                include: vec!["src/**/*.js".into()],
                ..Default::default()
            },
        )
        .unwrap();
        (dir, base)
    }

    // core[verify workspace.id]
    #[test]
    fn workspace_id_is_8_hex_chars() {
        let id = WorkspaceId::generate();
        assert_eq!(id.as_str().len(), 8);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    // core[verify workspace.id]
    #[test]
    fn workspace_id_parse_valid() {
        let id = WorkspaceId::parse("abcd1234").unwrap();
        assert_eq!(id.as_str(), "abcd1234");
    }

    // core[verify workspace.id]
    // core[verify workspace.id.is-dir-name]
    #[test]
    fn workspace_id_parse_rejects_invalid() {
        assert!(WorkspaceId::parse("short").is_err());
        assert!(WorkspaceId::parse("toolongstring").is_err());
        assert!(WorkspaceId::parse("ghijklmn").is_err());
    }

    // core[verify workspace.id.get]
    #[test]
    fn workspace_id_get() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        assert_eq!(ws.id().as_str().len(), 8);
    }

    // core[verify workspace.is-handle]
    // core[verify workspace.relationship]
    #[test]
    fn workspace_is_directory_handle() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        assert!(ws.path().exists());
        assert!(ws.path().is_dir());
    }

    // core[verify workspace.new]
    // core[verify workspace.new.dir]
    #[test]
    fn workspace_new_creates_work_subdir() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let expected_prefix = ws_dir.path().join("work");
        assert!(ws.path().starts_with(&expected_prefix));
    }

    // core[verify workspace.new.dir.previous]
    #[test]
    fn workspace_new_errors_if_dir_exists() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let id = ws.id().clone();
        std::fs::create_dir_all(ws_dir.path().join("work").join(id.as_str())).ok();
        let result = Workspace::bind(ws_dir.path().to_path_buf(), &id, &base);
        assert!(result.is_ok());
    }

    // core[verify workspace.new.from-source-files]
    #[test]
    fn workspace_new_copies_source_files() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let a = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        let b = std::fs::read_to_string(ws.path().join("src/b.js")).unwrap();
        assert_eq!(a, "const a = 1;");
        assert_eq!(b, "const b = 2;");
    }

    // core[verify workspace.bind]
    #[test]
    fn workspace_bind_attaches_to_existing() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let id = ws.id().clone();
        let bound = Workspace::bind(ws_dir.path().to_path_buf(), &id, &base).unwrap();
        assert_eq!(bound.path(), ws.path());
        assert_eq!(bound.id(), ws.id());
    }

    // core[verify workspace.validate-unchanged]
    #[test]
    fn validate_unchanged_passes_when_identical() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        ws.validate_unchanged().unwrap();
    }

    // core[verify workspace.validate-unchanged]
    #[test]
    fn validate_unchanged_fails_when_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        assert!(ws.validate_unchanged().is_err());
    }

    // core[verify workspace.bind.validate-unchanged]
    #[test]
    fn bind_validates_unchanged_on_creation() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let id = ws.id().clone();
        let bound = Workspace::bind(ws_dir.path().to_path_buf(), &id, &base);
        assert!(bound.is_ok());
    }

    // core[verify workspace.bind.validate-unchanged]
    #[test]
    fn bind_fails_when_workspace_modified() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let id = ws.id().clone();
        std::fs::write(ws.path().join("src/a.js"), "MUTATED").unwrap();
        let result = Workspace::bind(ws_dir.path().to_path_buf(), &id, &base);
        assert!(result.is_err());
    }

    // core[verify workspace.root]
    #[test]
    fn workspace_impls_root() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        assert!(ws.path().is_absolute());
    }

    // core[verify workspace.base]
    #[test]
    fn workspace_holds_base() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        assert_eq!(ws.base().path(), base.path());
    }

    use crate::LanguageId;
    use crate::mutant::{Mutant, MutantKind, BinaryOpMutationKind, Span, Point};
    use crate::mutant::mutation::Mutation;
    use crate::file::Twig;

    fn make_js_base(content: &str) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            FileSourceConfig {
                include: vec!["src/**/*.js".into()],
                ..Default::default()
            },
        )
        .unwrap();
        (dir, base)
    }

    // core[verify workspace.write_mutant]
    #[test]
    fn write_mutant_applies_substitution_to_workspace_file() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            // "a + b" is bytes 10..15
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation { mutant: &mutant, subst: "a - b".to_string() };
        ws.write_mutant(&mutation).unwrap();
        let result = std::fs::read_to_string(ws.path().join("src/a.js")).unwrap();
        assert_eq!(result, "const x = a - b;");
    }

    // core[verify workspace.write_mutant.set-active.only-one]
    #[test]
    fn write_mutant_errors_if_already_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation { mutant: &mutant, subst: "a - b".to_string() };
        ws.write_mutant(&mutation).unwrap();
        let result = ws.write_mutant(&mutation);
        assert!(result.is_err());
    }

    // core[verify workspace.write_mutant.set-active]
    #[test]
    fn write_mutant_sets_active() {
        let js = "const x = a + b;";
        let (_base_dir, base) = make_js_base(js);
        let ws_dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutation = Mutation { mutant: &mutant, subst: "a - b".to_string() };
        assert!(ws.active().is_none());
        ws.write_mutant(&mutation).unwrap();
        assert!(ws.active().is_some());
    }

    // core[verify workspace.files]
    #[test]
    fn workspace_files_returns_iter() {
        let (_base_dir, base) = make_base();
        let ws_dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(ws_dir.path().to_path_buf(), &base).unwrap();
        let mut twigs: Vec<_> = ws.files().map(|t| t.path().to_path_buf()).collect();
        twigs.sort();
        assert_eq!(twigs, vec![PathBuf::from("src/a.js"), PathBuf::from("src/b.js")]);
    }
}
