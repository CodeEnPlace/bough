use bough_core::SourceFile;
use bough_core::config::Config;
use bough_core::languages::LanguageId;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    Glob(glob::PatternError),
    ReadFile(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::Glob(e) => write!(f, "invalid glob pattern: {e}"),
            Error::ReadFile(path, e) => write!(f, "failed to read {}: {e}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct ShowSrcFiles {
    pub files: BTreeMap<LanguageId, Vec<SourceFile>>,
}

fn collect_glob(pattern: &str, base: &Path) -> Result<Vec<PathBuf>, Error> {
    let full = if Path::new(pattern).is_absolute() {
        pattern.to_string()
    } else {
        format!("{}/{pattern}", base.display())
    };
    let paths = glob::glob(&full)
        .map_err(Error::Glob)?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .collect();
    Ok(paths)
}

pub fn run(config: &Config) -> Result<ShowSrcFiles, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner = config.runner(runner_name);
    let runner_pwd = config.resolve_pwd(runner, None);

    let mut files: BTreeMap<LanguageId, Vec<SourceFile>> = BTreeMap::new();

    for lang in config.mutate_languages(runner_name) {
        let mut included = Vec::new();
        for pattern in &config.file_includes(runner_name, lang) {
            included.extend(collect_glob(pattern, runner_pwd)?);
        }

        let mut excluded = std::collections::HashSet::new();
        for pattern in &config.file_excludes(runner_name, lang) {
            for path in collect_glob(pattern, runner_pwd)? {
                excluded.insert(path);
            }
        }

        let mut paths: Vec<PathBuf> = included
            .into_iter()
            .filter(|p| !excluded.contains(p))
            .collect();
        paths.sort();
        paths.dedup();

        let mut src_files = Vec::with_capacity(paths.len());
        for path in paths {
            let sf = SourceFile::read(&path, lang).map_err(|e| Error::ReadFile(path, e))?;
            src_files.push(sf);
        }
        files.insert(lang, src_files);
    }

    Ok(ShowSrcFiles { files })
}

impl Render for ShowSrcFiles {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(&self.files).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        let mut out = String::new();
        for (lang, paths) in &self.files {
            out.push_str(&format!(
                "found {} files for {}\n",
                color("\x1b[1m", &paths.len().to_string()),
                color("\x1b[36m", &format!("{lang:?}")),
            ));
        }
        out
    }

    fn render_verbose(&self) -> String {
        let mut out = String::new();
        for (lang, files) in &self.files {
            out.push_str(&color(
                "\x1b[1m",
                &format!("{lang:?} ({} files)", files.len()),
            ));
            out.push('\n');
            for f in files {
                out.push_str(&format!(
                    "  {}\n",
                    f.path.display(),
                ));
            }
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 2).min(6) as usize);
        let mut out = String::new();

        out.push_str(&format!(
            "{} Src Files\n\n",
            "#".repeat((depth + 1).min(6) as usize),
        ));

        for (lang, files) in &self.files {
            out.push_str(&format!("{heading} {lang:?}\n\n"));
            for f in files {
                out.push_str(&format!("- `{}`\n", f.path.display()));
            }
            out.push('\n');
        }
        out
    }
}
