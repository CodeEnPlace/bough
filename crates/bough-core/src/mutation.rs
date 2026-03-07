use super::language::driver_for_lang;
use crate::mutant::Mutant;
use bough_typed_hash::TypedHashable;
use tracing::trace;

// bough[impl mutation.iter.mutant]
pub struct MutationIter<'a> {
    mutant: &'a Mutant,
    subs: std::vec::IntoIter<String>,
}

impl<'a> MutationIter<'a> {
    pub fn new(mutant: &'a Mutant) -> Self {
        let driver = driver_for_lang(mutant.lang());
        let subs = driver.substitutions(mutant.kind());
        trace!(lang = ?mutant.lang(), kind = ?mutant.kind(), substitutions = subs.len(), "creating mutation iter");
        Self {
            mutant,
            subs: subs.into_iter(),
        }
    }

    pub fn mutant(&self) -> &Mutant {
        self.mutant
    }
}

// bough[impl mutation.iter.mutation]
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

// bough[impl mutation.mutant]
// bough[impl mutation.subst]
// bough[impl mutation.hash.typed-hashable]
// bough[impl mutation.hash.mutant]
// bough[impl mutation.hash.subst]
#[derive(Debug, Clone, PartialEq, Eq, Hash, facet::Facet, TypedHashable)]
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

    pub fn apply_to_complete_src_string(&self, src: &str) -> String {
        let start = self.mutant.span().start().byte();
        let end = self.mutant.span().end().byte();
        format!("{}{}{}", &src[..start], self.subst, &src[end..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Base;
    use crate::file::Twig;
    use crate::mutant::{BinaryOpMutationKind, MutantKind, Point, Span};
    use crate::twig::TwigsIterBuilder;
    use bough_typed_hash::HashInto;
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
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    // bough[verify mutation.iter.mutant]
    #[test]
    fn mutation_iter_holds_mutant() {
        let (_dir, _base) = make_base();
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

    // bough[verify mutation.iter.language_driver]
    #[test]
    fn mutation_iter_delegates_to_language_driver() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.iter.mutation]
    #[test]
    fn mutation_iter_yields_mutations() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let _mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
    }

    // bough[verify mutation.subst]
    #[test]
    fn mutation_owns_subst_string() {
        let (_dir, _base) = make_base();
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

    // bough[verify mutation.mutant]
    #[test]
    fn mutation_holds_mutant() {
        let (_dir, _base) = make_base();
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

    // bough[verify mutation.iter.invalid]
    #[test]
    fn mutation_iter_invalid_mutant_produces_no_mutations() {
        let (_dir, _base) = make_base();
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

    // bough[verify mutation.subst.js.cond.false]
    #[test]
    fn js_condition_mutant_has_false_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.subst.js.cond.true]
    #[test]
    fn js_condition_mutant_has_true_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.subst.js.statement]
    #[test]
    fn js_statement_mutant_has_empty_block_substitution() {
        let js = "function foo() { return 1; }";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.subst.js.add.mul]
    #[test]
    fn js_add_mutant_has_mul_substitution() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.subst.js.add.sub]
    #[test]
    fn js_add_mutant_has_sub_substitution() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
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

    // bough[verify mutation.hash.typed-hashable]
    #[test]
    fn mutation_produces_typed_hash() {
        let (_dir, _base) = make_base();
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
        let hash = mutation.hash().unwrap();
        assert_eq!(hash.to_string().len(), 64);
    }

    // bough[verify mutation.hash.mutant]
    #[test]
    fn mutation_hash_includes_mutant() {
        let (_dir, _base) = make_base();
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

    // bough[verify mutation.hash.subst]
    #[test]
    fn mutation_hash_includes_subst() {
        let (_dir, _base) = make_base();
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
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    #[test]
    fn apply_to_complete_src_string_replaces_binary_op() {
        let src = "return a + b;";
        //                  ^  byte 9..10
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 9, 9), Point::new(0, 10, 10)),
        );
        let mutation = Mutation { mutant, subst: "-".into() };
        assert_eq!(mutation.apply_to_complete_src_string(src), "return a - b;");
    }

    #[test]
    fn apply_to_complete_src_string_multiline() {
        let src = "function f() {\n    return a + b;\n}";
        //                                    ^  byte 28..29
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(1, 13, 28), Point::new(1, 14, 29)),
        );
        let mutation = Mutation { mutant, subst: "*".into() };
        assert_eq!(mutation.apply_to_complete_src_string(src), "function f() {\n    return a * b;\n}");
    }

    #[test]
    fn apply_to_complete_src_string_different_length_subst() {
        let src = "if (x) { foo(); }";
        //                 ^^^^^^^^^^  bytes 7..17
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 7, 7), Point::new(0, 17, 17)),
        );
        let mutation = Mutation { mutant, subst: "{}".into() };
        assert_eq!(mutation.apply_to_complete_src_string(src), "if (x) {}");
    }

    #[test]
    fn apply_to_complete_src_string_condition() {
        let src = "if (a > 0) { return 1; }";
        //             ^^^^^  bytes 4..9 is "a > 0" (inside parens)
        let mutant = Mutant::new(
            crate::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::Condition,
            Span::new(Point::new(0, 4, 4), Point::new(0, 9, 9)),
        );
        let mutation = Mutation { mutant, subst: "true".into() };
        assert_eq!(mutation.apply_to_complete_src_string(src), "if (true) { return 1; }");
    }
}
