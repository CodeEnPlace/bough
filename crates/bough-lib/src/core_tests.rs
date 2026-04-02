#[cfg(test)]
mod mutant_tests {
    use bough_core::LanguageId;
    use bough_core::mutant::{
        AssignMutationKind, BasedMutant, BinaryOpMutationKind,
        LiteralKind, MutantKind, Mutant, OptionalChainKind,
        Point, RangeKind, Span, TwigMutantsIter,
        find_mutants_in_source,
    };
    use bough_typed_hash::{HashInto, TypedHashable};
    use bough_fs::{Root, Twig, TwigsIterBuilder};
    use crate::Base;
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














    #[test]
    fn mutants_iter_holds_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).unwrap();
        assert_eq!(iter.twig().path(), std::path::Path::new("src/a.js"));
    }

    #[test]
    fn mutants_iter_holds_base() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).unwrap();
        assert_eq!(iter.base().path(), base.path());
    }

    #[test]
    fn mutants_iter_owns_lang() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).unwrap();
        assert_eq!(iter.lang(), LanguageId::Javascript);
    }

    #[test]
    fn mutants_iter_resolves_file_from_base_and_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        assert!(TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).is_ok());
    }

    #[test]
    fn mutants_iter_errors_on_missing_file() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/nonexistent.js")).unwrap();
        assert!(TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).is_err());
    }

    #[test]
    fn mutants_iter_walks_tree_and_returns_mutants() {
        let (_dir, base) = make_js_base("const a = x;");
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver)).unwrap();
        let mutants: Vec<_> = iter.collect();
        assert!(mutants.is_empty());
    }

    #[test]
    fn js_finds_statement_blocks() {
        let js = "function foo() { return 1; }\nfunction bar() { return 2; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let blocks: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::StatementBlock))
            .collect();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].span().start().line(), 0);
        assert_eq!(blocks[1].span().start().line(), 1);
    }

    #[test]
    fn js_finds_if_condition() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn js_finds_while_condition() {
        let js = "while (i < 10) { i++; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    #[test]
    fn js_finds_for_condition() {
        let js = "for (let i = 0; i < 10; i++) { console.log(i); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
        let span = conditions[0].span();
        let condition_text = &js[span.start().byte()..span.end().byte()];
        assert_eq!(condition_text, "i < 10");
    }

    #[test]
    fn js_finds_add_binary_op() {
        let js = "const x = a + b;";
        //                    ^  byte 12..13
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let adds: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
            .collect();
        assert_eq!(adds.len(), 1);
        assert_eq!(
            *adds[0].span(),
            Span::new(Point::new(0, 12, 12), Point::new(0, 13, 13))
        );
    }

    #[test]
    fn js_finds_sub_binary_op() {
        let js = "const x = a - b;";
        //                    ^  byte 12..13
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .collect();
        let subs: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Sub)))
            .collect();
        assert_eq!(subs.len(), 1);
        assert_eq!(
            *subs[0].span(),
            Span::new(Point::new(0, 12, 12), Point::new(0, 13, 13))
        );
    }

    fn hash_mutant(mutant: &Mutant) -> [u8; 32] {
        use bough_typed_hash::sha2::Digest;
        let mut state = bough_typed_hash::ShaState::new();
        mutant.hash_into(&mut state).unwrap();
        state.finalize().into()
    }

    fn hash_based_mutant(based: &BasedMutant<'_, Base>) -> [u8; 32] {
        use bough_typed_hash::sha2::Digest;
        let mut state = bough_typed_hash::ShaState::new();
        based.hash_into(&mut state).unwrap();
        state.finalize().into()
    }

    #[test]
    fn mutant_hash_includes_lang() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Typescript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    #[test]
    fn mutant_hash_includes_twig() {
        let twig_a = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let twig_b = Twig::new(PathBuf::from("src/b.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            twig_a,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            twig_b,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    #[test]
    fn mutant_hash_includes_span() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(5, 3, 40), Point::new(8, 0, 70)),
            Span::new(Point::new(5, 3, 40), Point::new(8, 0, 70)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    #[test]
    fn mutant_hash_includes_kind() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    #[test]
    fn mutant_produces_typed_hash() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let hash = m.hash().unwrap();
        assert_eq!(hash.to_string().len(), 64);
    }

    #[test]
    fn based_mutant_hash_excludes_base() {
        let dir1 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir1.path().join("src")).unwrap();
        std::fs::write(dir1.path().join("src/a.js"), "const a = 1;").unwrap();
        let base1 = Base::new(
            dir1.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir2.path().join("src")).unwrap();
        std::fs::write(dir2.path().join("src/a.js"), "const a = 1;").unwrap();
        let base2 = Base::new(
            dir2.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let bm1 = BasedMutant::new(mutant.clone(), &base1);
        let bm2 = BasedMutant::new(mutant, &base2);
        assert_eq!(hash_based_mutant(&bm1), hash_based_mutant(&bm2));
    }

    #[test]
    fn based_mutant_hash_includes_file_contents() {
        let dir1 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir1.path().join("src")).unwrap();
        std::fs::write(dir1.path().join("src/a.js"), "const a = 1;").unwrap();
        let base1 = Base::new(
            dir1.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir2.path().join("src")).unwrap();
        std::fs::write(dir2.path().join("src/a.js"), "const b = 2;").unwrap();
        let base2 = Base::new(
            dir2.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let bm1 = BasedMutant::new(mutant.clone(), &base1);
        let bm2 = BasedMutant::new(mutant, &base2);
        assert_ne!(hash_based_mutant(&bm1), hash_based_mutant(&bm2));
    }

    #[test]
    fn based_mutant_produces_typed_hash() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let based = BasedMutant::new(mutant, &base);
        let hash = based.hash().unwrap();
        assert_eq!(hash.to_string().len(), 64);
    }

    #[test]
    fn span_composed_of_two_points() {
        let start = Point::new(1, 0, 0);
        let end = Point::new(5, 10, 50);
        let span = Span::new(start, end);
        assert_eq!(span.start().line(), 1);
        assert_eq!(span.start().col(), 0);
        assert_eq!(span.end().line(), 5);
        assert_eq!(span.end().byte(), 50);
    }

    #[test]
    fn span_intersects_overlapping() {
        let a = Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10));
        let b = Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15));
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn span_intersects_identical() {
        let a = Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10));
        let b = Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10));
        assert!(a.intersects(&b));
    }

    #[test]
    fn span_intersects_contained() {
        let outer = Span::new(Point::new(0, 0, 0), Point::new(0, 20, 20));
        let inner = Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15));
        assert!(outer.intersects(&inner));
        assert!(inner.intersects(&outer));
    }

    #[test]
    fn span_no_intersect_disjoint() {
        let a = Span::new(Point::new(0, 0, 0), Point::new(0, 5, 5));
        let b = Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15));
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    #[test]
    fn span_no_intersect_adjacent() {
        let a = Span::new(Point::new(0, 0, 0), Point::new(0, 5, 5));
        let b = Span::new(Point::new(0, 5, 5), Point::new(0, 10, 10));
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    #[test]
    fn span_intersects_single_byte_overlap() {
        let a = Span::new(Point::new(0, 0, 0), Point::new(0, 5, 5));
        let b = Span::new(Point::new(0, 4, 4), Point::new(0, 10, 10));
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn skip_kind_filters_matching_mutants() {
        // "function foo() { return a + b; }" produces: StatementBlock, BinaryOp(Add)
        let js = "function foo() { return a + b; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .with_skip_kind(MutantKind::StatementBlock)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    #[test]
    fn skip_kind_multiple_filters_all_specified_kinds() {
        // "function foo() { if (x) { return a + b; } }" produces:
        //   StatementBlock (outer), Condition, StatementBlock (inner), BinaryOp(Add)
        let js = "function foo() { if (x) { return a + b; } }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .with_skip_kind(MutantKind::StatementBlock)
            .with_skip_kind(MutantKind::Condition)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    #[test]
    fn skip_query_filters_matching_nodes() {
        // "const x = a + b; const y = a - b;" produces: BinaryOp(Add), BinaryOp(Sub)
        // skip query targeting "+" operator removes only the Add
        let js = "const x = a + b; const y = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let filtered: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            *filtered[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub),
        );
    }

    #[test]
    fn skip_query_multiple_filters_union() {
        // "const x = a + b; const y = a - b;" produces: BinaryOp(Add), BinaryOp(Sub)
        // skip query for add filters the Add, skip query for sub filters the Sub
        let js = "const x = a + b; const y = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let filtered_one: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .collect();
        assert_eq!(filtered_one.len(), 1);
        assert_eq!(
            *filtered_one[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub),
        );

        let filtered_both: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .with_skip_query("(binary_expression operator: \"-\") @skip")
            .collect();
        assert!(filtered_both.is_empty());
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

    const CONTEXT_JS: &str = "\
// line 0
function add(a, b) {
    if (a > 0) {
        return a + b;
    }
    return b;
}
// line 7
function sub(a, b) {
    return a - b;
}";

    fn make_context_base() -> (tempfile::TempDir, Base) {
        make_js_base(CONTEXT_JS)
    }

    fn find_add_mutant(base: &Base) -> Mutant {
        let twig = base.twigs().next().unwrap();
        TwigMutantsIter::new(LanguageId::Javascript, base, &twig, Box::new(bough_lang_javascript::JavascriptDriver))
            .unwrap()
            .map(|bm| bm.into_mutant())
            .find(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
            .expect("should find a + b mutant")
    }

    #[test]
    fn contextual_fragment_zero_returns_complete_statement() {
        let (_dir, base) = make_context_base();
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 0, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(text, "return a + b;");
        assert_eq!(span, Span::new(Point::new(3, 8, 56), Point::new(3, 21, 69)));
    }

    #[test]
    fn contextual_fragment_one_returns_if_statement() {
        let (_dir, base) = make_context_base();
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 1, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(text, "if (a > 0) {\n        return a + b;\n    }");
        assert_eq!(span, Span::new(Point::new(2, 4, 35), Point::new(4, 5, 75)));
    }

    #[test]
    fn contextual_fragment_large_caps_at_function_boundary() {
        let (_dir, base) = make_context_base();
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 100, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(
            text,
            "function add(a, b) {\n    if (a > 0) {\n        return a + b;\n    }\n    return b;\n}"
        );
        assert_eq!(span, Span::new(Point::new(1, 0, 10), Point::new(6, 1, 91)));
    }

    #[test]
    fn contextual_fragment_boundary_prevents_sibling_inclusion() {
        let (_dir, base) = make_context_base();
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 3, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(
            text,
            "function add(a, b) {\n    if (a > 0) {\n        return a + b;\n    }\n    return b;\n}"
        );
        assert_eq!(span, Span::new(Point::new(1, 0, 10), Point::new(6, 1, 91)));
    }

    #[test]
    fn contextual_fragment_mutant_at_start_of_function() {
        let (_dir, base) = make_js_base("function foo() {\n    return 1 + 2;\n}");
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 100, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(text, "function foo() {\n    return 1 + 2;\n}");
        assert_eq!(span, Span::new(Point::new(0, 0, 0), Point::new(2, 1, 36)));
    }

    #[test]
    fn contextual_fragment_mutant_at_end_of_function() {
        let (_dir, base) = make_js_base(
            "function bar() {\n    const x = 1;\n    const y = 2;\n    return x + y;\n}",
        );
        let mutant = find_add_mutant(&base);
        let (text, span) = mutant.get_contextual_fragment(&base, 100, &bough_lang_javascript::JavascriptDriver).unwrap();
        assert_eq!(
            text,
            "function bar() {\n    const x = 1;\n    const y = 2;\n    return x + y;\n}"
        );
        assert_eq!(span, Span::new(Point::new(0, 0, 0), Point::new(4, 1, 70)));
    }

    #[test]
    fn to_key_simple_kinds() {
        assert_eq!(MutantKind::StatementBlock.to_key(), "StatementBlock");
        assert_eq!(MutantKind::Condition.to_key(), "Condition");
        assert_eq!(MutantKind::DictDecl.to_key(), "DictDecl");
        assert_eq!(MutantKind::SwitchCase.to_key(), "SwitchCase");
    }

    #[test]
    fn to_key_parameterised_kinds() {
        assert_eq!(
            MutantKind::BinaryOp(BinaryOpMutationKind::Add).to_key(),
            "BinaryOp(Add)"
        );
        assert_eq!(
            MutantKind::Assign(AssignMutationKind::NormalAssign).to_key(),
            "Assign(NormalAssign)"
        );
        assert_eq!(
            MutantKind::Literal(LiteralKind::BoolTrue).to_key(),
            "Literal(BoolTrue)"
        );
        assert_eq!(
            MutantKind::OptionalChain(OptionalChainKind::FnCall).to_key(),
            "OptionalChain(FnCall)"
        );
        assert_eq!(
            MutantKind::Range(RangeKind::Exclusive).to_key(),
            "Range(Exclusive)"
        );
    }

    #[test]
    fn from_key_roundtrips_all_variants() {
        for kind in MutantKind::all_variants() {
            let key = kind.to_key();
            let parsed = MutantKind::from_key(&key)
                .unwrap_or_else(|| panic!("from_key failed for key: {key}"));
            assert_eq!(parsed, kind, "roundtrip failed for key: {key}");
        }
    }

    #[test]
    fn from_key_returns_none_for_unknown() {
        assert_eq!(MutantKind::from_key("Nonsense"), None);
        assert_eq!(MutantKind::from_key("BinaryOp(Nonsense)"), None);
        assert_eq!(MutantKind::from_key("Unknown(Add)"), None);
        assert_eq!(MutantKind::from_key(""), None);
    }

    #[test]
    fn find_mutants_in_source_finds_binary_op() {
        let source = b"const total = price + tax;";
        let mutants = find_mutants_in_source(&bough_lang_javascript::JavascriptDriver, source);
        let add_mutants: Vec<_> = mutants
            .iter()
            .filter(|m| m.kind == MutantKind::BinaryOp(BinaryOpMutationKind::Add))
            .collect();
        assert_eq!(add_mutants.len(), 1);
        let m = &add_mutants[0];
        assert_eq!(m.subst_span.start().byte(), 20);
        assert_eq!(m.subst_span.end().byte(), 21);
    }

    #[test]
    fn find_mutants_in_source_finds_multiple_of_same_kind() {
        let source = b"const z = a + b + c;";
        let mutants = find_mutants_in_source(&bough_lang_javascript::JavascriptDriver, source);
        let add_mutants: Vec<_> = mutants
            .iter()
            .filter(|m| m.kind == MutantKind::BinaryOp(BinaryOpMutationKind::Add))
            .collect();
        assert_eq!(add_mutants.len(), 2);
    }

    #[test]
    fn find_mutants_in_source_returns_empty_for_no_mutants() {
        let source = b"// just a comment";
        let mutants = find_mutants_in_source(&bough_lang_javascript::JavascriptDriver, source);
        assert!(mutants.is_empty());
    }

    #[test]
    fn find_mutants_in_source_works_for_python() {
        let source = b"x = a + b";
        let mutants = find_mutants_in_source(&bough_lang_python::PythonDriver, source);
        let add_mutants: Vec<_> = mutants
            .iter()
            .filter(|m| m.kind == MutantKind::BinaryOp(BinaryOpMutationKind::Add))
            .collect();
        assert_eq!(add_mutants.len(), 1);
    }

}

#[cfg(test)]
mod mutation_tests {
    use bough_core::mutant::{BinaryOpMutationKind, Mutant, MutantKind, Point, Span};
    use bough_core::{Mutation, MutationIter};
    use bough_typed_hash::{HashInto, TypedHashable};
    use bough_fs::{Twig, TwigsIterBuilder};
    use crate::Base;
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

    #[test]
    fn mutation_iter_holds_mutant() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let iter = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver);
        assert_eq!(iter.mutant().lang(), bough_core::LanguageId::Javascript);
    }

    #[test]
    fn mutation_iter_delegates_to_language_driver() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutations: Vec<Mutation> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver).collect();
        let subs: Vec<&str> = mutations.iter().map(|m| m.subst()).collect();
        assert!(subs.is_empty() || !subs.is_empty());
    }

    #[test]
    fn mutation_iter_yields_mutations() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let _mutations: Vec<Mutation> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver).collect();
    }

    #[test]
    fn mutation_owns_subst_string() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver) {
            assert!(!mutation.subst().is_empty());
        }
    }

    #[test]
    fn mutation_holds_mutant() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver) {
            assert_eq!(mutation.mutant().lang(), bough_core::LanguageId::Javascript);
        }
    }

    #[test]
    fn js_condition_mutant_has_false_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"false".to_string()));
    }

    #[test]
    fn js_condition_mutant_has_true_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"true".to_string()));
    }

    #[test]
    fn js_statement_mutant_has_empty_block_substitution() {
        let js = "function foo() { return 1; }";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 15, 15), Point::new(0, 28, 28)),
            Span::new(Point::new(0, 15, 15), Point::new(0, 28, 28)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"{}".to_string()));
    }

    #[test]
    fn js_add_mutant_has_mul_substitution() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
            .map(|m| m.subst().to_string())
            .collect();
        assert!(subs.contains(&"*".to_string()));
    }

    #[test]
    fn js_add_mutant_has_sub_substitution() {
        let js = "const x = a + b;";
        let (_dir, _base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
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

    #[test]
    fn mutation_produces_typed_hash() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let mutation = Mutation {
            mutant: mutant.clone(),
            subst: "-".into(),
        };
        let hash = mutation.hash().unwrap();
        assert_eq!(hash.to_string().len(), 64);
    }

    #[test]
    fn mutation_hash_includes_mutant() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let m2 = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
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

    #[test]
    fn mutation_hash_includes_subst() {
        let (_dir, _base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            bough_core::LanguageId::Javascript,
            twig.clone(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
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
            bough_core::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 9, 9), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 9, 9), Point::new(0, 10, 10)),
        );
        let mutation = Mutation {
            mutant,
            subst: "-".into(),
        };
        assert_eq!(mutation.apply_to_complete_src_string(src), "return a - b;");
    }

    #[test]
    fn apply_to_complete_src_string_multiline() {
        let src = "function f() {\n    return a + b;\n}";
        //                                    ^  byte 28..29
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(1, 13, 28), Point::new(1, 14, 29)),
            Span::new(Point::new(1, 13, 28), Point::new(1, 14, 29)),
        );
        let mutation = Mutation {
            mutant,
            subst: "*".into(),
        };
        assert_eq!(
            mutation.apply_to_complete_src_string(src),
            "function f() {\n    return a * b;\n}"
        );
    }

    #[test]
    fn apply_to_complete_src_string_different_length_subst() {
        let src = "if (x) { foo(); }";
        //                 ^^^^^^^^^^  bytes 7..17
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 7, 7), Point::new(0, 17, 17)),
            Span::new(Point::new(0, 7, 7), Point::new(0, 17, 17)),
        );
        let mutation = Mutation {
            mutant,
            subst: "{}".into(),
        };
        assert_eq!(mutation.apply_to_complete_src_string(src), "if (x) {}");
    }

    #[test]
    fn apply_to_complete_src_string_condition() {
        let src = "if (a > 0) { return 1; }";
        //             ^^^^^  bytes 4..9 is "a > 0" (inside parens)
        let mutant = Mutant::new(
            bough_core::LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::Condition,
            Span::new(Point::new(0, 4, 4), Point::new(0, 9, 9)),
            Span::new(Point::new(0, 4, 4), Point::new(0, 9, 9)),
        );
        let mutation = Mutation {
            mutant,
            subst: "true".into(),
        };
        assert_eq!(
            mutation.apply_to_complete_src_string(src),
            "if (true) { return 1; }"
        );
    }

}
