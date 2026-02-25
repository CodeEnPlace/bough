use crate::Hash;
use std::path::PathBuf;

pub fn hashed_path(dir: &PathBuf, content: &str, label: &str) -> PathBuf {
    dir.join(format!("{}.{label}.json", Hash::of(content)))
}
