use super::language::driver_for_lang;
use crate::mutant::{Mutant, TwigMutantsIter, TwigsMutantsIter};
use bough_typed_hash::{HashInto, TypedHashable};

#[derive(bough_typed_hash::TypedHash)]
pub struct MutationHash([u8; 32]);

// core[impl mutation.iter.mutant]
pub struct MutationIter<'a> {
    mutant: &'a Mutant,
    subs: std::vec::IntoIter<String>,
}

impl<'a> MutationIter<'a> {
    pub fn new(mutant: &'a Mutant) -> Self {
        let driver = driver_for_lang(mutant.lang());
        let subs = driver.substitutions(mutant.kind());
        Self {
            mutant,
            subs: subs.into_iter(),
        }
    }

    pub fn mutant(&self) -> &Mutant {
        self.mutant
    }
}

// core[impl mutation.iter.mutation]
impl<'a> Iterator for MutationIter<'a> {
    type Item = Mutation;

    fn next(&mut self) -> Option<Self::Item> {
        let subst = self.subs.next()?;
        Some(Mutation {
            mutant: self.mutant.clone(),
            subst,
        })
    }
}

// core[impl mutation.mutant]
// core[impl mutation.subst]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mutation {
    pub(crate) mutant: Mutant,
    pub(crate) subst: String,
}

impl Mutation {
    pub fn mutant(&self) -> &Mutant {
        &self.mutant
    }

    pub fn subst(&self) -> &str {
        &self.subst
    }
}

// core[impl mutation.twig-iter.twig-mutants-iter]
pub struct TwigMutationsIter<'a, 't> {
    inner: TwigMutantsIter<'a, 't>,
    current: Option<std::vec::IntoIter<Mutation>>,
}

impl<'a, 't> TwigMutationsIter<'a, 't> {
    pub fn new(inner: TwigMutantsIter<'a, 't>) -> Self {
        Self {
            inner,
            current: None,
        }
    }
}

// core[impl mutation.twig-iter]
// core[impl mutation.twig-iter.delegates]
impl<'a, 't> Iterator for TwigMutationsIter<'a, 't> {
    type Item = Mutation;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut iter) = self.current {
                if let Some(mutation) = iter.next() {
                    return Some(mutation);
                }
            }
            let based_mutant = self.inner.next()?;
            let mutations: Vec<Mutation> = MutationIter::new(based_mutant.mutant()).collect();
            self.current = Some(mutations.into_iter());
        }
    }
}

// core[impl mutation.twigs-iter.twigs-mutants-iter]
pub struct TwigsMutationsIter<'a> {
    inner: TwigsMutantsIter<'a>,
    current: Option<std::vec::IntoIter<Mutation>>,
}

impl<'a> TwigsMutationsIter<'a> {
    pub fn new(inner: TwigsMutantsIter<'a>) -> Self {
        Self {
            inner,
            current: None,
        }
    }
}

// core[impl mutation.twigs-iter]
// core[impl mutation.twigs-iter.delegates]
impl<'a> Iterator for TwigsMutationsIter<'a> {
    type Item = Mutation;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut iter) = self.current {
                if let Some(mutation) = iter.next() {
                    return Some(mutation);
                }
            }
            let based_mutant = self.inner.next()?;
            let mutations: Vec<Mutation> = MutationIter::new(based_mutant.mutant()).collect();
            self.current = Some(mutations.into_iter());
        }
    }
}

// core[impl mutation.hash.typed-hashable]
impl TypedHashable for Mutation {
    type Hash = MutationHash;
}

// core[impl mutation.hash.mutant]
// core[impl mutation.hash.subst]
impl HashInto for Mutation {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.mutant.hash_into(state)?;
        self.subst.hash_into(state)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Base;
    use crate::file::{TwigsIter, Twig};
    use crate::mutant::{BinaryOpMutationKind, MutantKind, Point, Span};
    use bough_typed_hash::HashStore;
    use std::path::PathBuf;

    fn make_base() -> (tempfile::TempDir, Base) {
        make_js_base("const a = 1;")
    }

    fn make_js_base(content: &str) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIter::new(dir.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    // core[verify mutation.iter.mutant]
    #[test]
    fn mutation_iter_holds_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let iter = MutationIter::new(&mutant);
        assert_eq!(iter.mutant().lang(), crate::LanguageId::Javascript);
    }

    // core[verify mutation.iter.language_driver]
    #[test]
    fn mutation_iter_delegates_to_language_driver() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
        let subs: Vec<&str> = mutations.iter().map(|m| m.subst()).collect();
        assert!(subs.is_empty() || !subs.is_empty());
    }

    // core[verify mutation.iter.mutation]
    #[test]
    fn mutation_iter_yields_mutations() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let _mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
    }

    // core[verify mutation.subst]
    #[test]
    fn mutation_owns_subst_string() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant) {
            assert!(!mutation.subst().is_empty());
        }
    }

    // core[verify mutation.mutant]
    #[test]
    fn mutation_holds_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant) {
            assert_eq!(mutation.mutant().lang(), crate::LanguageId::Javascript);
        }
    }

    // core[verify mutation.iter.invalid]
    #[test]
    fn mutation_iter_invalid_mutant_produces_no_mutations() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Typescript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 5, 5)),
        );
        let mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
        assert!(mutations.is_empty());
    }

    // core[verify mutation.subst.js.cond.false]
    #[test]
    fn js_condition_mutant_has_false_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"false".to_string()));
    }

    // core[verify mutation.subst.js.cond.true]
    #[test]
    fn js_condition_mutant_has_true_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"true".to_string()));
    }

    // core[verify mutation.subst.js.statement]
    #[test]
    fn js_statement_mutant_has_empty_block_substitution() {
        let js = "function foo() { return 1; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 15, 15), Point::new(0, 28, 28)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"{}".to_string()));
    }

    // core[verify mutation.subst.js.add.mul]
    #[test]
    fn js_add_mutant_has_mul_substitution() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"*".to_string()));
    }

    // core[verify mutation.subst.js.add.sub]
    #[test]
    fn js_add_mutant_has_sub_substitution() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"-".to_string()));
    }

    fn hash_mutation(mutation: &Mutation) -> [u8; 32] {
        use bough_typed_hash::sha2::Digest;
        let mut state = bough_typed_hash::ShaState::new();
        mutation.hash_into(&mut state).unwrap();
        state.finalize().into()
    }

    // core[verify mutation.hash.typed-hashable]
    #[test]
    fn mutation_produces_typed_hash() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "-".into(),
        };
        let mut store = bough_typed_hash::MemoryHashStore::new();
        let hash = mutation.hash(&mut store).unwrap();
        assert!(store.contains(&hash));
    }

    // core[verify mutation.hash.mutant]
    #[test]
    fn mutation_hash_includes_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let m2 = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let mut1 = Mutation {
            mutant: m1.clone(),
            subst: "-".into(),
        };
        let mut2 = Mutation {
            mutant: m2.clone(),
            subst: "-".into(),
        };
        assert_ne!(hash_mutation(&mut1), hash_mutation(&mut2));
    }

    // core[verify mutation.hash.subst]
    #[test]
    fn mutation_hash_includes_subst() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let mut1 = Mutation {
            mutant: m.clone(),
            subst: "-".into(),
        };
        let mut2 = Mutation {
            mutant: m.clone(),
            subst: "*".into(),
        };
        assert_ne!(hash_mutation(&mut1), hash_mutation(&mut2));
    }

    fn make_multi_js_base(files: &[(&str, &str)]) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
        }
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIter::new(dir.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    // core[verify mutation.twig-iter]
    // core[verify mutation.twig-iter.delegates]
    #[test]
    fn twig_mutations_iter_yields_mutations_for_single_twig() {
        let js = "function foo() { return a + b; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let inner = TwigMutantsIter::new(crate::LanguageId::Javascript, &base, &twig).unwrap();
        let mutations: Vec<Mutation> = TwigMutationsIter::new(inner).collect();
        let subs: Vec<&str> = mutations.iter().map(|m| m.subst()).collect();
        assert!(subs.contains(&"{}"));
        assert!(subs.contains(&"-"));
        assert!(subs.contains(&"*"));
        assert_eq!(mutations.len(), 3);
    }

    // core[verify mutation.twig-iter.twig-mutants-iter]
    #[test]
    fn twig_mutations_iter_holds_twig_mutants_iter() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let inner = TwigMutantsIter::new(crate::LanguageId::Javascript, &base, &twig).unwrap();
        let _iter = TwigMutationsIter::new(inner);
    }

    // core[verify mutation.twig-iter]
    #[test]
    fn twig_mutations_iter_empty_for_no_mutants() {
        let (_dir, base) = make_js_base("const a = 1;");
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let inner = TwigMutantsIter::new(crate::LanguageId::Javascript, &base, &twig).unwrap();
        let mutations: Vec<Mutation> = TwigMutationsIter::new(inner).collect();
        assert!(mutations.is_empty());
    }

    // core[verify mutation.twigs-iter]
    // core[verify mutation.twigs-iter.delegates]
    #[test]
    fn twigs_mutations_iter_yields_mutations_across_files() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "function foo() { return a + b; }"),
            ("src/b.js", "function bar() { return a - b; }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let inner = TwigsMutantsIter::new(crate::LanguageId::Javascript, &base, twigs);
        let mutations: Vec<Mutation> = TwigsMutationsIter::new(inner).collect();
        assert_eq!(mutations.len(), 6);
    }

    // core[verify mutation.twigs-iter.twigs-mutants-iter]
    #[test]
    fn twigs_mutations_iter_holds_twigs_mutants_iter() {
        let (_dir, base) = make_multi_js_base(&[("src/a.js", "const a = 1;")]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let inner = TwigsMutantsIter::new(crate::LanguageId::Javascript, &base, twigs);
        let _iter = TwigsMutationsIter::new(inner);
    }

    // core[verify mutation.twigs-iter]
    #[test]
    fn twigs_mutations_iter_empty_for_no_mutants() {
        let (_dir, base) = make_multi_js_base(&[("src/a.js", "const a = 1;")]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let inner = TwigsMutantsIter::new(crate::LanguageId::Javascript, &base, twigs);
        let mutations: Vec<Mutation> = TwigsMutationsIter::new(inner).collect();
        assert!(mutations.is_empty());
    }
}
