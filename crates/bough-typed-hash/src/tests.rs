use super::*;

#[derive(TypedHash)]
struct TestHash([u8; 32]);

#[derive(Clone, TypedHashable)]
struct Widget {
    name: String,
    count: u32,
}

#[derive(Clone, HashInto)]
struct Part {
    label: String,
    weight: f64,
}

#[derive(Clone, TypedHashable)]
struct Assembly {
    part: Part,
    quantity: u32,
}

#[test]
fn hash_into_deterministic() {
    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    "hello".hash_into(&mut s1).unwrap();
    "hello".hash_into(&mut s2).unwrap();
    let r1: [u8; 32] = s1.finalize().into();
    let r2: [u8; 32] = s2.finalize().into();
    assert_eq!(r1, r2);
}

#[test]
fn hash_into_different_inputs_differ() {
    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    "hello".hash_into(&mut s1).unwrap();
    "world".hash_into(&mut s2).unwrap();
    let r1: [u8; 32] = s1.finalize().into();
    let r2: [u8; 32] = s2.finalize().into();
    assert_ne!(r1, r2);
}

#[test]
fn typed_hash_display_is_64_hex() {
    let h = TestHash::from_raw([0xab; 32]);
    let s = h.to_string();
    assert_eq!(s.len(), 64);
    assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn typed_hash_debug() {
    let h = TestHash::from_raw([0; 32]);
    let d = format!("{h:?}");
    assert!(d.starts_with("TestHash("));
}

#[test]
fn typed_hash_clone_eq() {
    let h1 = TestHash::from_raw([1; 32]);
    let h2 = h1.clone();
    assert_eq!(h1, h2);
}

#[test]
fn typed_hash_serialize() {
    let h = TestHash::from_raw([0xab; 32]);
    let json = serde_json::to_string(&h).unwrap();
    assert_eq!(json, format!("\"{}\"", h));
}

#[test]
fn typed_hashable_produces_hash() {
    let w = Widget { name: "gear".into(), count: 5 };
    let mut store = MemoryHashStore::new();
    let h = w.hash(&mut store).unwrap();
    assert_eq!(h.to_string().len(), 64);
}

#[test]
fn typed_hashable_deterministic() {
    let a = Widget { name: "gear".into(), count: 5 };
    let b = Widget { name: "gear".into(), count: 5 };
    assert_eq!(a.hash(&mut MemoryHashStore::new()).unwrap(), b.hash(&mut MemoryHashStore::new()).unwrap());
}

#[test]
fn typed_hashable_different_values_differ() {
    let a = Widget { name: "gear".into(), count: 5 };
    let b = Widget { name: "gear".into(), count: 6 };
    assert_ne!(a.hash(&mut MemoryHashStore::new()).unwrap(), b.hash(&mut MemoryHashStore::new()).unwrap());
}

#[test]
fn typed_hashable_hash_inserts_into_store() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "auto".into(), count: 1 };
    let h = w.hash(&mut MemoryHashStore::new()).unwrap();
    assert!(!store.contains(&h));
    w.hash(&mut store).unwrap();
    assert!(store.contains(&h));
}

#[test]
fn hash_into_derive_nested() {
    let a = Assembly { part: Part { label: "bolt".into(), weight: 1.5 }, quantity: 10 };
    let b = Assembly { part: Part { label: "bolt".into(), weight: 1.5 }, quantity: 10 };
    let c = Assembly { part: Part { label: "nut".into(), weight: 0.5 }, quantity: 10 };
    assert_eq!(a.hash(&mut MemoryHashStore::new()).unwrap(), b.hash(&mut MemoryHashStore::new()).unwrap());
    assert_ne!(a.hash(&mut MemoryHashStore::new()).unwrap(), c.hash(&mut MemoryHashStore::new()).unwrap());
}

#[test]
fn hash_into_derive_enum() {
    #[derive(HashInto)]
    enum Shape {
        Circle(f64),
        Rect { w: f64, h: f64 },
        #[allow(dead_code)]
        Point,
    }

    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    let mut s3 = Sha256::new();
    Shape::Circle(1.0).hash_into(&mut s1).unwrap();
    Shape::Circle(1.0).hash_into(&mut s2).unwrap();
    Shape::Rect { w: 1.0, h: 1.0 }.hash_into(&mut s3).unwrap();
    let r1: [u8; 32] = s1.finalize().into();
    let r2: [u8; 32] = s2.finalize().into();
    let r3: [u8; 32] = s3.finalize().into();
    assert_eq!(r1, r2);
    assert_ne!(r1, r3);
}

#[test]
fn memory_store_insert_get() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "sprocket".into(), count: 3 };
    let h = w.hash(&mut store).unwrap();
    assert!(store.contains(&h));
    assert_eq!(store.get(&h).unwrap().name, "sprocket");
}

#[test]
fn memory_store_resolve_prefix() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "a".into(), count: 1 };
    let h = w.hash(&mut store).unwrap();

    let hex = h.to_string();
    let prefix = &hex[..4];
    let matches = store.resolve_prefix(prefix);
    assert!(matches.iter().any(|m| m.as_bytes() == h.as_bytes()));
}

#[test]
fn typed_hash_parse_full_hex() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "x".into(), count: 1 };
    let h = w.hash(&mut store).unwrap();
    let hex = h.to_string();

    let parsed = WidgetHash::parse::<Widget>(&hex, &store).unwrap();
    assert_eq!(parsed, h);
}

#[test]
fn typed_hash_parse_prefix() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "y".into(), count: 2 };
    let h = w.hash(&mut store).unwrap();
    let hex = h.to_string();

    let parsed = WidgetHash::parse::<Widget>(&hex[..6], &store).unwrap();
    assert_eq!(parsed, h);
}

#[test]
fn typed_hash_parse_not_found() {
    let store = MemoryHashStore::<Widget>::new();
    let err = WidgetHash::parse::<Widget>("aabbccdd", &store);
    assert!(matches!(err, Err(HashError::NotFound(_))));
}

#[test]
fn typed_hash_parse_prefix_too_short() {
    let store = MemoryHashStore::<Widget>::new();
    let err = WidgetHash::parse::<Widget>("a", &store);
    assert!(matches!(err, Err(HashError::PrefixTooShort { .. })));
}

#[test]
fn typed_hash_parse_invalid_hex() {
    let store = MemoryHashStore::<Widget>::new();
    let err = WidgetHash::parse::<Widget>("zzzz", &store);
    assert!(matches!(err, Err(HashError::InvalidHex(_))));
}

#[test]
fn typed_hash_from_bytes_validates() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "z".into(), count: 9 };
    let h = w.hash(&mut store).unwrap();

    let ok = WidgetHash::from_bytes::<Widget>(*h.as_bytes(), &store);
    assert!(ok.is_ok());

    let missing = WidgetHash::from_bytes::<Widget>([0xff; 32], &store);
    assert!(matches!(missing, Err(HashError::NotFound(_))));
}

#[test]
fn unvalidated_hash_roundtrip() {
    let mut store = MemoryHashStore::new();
    let w = Widget { name: "q".into(), count: 7 };
    let h = w.hash(&mut store).unwrap();
    let hex = h.to_string();

    let json = format!("\"{hex}\"");
    let unvalidated: UnvalidatedHash = serde_json::from_str(&json).unwrap();
    let validated = unvalidated.validate::<Widget>(&store).unwrap();
    assert_eq!(validated, h);
}

#[test]
fn chain_store_searches_all() {
    let mut s1 = MemoryHashStore::new();
    let mut s2 = MemoryHashStore::new();

    let w1 = Widget { name: "alpha".into(), count: 1 };
    let w2 = Widget { name: "beta".into(), count: 2 };
    let h1 = w1.hash(&mut s1).unwrap();
    let h2 = w2.hash(&mut s2).unwrap();

    let chain = ChainStore::new(vec![&mut s1, &mut s2]);
    assert!(chain.contains(&h1));
    assert!(chain.contains(&h2));
    assert_eq!(chain.get(&h1).unwrap().name, "alpha");
    assert_eq!(chain.get(&h2).unwrap().name, "beta");
}

#[test]
fn chain_store_inserts_into_first() {
    let mut s1 = MemoryHashStore::new();
    let mut s2 = MemoryHashStore::new();

    let w = Widget { name: "gamma".into(), count: 3 };
    let h = w.hash(&mut MemoryHashStore::new()).unwrap();

    {
        let mut chain = ChainStore::new(vec![&mut s1, &mut s2]);
        chain.insert(w);
    }

    assert!(s1.contains(&h));
    assert!(!s2.contains(&h));
}

#[test]
fn chain_store_dedupes_prefix() {
    let mut s1 = MemoryHashStore::new();
    let mut s2 = MemoryHashStore::new();

    let w = Widget { name: "delta".into(), count: 4 };
    let h = w.hash(&mut MemoryHashStore::new()).unwrap();
    let hex = h.to_string();
    s1.insert(Widget { name: "delta".into(), count: 4 });
    s2.insert(Widget { name: "delta".into(), count: 4 });

    let chain = ChainStore::new(vec![&mut s1, &mut s2]);
    let matches = chain.resolve_prefix(&hex[..4]);
    assert_eq!(matches.len(), 1);
}

#[test]
fn path_hash_into_reads_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "hello world").unwrap();

    let mut s1 = Sha256::new();
    path.hash_into(&mut s1).unwrap();
    let r1: [u8; 32] = s1.finalize().into();

    let mut s2 = Sha256::new();
    path.hash_into(&mut s2).unwrap();
    let r2: [u8; 32] = s2.finalize().into();
    assert_eq!(r1, r2);

    std::fs::write(&path, "changed").unwrap();
    let mut s3 = Sha256::new();
    path.hash_into(&mut s3).unwrap();
    let r3: [u8; 32] = s3.finalize().into();
    assert_ne!(r1, r3);
}

#[test]
fn path_hash_into_error_on_missing() {
    let path = std::path::Path::new("/nonexistent/file.txt");
    let mut state = Sha256::new();
    assert!(path.hash_into(&mut state).is_err());
}

#[test]
fn option_none_vs_some_differ() {
    let none: Option<u32> = None;
    let some = Some(0u32);
    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    none.hash_into(&mut s1).unwrap();
    some.hash_into(&mut s2).unwrap();
    assert_ne!(
        <[u8; 32]>::from(s1.finalize()),
        <[u8; 32]>::from(s2.finalize()),
    );
}

#[test]
fn vec_order_matters() {
    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    vec![1u32, 2].hash_into(&mut s1).unwrap();
    vec![2u32, 1].hash_into(&mut s2).unwrap();
    assert_ne!(
        <[u8; 32]>::from(s1.finalize()),
        <[u8; 32]>::from(s2.finalize()),
    );
}

#[test]
fn string_length_prefix_prevents_collisions() {
    #[derive(HashInto)]
    struct Pair { a: String, b: String }

    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    Pair { a: "ab".into(), b: "c".into() }.hash_into(&mut s1).unwrap();
    Pair { a: "a".into(), b: "bc".into() }.hash_into(&mut s2).unwrap();
    assert_ne!(
        <[u8; 32]>::from(s1.finalize()),
        <[u8; 32]>::from(s2.finalize()),
    );
}

#[test]
fn hash_std_trait_impls() {
    use std::collections::HashSet;
    let h1 = TestHash::from_raw([1; 32]);
    let h2 = TestHash::from_raw([2; 32]);
    let mut set = HashSet::new();
    set.insert(h1.clone());
    set.insert(h2.clone());
    assert_eq!(set.len(), 2);
    assert!(set.contains(&h1));
}

#[cfg(feature = "disk")]
mod disk_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Serialize, Deserialize, TypedHashable)]
    pub struct DiskItem {
        pub val: u32,
    }

    #[test]
    fn disk_store_insert_and_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = DiskHashStore::<DiskItem>::new(dir.path().to_path_buf());

        let item = DiskItem { val: 42 };
        let h = item.hash(&mut store).unwrap();

        let hex = h.to_string();
        let matches = store.resolve_prefix(&hex[..4]);
        assert!(!matches.is_empty());
    }

    #[test]
    fn disk_store_get_cached() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = DiskHashStore::<DiskItem>::new(dir.path().to_path_buf());

        let item = DiskItem { val: 99 };
        let h = item.hash(&mut store).unwrap();

        assert_eq!(store.get(&h).unwrap().val, 99);
    }

    #[test]
    fn disk_store_reloads_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let h = {
            let mut store = DiskHashStore::<DiskItem>::new(dir.path().to_path_buf());
            let item = DiskItem { val: 77 };
            let h = item.hash(&mut store).unwrap();
            h
        };

        let store = DiskHashStore::<DiskItem>::new(dir.path().to_path_buf());
        assert!(store.contains(&h));
        assert_eq!(store.get(&h).unwrap().val, 77);
    }
}
