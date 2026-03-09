use std::path::PathBuf;
use tracing::{debug, trace, warn};

// bough[impl fds.type]
pub struct FacetDiskStore<Key, Val>
where
    Key: std::fmt::Display + std::str::FromStr,
    Val: for<'a> facet::Facet<'a>,
{
    dir: PathBuf,
    _phantom: std::marker::PhantomData<(Key, Val)>,
}

// bough[impl fds.live]
impl<Key, Val> FacetDiskStore<Key, Val>
where
    Key: std::fmt::Display + std::str::FromStr,
    Val: for<'a> facet::Facet<'a>,
{
    // bough[impl fds.new]
    // bough[impl fds.new.mkdir]
    // bough[impl fds.live.startup]
    pub fn new(dir: PathBuf) -> Self {
        debug!(dir = %dir.display(), "creating facet disk store");
        std::fs::create_dir_all(&dir).ok();
        Self {
            dir,
            _phantom: std::marker::PhantomData,
        }
    }

    // bough[impl fds.get]
    // bough[impl fds.get.invalid]
    // bough[impl fds.live.intercepted]
    pub fn get(&self, key: &Key) -> Option<Val> {
        let path = self.dir.join(format!("{key}.json"));
        trace!(key = %key, path = %path.display(), "getting from disk store");
        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => {
                trace!(key = %key, "key not found on disk");
                return None;
            }
        };
        match facet_json::from_str(&data) {
            Ok(v) => Some(v),
            Err(e) => {
                warn!(key = %key, error = %e, "failed to deserialize from disk store");
                None
            }
        }
    }

    // bough[impl fds.set]
    // bough[impl fds.files]
    pub fn set(&mut self, key: Key, val: Val) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!("{key}.json"));
        trace!(key = %key, path = %path.display(), "writing to disk store");
        let json = facet_json::to_string(&val)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    // bough[impl fds.remove]
    pub fn remove(&mut self, key: &Key) -> Option<Val> {
        let path = self.dir.join(format!("{key}.json"));
        trace!(key = %key, path = %path.display(), "removing from disk store");
        let data = std::fs::read_to_string(&path).ok()?;
        let val: Val = facet_json::from_str(&data).ok()?;
        std::fs::remove_file(path).ok()?;
        Some(val)
    }

    // bough[impl fds.keys]
    // bough[impl fds.keys.invalid]
    pub fn keys(&self) -> impl Iterator<Item = Key> {
        let read_dir = match std::fs::read_dir(&self.dir) {
            Ok(rd) => rd,
            Err(_) => return Vec::new().into_iter(),
        };
        read_dir
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let stem = name.strip_suffix(".json")?;
                stem.parse().ok()
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestKey(String);

    impl std::fmt::Display for TestKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::str::FromStr for TestKey {
        type Err = std::convert::Infallible;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(TestKey(s.to_owned()))
        }
    }

    #[derive(Debug, Clone, PartialEq, facet::Facet)]
    struct TestVal {
        data: String,
        count: u32,
    }

    // bough[verify fds.type]
    // bough[verify fds.new]
    #[test]
    fn new_creates_store_pointing_at_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        assert_eq!(store.keys().count(), 0);
    }

    // bough[verify fds.set]
    // bough[verify fds.get]
    #[test]
    fn set_then_get_returns_value() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let key = TestKey("hello".into());
        let val = TestVal {
            data: "world".into(),
            count: 42,
        };
        store.set(key.clone(), val.clone()).unwrap();
        let got = store.get(&key).unwrap();
        assert_eq!(got, val);
    }

    // bough[verify fds.get]
    #[test]
    fn get_missing_key_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        assert!(store.get(&TestKey("nope".into())).is_none());
    }

    // bough[verify fds.keys]
    #[test]
    fn keys_iterates_inserted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let val = TestVal {
            data: "x".into(),
            count: 1,
        };
        store.set(TestKey("a".into()), val.clone()).unwrap();
        store.set(TestKey("b".into()), val.clone()).unwrap();
        store.set(TestKey("c".into()), val).unwrap();
        let mut keys: Vec<_> = store.keys().map(|k| k.0).collect();
        keys.sort();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    // bough[verify fds.files]
    #[test]
    fn stores_as_json_files_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let key = TestKey("mykey".into());
        let val = TestVal {
            data: "test".into(),
            count: 7,
        };
        store.set(key, val).unwrap();
        let file_path = dir.path().join("mykey.json");
        assert!(file_path.exists(), "expected {file_path:?} to exist");
        let contents = std::fs::read_to_string(&file_path).unwrap();
        let roundtrip: TestVal = facet_json::from_str(&contents).unwrap();
        assert_eq!(roundtrip.data, "test");
        assert_eq!(roundtrip.count, 7);
    }

    // bough[verify fds.set]
    #[test]
    fn set_overwrites_existing_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let key = TestKey("k".into());
        store
            .set(
                key.clone(),
                TestVal {
                    data: "old".into(),
                    count: 1,
                },
            )
            .unwrap();
        store
            .set(
                key.clone(),
                TestVal {
                    data: "new".into(),
                    count: 2,
                },
            )
            .unwrap();
        let got = store.get(&key).unwrap();
        assert_eq!(got.data, "new");
        assert_eq!(got.count, 2);
    }

    // bough[verify fds.live]
    // bough[verify fds.live.intercepted]
    #[test]
    fn get_reads_from_disk_not_cache() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let key = TestKey("live".into());
        let val = TestVal {
            data: "original".into(),
            count: 1,
        };
        store.set(key.clone(), val).unwrap();

        let modified = r#"{"data":"modified","count":99}"#;
        std::fs::write(dir.path().join("live.json"), modified).unwrap();

        let got = store.get(&key).unwrap();
        assert_eq!(got.data, "modified");
        assert_eq!(got.count, 99);
    }

    // bough[verify fds.remove]
    #[test]
    fn remove_returns_val_and_deletes_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        let key = TestKey("rm".into());
        let val = TestVal {
            data: "gone".into(),
            count: 3,
        };
        store.set(key.clone(), val.clone()).unwrap();
        assert!(dir.path().join("rm.json").exists());

        let removed = store.remove(&key).unwrap();
        assert_eq!(removed, val);
        assert!(!dir.path().join("rm.json").exists());
        assert!(store.get(&key).is_none());
    }

    // bough[verify fds.remove]
    #[test]
    fn remove_missing_key_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let mut store: FacetDiskStore<TestKey, TestVal> =
            FacetDiskStore::new(dir.path().to_path_buf());
        assert!(store.remove(&TestKey("nope".into())).is_none());
    }

    // bough[verify fds.keys.invalid]
    #[test]
    fn keys_skips_invalidly_named_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.json"), r#"{"data":"x","count":1}"#).unwrap();
        std::fs::write(dir.path().join("no_extension"), "garbage").unwrap();
        std::fs::write(dir.path().join("wrong.txt"), "garbage").unwrap();
        std::fs::write(
            dir.path().join("also_good.json"),
            r#"{"data":"y","count":2}"#,
        )
        .unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        let mut keys: Vec<_> = store.keys().map(|k| k.0).collect();
        keys.sort();
        assert_eq!(keys, vec!["also_good", "good"]);
    }

    // bough[verify fds.get.invalid]
    #[test]
    fn get_returns_none_for_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.json"), "not valid json!!!").unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        assert!(store.get(&TestKey("bad".into())).is_none());
    }

    // bough[verify fds.new.mkdir]
    #[test]
    fn new_creates_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store_dir = dir.path().join("nested/deep/store");
        assert!(!store_dir.exists());
        let _store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(store_dir.clone());
        assert!(store_dir.exists());
    }

    // bough[verify fds.live.startup]
    #[test]
    fn discovers_preexisting_files() {
        let dir = tempfile::tempdir().unwrap();
        let val_json = r#"{"data":"preexisting","count":5}"#;
        std::fs::write(dir.path().join("old.json"), val_json).unwrap();

        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());

        let got = store.get(&TestKey("old".into())).unwrap();
        assert_eq!(got.data, "preexisting");
        assert_eq!(got.count, 5);

        let keys: Vec<_> = store.keys().map(|k| k.0).collect();
        assert_eq!(keys, vec!["old"]);
    }
}
