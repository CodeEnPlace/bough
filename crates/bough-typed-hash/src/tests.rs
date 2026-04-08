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
    let h2 = h1;
    assert_eq!(h1, h2);
}

#[test]
fn typed_hash_display_format() {
    let h = TestHash::from_raw([0xab; 32]);
    let displayed = h.to_string();
    assert_eq!(displayed.len(), 64);
    assert!(displayed.chars().all(|c| c == 'a' || c == 'b'));
}

#[test]
fn typed_hashable_produces_hash() {
    let w = Widget {
        name: "gear".into(),
        count: 5,
    };
    let h = w.hash().unwrap();
    assert_eq!(h.to_string().len(), 64);
}

#[test]
fn typed_hashable_deterministic() {
    let a = Widget {
        name: "gear".into(),
        count: 5,
    };
    let b = Widget {
        name: "gear".into(),
        count: 5,
    };
    assert_eq!(a.hash().unwrap(), b.hash().unwrap());
}

#[test]
fn typed_hashable_different_values_differ() {
    let a = Widget {
        name: "gear".into(),
        count: 5,
    };
    let b = Widget {
        name: "gear".into(),
        count: 6,
    };
    assert_ne!(a.hash().unwrap(), b.hash().unwrap());
}

#[test]
fn hash_into_derive_nested() {
    let a = Assembly {
        part: Part {
            label: "bolt".into(),
            weight: 1.5,
        },
        quantity: 10,
    };
    let b = Assembly {
        part: Part {
            label: "bolt".into(),
            weight: 1.5,
        },
        quantity: 10,
    };
    let c = Assembly {
        part: Part {
            label: "nut".into(),
            weight: 0.5,
        },
        quantity: 10,
    };
    assert_eq!(a.hash().unwrap(), b.hash().unwrap());
    assert_ne!(a.hash().unwrap(), c.hash().unwrap());
}

#[test]
fn hash_into_derive_enum() {
    #[derive(HashInto)]
    enum Shape {
        Circle(f64),
        Rect {
            w: f64,
            h: f64,
        },
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
fn unvalidated_hash_full_hex() {
    let w = Widget {
        name: "q".into(),
        count: 7,
    };
    let h = w.hash().unwrap();
    let hex = h.to_string();

    let unvalidated = UnvalidatedHash::new(hex);
    let validated = unvalidated.validate(&[h]).unwrap();
    assert_eq!(validated, h);
}

#[test]
fn unvalidated_hash_prefix() {
    let w = Widget {
        name: "y".into(),
        count: 2,
    };
    let h = w.hash().unwrap();
    let hex = h.to_string();

    let unvalidated = UnvalidatedHash::new(hex[..6].to_string());
    let validated = unvalidated.validate(&[h]).unwrap();
    assert_eq!(validated, h);
}

#[test]
fn unvalidated_hash_not_found() {
    let known: Vec<WidgetHash> = vec![];
    let err = UnvalidatedHash::new("aabbccdd".into()).validate(&known);
    assert!(matches!(err, Err(HashError::NotFound(_))));
}

#[test]
fn unvalidated_hash_invalid_hex() {
    let known: Vec<WidgetHash> = vec![];
    let err = UnvalidatedHash::new("zzzz".into()).validate(&known);
    assert!(matches!(err, Err(HashError::InvalidHex(_))));
}

#[test]
fn unvalidated_hash_ambiguous() {
    let h1 = WidgetHash::from_raw([0xaa; 32]);
    let mut h2_bytes = [0xaa; 32];
    h2_bytes[31] = 0xbb;
    let h2 = WidgetHash::from_raw(h2_bytes);

    let err = UnvalidatedHash::new("aa".into()).validate(&[h1, h2]);
    assert!(matches!(err, Err(HashError::Ambiguous { .. })));
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
    struct Pair {
        a: String,
        b: String,
    }

    let mut s1 = Sha256::new();
    let mut s2 = Sha256::new();
    Pair {
        a: "ab".into(),
        b: "c".into(),
    }
    .hash_into(&mut s1)
    .unwrap();
    Pair {
        a: "a".into(),
        b: "bc".into(),
    }
    .hash_into(&mut s2)
    .unwrap();
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
    set.insert(h1);
    set.insert(h2);
    assert_eq!(set.len(), 2);
    assert!(set.contains(&h1));
}

#[test]
fn bytes_to_hex_roundtrip() {
    let bytes = [0xde; 32];
    let hex = bytes_to_hex(&bytes);
    assert_eq!(hex.len(), 64);
    let back = hex_to_bytes(&hex).unwrap();
    assert_eq!(back, bytes);
}
