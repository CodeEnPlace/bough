use bough_cli_tests::Fixture;

#[test]
fn noop_missing_config() {
    let fixture = Fixture::new().build();

    let result = fixture.run("noop");

    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, "");
    let stderr = result.redacted_stderr(&fixture);
    assert!(
        stderr.contains("Missing required fields"),
        "stderr should mention missing fields, got: {stderr}"
    );
    assert!(
        stderr.contains("<CONFIG_SEARCH_PATHS>"),
        "config search paths should be redacted, got: {stderr}"
    );
}

#[test]
fn noop_invalid_config() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []
"#,
        )
        .build();

    let result = fixture.run("noop");

    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, "");
    let stderr = result.redacted_stderr(&fixture);
    assert!(
        stderr.contains("test.cmd is required"),
        "stderr should mention missing test cmd, got: {stderr}"
    );
}

#[test]
fn noop_with_valid_config() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "if (x > 1) {}")
        .build();

    let result = fixture.run("noop");

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "\n");
    assert_eq!(result.stderr, "");
}

#[test]
fn show_mutations_multiple_files() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "if (x > 1) {}")
        .with_file("src/b.js", "if (y < 2) {}")
        .build();

    let result = fixture.run("show mutations");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "\
ef55a7704df7b1f2eda6d0f4c6094f633c7be7f0701d52441e20f59fe76e797c js src/a.js 1:5 - 1:10 not run Condition -> true
1eb4cc2c7d61b6edac1d731200e09db106078ec309675e0c26ae529defcf00bb js src/a.js 1:5 - 1:10 not run Condition -> false
1b903677268c6bfb2b0ce68d73c3afb713be309c959c26503fb4212dc59e79ea js src/a.js 1:7 - 1:8 not run BinaryOp(Gt) -> <=
53b9b358ff24fbeb7903eb52c3a12dfc162fc922fb34696fc211bc0f84a478d2 js src/a.js 1:7 - 1:8 not run BinaryOp(Gt) -> >=
719d9d6a24082ae6e114adbe1d942f88f05b570d47b97803712d590707f1b955 js src/a.js 1:9 - 1:10 not run Literal(Number) -> 0
ea287652cac5f3a566a8fb4c582421a8cd07cc69ff7fba122066c6b166297319 js src/a.js 1:9 - 1:10 not run Literal(Number) -> 1
368ca084999c641c241259a008c1db431c99c1ee612acf3ef68872729b0b20cb js src/a.js 1:9 - 1:10 not run Literal(Number) -> -1
c3cc1162a3ff5de12e88d90a1d5a3368b969daed14d81451061e9eddecccb9d4 js src/a.js 1:9 - 1:10 not run Literal(Number) -> Infinity
46b55de46dde476c7656278c4bff79a2eab355fd7323fb7bbd3a4cc4fd57df6d js src/a.js 1:9 - 1:10 not run Literal(Number) -> -Infinity
88796a09235eb87b07cb323b0de03f2f0f358b66ed90e52638755c8c1c117f84 js src/a.js 1:9 - 1:10 not run Literal(Number) -> NaN
901060bfab2e89e53f3a73d024e46566bc1f948712d24487e2564dcf0c2f9b5e js src/a.js 1:12 - 1:14 not run StatementBlock -> {}
bf8647631c7ea131a46aed9f7f7c347c119fc613bcb00d01b07c85aefe33dbd9 js src/b.js 1:5 - 1:10 not run Condition -> true
2b0b55b1659dbea64f0fbeefcb4e22b4a5b4aa021273f4dc14570ccaebb33d98 js src/b.js 1:5 - 1:10 not run Condition -> false
e98c951ddcc5a850903539fd3fbda4059f4d720cb663b4052bba80a3b965885d js src/b.js 1:7 - 1:8 not run BinaryOp(Lt) -> >=
a86ff3e95b8e29b83c696dfc687425dea3d21e5c24dae41b4331fe215ba1259d js src/b.js 1:7 - 1:8 not run BinaryOp(Lt) -> <=
40ddbed8c2c97f25b81e1797d690c219f11c5e2f205b474c69e7d0c2d4a81ac5 js src/b.js 1:9 - 1:10 not run Literal(Number) -> 0
fdf0513728c012ad166f26ef0ab32f6007305054f3a4b1eaec04ad50354b45dc js src/b.js 1:9 - 1:10 not run Literal(Number) -> 1
50a5b35e0fcd1bc080131f52aa5bba38036c106e955d7bb05106f27a5d3f333d js src/b.js 1:9 - 1:10 not run Literal(Number) -> -1
e57ca9300fdc761bc3b06a4ef91ba9d23320a5b8320eef8815f9d71ba510aa27 js src/b.js 1:9 - 1:10 not run Literal(Number) -> Infinity
b18af729b78d7c8c3c9b0c1e9d196c86e11916fdfec532b8a5c836041e18690b js src/b.js 1:9 - 1:10 not run Literal(Number) -> -Infinity
fbb1c239d3ecf4b31ad2aacdbb0b0d7c0eb6369b555eec4f41bdce9a97552fd2 js src/b.js 1:9 - 1:10 not run Literal(Number) -> NaN
36128e2faa4ebab7387520f98385d18d2ba9e4c5027612b5b28fff5b8a96e88f js src/b.js 1:12 - 1:14 not run StatementBlock -> {}
"
    );
}

#[test]
fn show_mutations_lang_filter() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[lang.ts]
include = ["**/*.ts"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "if (x > 1) {}")
        .with_file("src/b.ts", "if (y < 2) {}")
        .build();

    let result = fixture.run("show mutations ts");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "\
08fd1d84af296da3050e290dda9511df2b358f731476ead6331a01f43b507717 ts src/b.ts 1:5 - 1:10 not run Condition -> true
e0e73dc8df81bfe654548f16656125b4c4c42611fd9aa4a131fcc18d5c8c2300 ts src/b.ts 1:5 - 1:10 not run Condition -> false
d31a447490a0534e872027120b46e8ad9fbfc82813fc41175e8e5876c4483dab ts src/b.ts 1:7 - 1:8 not run BinaryOp(Lt) -> >=
c71f0e0a7ad8d7b7d9f06baf8d0e944634082ec41a2a5d490146691f84cf05b4 ts src/b.ts 1:7 - 1:8 not run BinaryOp(Lt) -> <=
26db377649aa16a89661330b5b02ff673049cfb3cff2641d6d6506569ea3d9a9 ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> 0
e3d53260b5fe2a1f91ca0dc40a5bcb0c394af69733265787f7670b25e34bf2cd ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> 1
4be3cb9abf884c2a782e847d8f20ae8c1ca690347677e35c4ddd22a5c14d77a7 ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> -1
080192681e39f403a3dfc3321a032196e0cf2e4ca26c87701ee7e8b42039a8a2 ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> Infinity
4442c95140f14a8d9e182e0fd6c9a4ab2cb9007a7c1059cfbfcd8c0ebb031cbe ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> -Infinity
072ed793778608e08ef290bcd293d6c589bc29ace77f7ef7241e7d3c5f060bce ts src/b.ts 1:9 - 1:10 not run Literal(Number) -> NaN
d07ac60ebe04ada1035d6e1b5881b75074deadbc9dd994ebd16f5b7cfafd35d9 ts src/b.ts 1:12 - 1:14 not run StatementBlock -> {}
"
    );
}

#[test]
fn show_mutations_file_filter() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "if (x > 1) {}")
        .with_file("src/b.js", "if (y < 2) {}")
        .build();

    let result = fixture.run("show mutations js src/a.js");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "\
ef55a7704df7b1f2eda6d0f4c6094f633c7be7f0701d52441e20f59fe76e797c js src/a.js 1:5 - 1:10 not run Condition -> true
1eb4cc2c7d61b6edac1d731200e09db106078ec309675e0c26ae529defcf00bb js src/a.js 1:5 - 1:10 not run Condition -> false
1b903677268c6bfb2b0ce68d73c3afb713be309c959c26503fb4212dc59e79ea js src/a.js 1:7 - 1:8 not run BinaryOp(Gt) -> <=
53b9b358ff24fbeb7903eb52c3a12dfc162fc922fb34696fc211bc0f84a478d2 js src/a.js 1:7 - 1:8 not run BinaryOp(Gt) -> >=
719d9d6a24082ae6e114adbe1d942f88f05b570d47b97803712d590707f1b955 js src/a.js 1:9 - 1:10 not run Literal(Number) -> 0
ea287652cac5f3a566a8fb4c582421a8cd07cc69ff7fba122066c6b166297319 js src/a.js 1:9 - 1:10 not run Literal(Number) -> 1
368ca084999c641c241259a008c1db431c99c1ee612acf3ef68872729b0b20cb js src/a.js 1:9 - 1:10 not run Literal(Number) -> -1
c3cc1162a3ff5de12e88d90a1d5a3368b969daed14d81451061e9eddecccb9d4 js src/a.js 1:9 - 1:10 not run Literal(Number) -> Infinity
46b55de46dde476c7656278c4bff79a2eab355fd7323fb7bbd3a4cc4fd57df6d js src/a.js 1:9 - 1:10 not run Literal(Number) -> -Infinity
88796a09235eb87b07cb323b0de03f2f0f358b66ed90e52638755c8c1c117f84 js src/a.js 1:9 - 1:10 not run Literal(Number) -> NaN
901060bfab2e89e53f3a73d024e46566bc1f948712d24487e2564dcf0c2f9b5e js src/a.js 1:12 - 1:14 not run StatementBlock -> {}
"
    );
}

#[test]
fn show_mutations_empty() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "// just a comment")
        .build();

    let result = fixture.run("show mutations");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "\n");
}

#[test]
fn show_mutations_json() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("show mutations -f json");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        r#"[[{"mutation":{"mutant":{"lang":"js","twig":["src/index.js"],"kind":{"Literal":"BoolTrue"},"subst_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}},"effect_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}}},"subst":"false"},"outcome":null}]]
"#
    );
}

#[test]
fn show_mutations_python() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.py]
include = ["**/*.py"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/main.py", "assert x > 1")
        .build();

    let result = fixture.run("show mutations");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "\
da4a7ef3ecacf4977c2b744682ed76b22a3ea1db371cd1b477247e06aa26894a py src/main.py 1:1 - 1:13 not run Assert -> pass
5de891d5e00fa25b9f491c43596318b4fbc294f9da202c4ca3145a117d757faa py src/main.py 1:10 - 1:11 not run BinaryOp(Gt) -> <=
b7ba3768839be49c165232d6d4262f5649912b060c5686f45ec69f4f0add900a py src/main.py 1:10 - 1:11 not run BinaryOp(Gt) -> >=
434f82a02aeb74cf832ef8d398c7cdbb7f65e223fd2e2500cc1ce9ef45750a58 py src/main.py 1:12 - 1:13 not run Literal(Number) -> 0
375fd00053c7c94c92e2b088317e77c1d649961326ea0da97f8c012d3f18aaec py src/main.py 1:12 - 1:13 not run Literal(Number) -> 1
ab4f23d2b1c743cb754aab1572ec12967c177efe8558dfbcfcb002b5c1393d86 py src/main.py 1:12 - 1:13 not run Literal(Number) -> -1
8bd51618f60b3432cdfd37f54a882fdbafdfbb317ebd3f0611e3537ef96083ec py src/main.py 1:12 - 1:13 not run Literal(Number) -> float('inf')
8bc12fcc09b00b769f8ec1fd6aaa73302d8e9371ec16a85914a577f1df73a4a5 py src/main.py 1:12 - 1:13 not run Literal(Number) -> float('-inf')
b2d036e91ef2daa67892e8979fa0544e295ee168caf6dee9914df28181b37649 py src/main.py 1:12 - 1:13 not run Literal(Number) -> float('nan')
"
    );
}

#[test]
fn show_mutation_by_hash() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run(
        "show mutation c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8",
    );

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8 js src/index.js 1:1 - 1:5 not run Literal(BoolTrue) -> false\n"
    );
}

#[test]
fn show_mutation_invalid_hash() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("show mutation deadbeef");

    assert_ne!(result.code, 0);
}

#[test]
fn show_mutation_json() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run(
        "show mutation c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8 -f json",
    );

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        r#"{"mutation":{"mutant":{"lang":"js","twig":["src/index.js"],"kind":{"Literal":"BoolTrue"},"subst_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}},"effect_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}}},"subst":"false"},"outcome":null}
"#
    );
}

#[test]
fn show_files_all() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "true")
        .with_file("src/b.js", "false")
        .build();

    let result = fixture.run("show files");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.redacted_stdout(&fixture),
        "<TMP>/src/a.js <TMP>/src/b.js\n"
    );
}

#[test]
fn show_files_lang_filter() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[lang.ts]
include = ["**/*.ts"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "true")
        .with_file("src/b.ts", "true")
        .build();

    let result = fixture.run("show files ts");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.redacted_stdout(&fixture),
        "ts <TMP>/src/b.ts\n"
    );
}

#[test]
fn show_files_no_matches() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.ts", "true")
        .build();

    let result = fixture.run("show files js");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(result.redacted_stdout(&fixture), "js \n");
}

#[test]
fn show_files_respects_exclude() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = ["src/vendor/**"]

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/a.js", "true")
        .with_file("src/vendor/b.js", "true")
        .build();

    let result = fixture.run("show files");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.redacted_stdout(&fixture),
        "<TMP>/src/a.js\n"
    );
}

#[test]
fn show_config_json() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("show config -f json");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.redacted_stdout(&fixture),
        r#"{"workers":1,"threads":1,"base_root_dir":"<TMP>","include":["src/**"],"exclude":[],"lang":{"js":{"include":["**/*.js"],"exclude":[],"skip":null}},"pwd":null,"env":null,"timeout":null,"test":{"cmd":"echo test","pwd":null,"env":null,"timeout":null},"init":null,"reset":null,"find":{"number":1,"number_per_file":1,"factors":["EncompasingMissedMutationsCount","TSNodeDepth"]}}
"#
    );
}

#[test]
fn show_config_terse() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("show config");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.redacted_stdout(&fixture),
        r#"{"workers":1,"threads":1,"base_root_dir":"<TMP>","include":["src/**"],"exclude":[],"lang":{"js":{"include":["**/*.js"],"exclude":[],"skip":null}},"pwd":null,"env":null,"timeout":null,"test":{"cmd":"echo test","pwd":null,"env":null,"timeout":null},"init":null,"reset":null,"find":{"number":1,"number_per_file":1,"factors":["EncompasingMissedMutationsCount","TSNodeDepth"]}}
"#
    );
}

#[test]
fn tend_state_fresh() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("step tend-state");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "+1 -0\n");

    let state_dir = fixture.path().join(".bough/state");
    let entries: Vec<_> = std::fs::read_dir(&state_dir)
        .expect("state dir should exist")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries,
        vec!["c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8.json"]
    );

    let state_content = std::fs::read_to_string(
        state_dir.join("c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8.json"),
    )
    .unwrap();
    assert_eq!(
        state_content,
        r#"{"mutation":{"mutant":{"lang":"js","twig":["src/index.js"],"kind":{"Literal":"BoolTrue"},"subst_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}},"effect_span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":4,"byte":4}}},"subst":"false"},"outcome":null}"#
    );
}

#[test]
fn tend_state_idempotent() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    fixture.run("step tend-state");
    let result = fixture.run("step tend-state");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(result.stdout, "+0 -0\n");

    let count = std::fs::read_dir(fixture.path().join(".bough/state"))
        .unwrap()
        .count();
    assert_eq!(count, 1);
}

#[test]
fn tend_state_removes_stale() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let first = fixture.run("step tend-state");
    assert_eq!(first.stdout, "+1 -0\n");

    std::fs::remove_file(fixture.path().join("src/index.js")).unwrap();

    let second = fixture.run("step tend-state");
    assert_eq!(second.code, 0);
    assert_eq!(second.stderr, "");
    assert_eq!(second.stdout, "+0 -1\n");

    let count = std::fs::read_dir(fixture.path().join(".bough/state"))
        .unwrap()
        .count();
    assert_eq!(count, 0);
}

#[test]
fn tend_state_json() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("step tend-state -f json");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        r#"{"added":["c647a18bc3123b913cf096283cb24f46f49b73c5bc91026e82e85c8b6ccf13b8"],"removed":[]}
"#
    );
}

#[test]
fn tend_workspaces_creates_dirs() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("step tend-workspaces");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    let ids: Vec<&str> = result.stdout.trim().split_whitespace().collect();
    assert_eq!(ids.len(), 1);

    let ws_dir = fixture
        .path()
        .join(".bough/workspaces/work")
        .join(ids[0]);
    assert!(ws_dir.is_dir(), "workspace dir should exist");
    assert!(
        ws_dir.join("src/index.js").is_file(),
        "workspace should contain source files"
    );
    assert_eq!(
        std::fs::read_to_string(ws_dir.join("src/index.js")).unwrap(),
        "true"
    );
}

#[test]
fn tend_workspaces_workers_count() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
workers = 2
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let result = fixture.run("step tend-workspaces");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    let ids: Vec<&str> = result.stdout.trim().split_whitespace().collect();
    assert_eq!(ids.len(), 2);

    for id in &ids {
        let ws_dir = fixture
            .path()
            .join(".bough/workspaces/work")
            .join(id);
        assert!(ws_dir.is_dir(), "workspace {id} dir should exist");
        assert_eq!(
            std::fs::read_to_string(ws_dir.join("src/index.js")).unwrap(),
            "true"
        );
    }
}

#[test]
fn tend_workspaces_idempotent() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "true")
        .build();

    let first = fixture.run("step tend-workspaces");
    let first_ids: Vec<&str> = first.stdout.trim().split_whitespace().collect();

    let second = fixture.run("step tend-workspaces");
    assert_eq!(second.code, 0);
    let second_ids: Vec<&str> = second.stdout.trim().split_whitespace().collect();

    assert_eq!(first_ids, second_ids);
}

#[test]
fn show_mutations() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#,
        )
        .with_file("src/index.js", "if (x > 1) {}")
        .build();

    let result = fixture.run("show mutations");

    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, "");
    assert_eq!(
        result.stdout,
        "\
a40cc96b702219f1526e829e301381121f416f559a2b012785bfac4b68488e5b js src/index.js 1:5 - 1:10 not run Condition -> true
f17d586921ddec3b68cbdc06c976fc4349ca9ce0bc7b76be4efb889d642b9540 js src/index.js 1:5 - 1:10 not run Condition -> false
6027fb605ff10f770a29dd5dd7cca5beeb09aa7b42c94af52f2bffcd56c5f7b5 js src/index.js 1:7 - 1:8 not run BinaryOp(Gt) -> <=
d882e8b7c34208b68e79cd40dbee0622377369fe103c08678b19b77716048a84 js src/index.js 1:7 - 1:8 not run BinaryOp(Gt) -> >=
ef4d77bc324b67635e0336dee9299b1fd0a367ac0e097606ed229f98ca0c62eb js src/index.js 1:9 - 1:10 not run Literal(Number) -> 0
93ccfccd1890fb9184deb677b4cc46989e37b4aaabb92ca4aa43024f5f4d05c2 js src/index.js 1:9 - 1:10 not run Literal(Number) -> 1
804d1eaf978220f3c0a41094547b946b74d799823590d1c958a5be4f6e2125b2 js src/index.js 1:9 - 1:10 not run Literal(Number) -> -1
59907ff456079c1e540cc1049d1df203cc1913af107d2240a029c086dc3d4c29 js src/index.js 1:9 - 1:10 not run Literal(Number) -> Infinity
7a605e73313255d2844f5633337a34ad4761fd2f8eb4516130f3c580cd05e138 js src/index.js 1:9 - 1:10 not run Literal(Number) -> -Infinity
638f7af4fd2016ed5907a302a4ce6343a4f3c92e7a93f7360d94cd9d622e55f3 js src/index.js 1:9 - 1:10 not run Literal(Number) -> NaN
09f483e870130067a920cf780d175cc0af0a54970ef5733bcc1ad88b9f374433 js src/index.js 1:12 - 1:14 not run StatementBlock -> {}
"
    );
}
