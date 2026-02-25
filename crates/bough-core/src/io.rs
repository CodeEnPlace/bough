use bough_sha::ShaHashable;
use std::path::PathBuf;

pub fn hashed_path(dir: &PathBuf, content: &str, label: &str) -> PathBuf {
    dir.join(format!("{}.{label}.json", content.sha_hash()))
}
