use std::collections::HashMap;
use std::path::PathBuf;

// core[impl fds.type]
pub struct FacetDiskStore<Key, Val>
where
    Key: for<'a> facet::Facet<'a> + std::fmt::Display + Eq + std::hash::Hash + Clone,
    Val: for<'a> facet::Facet<'a> + Clone,
{
    dir: PathBuf,
    cache: HashMap<Key, Val>,
}

// core[impl fds.new]
// core[impl fds.get]
// core[impl fds.set]
// core[impl fds.keys]
// core[impl fds.files]
impl<Key, Val> FacetDiskStore<Key, Val>
where
    Key: for<'a> facet::Facet<'a> + std::fmt::Display + Eq + std::hash::Hash + Clone,
    Val: for<'a> facet::Facet<'a> + Clone,
{
    pub fn new(dir: PathBuf) -> Self {
        todo!()
    }

    pub fn get(&self, key: &Key) -> Option<&Val> {
        todo!()
    }

    pub fn set(&mut self, key: Key, val: Val) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        todo!();
        #[allow(unreachable_code)]
        std::iter::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash, facet::Facet)]
    struct TestKey(String);

    impl std::fmt::Display for TestKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[derive(Debug, Clone, PartialEq, facet::Facet)]
    struct TestVal {
        data: String,
        count: u32,
    }

    // core[verify fds.type]
    // core[verify fds.new]
    #[test]
    fn new_creates_store_pointing_at_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        assert_eq!(store.keys().count(), 0);
    }

    // core[verify fds.set]
    // core[verify fds.get]
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
        assert_eq!(got, &val);
    }

    // core[verify fds.get]
    #[test]
    fn get_missing_key_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store: FacetDiskStore<TestKey, TestVal> = FacetDiskStore::new(dir.path().to_path_buf());
        assert!(store.get(&TestKey("nope".into())).is_none());
    }

    // core[verify fds.keys]
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
        let mut keys: Vec<_> = store.keys().map(|k| k.0.clone()).collect();
        keys.sort();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    // core[verify fds.files]
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

    // core[verify fds.set]
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
}
