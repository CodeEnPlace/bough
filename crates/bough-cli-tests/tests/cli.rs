use bough_cli_tests::Fixture;

#[test]
fn noop_with_valid_config() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            "\
base_root_dir = \".\"
include = [\"src/**\"]
exclude = []

[lang.js]
include = [\"**/*.js\"]
exclude = []

[test]
cmd = \"echo test\"
",
        )
        .with_file("src/index.js", "if (x > 1) {}")
        .build();

    let result = fixture.run("noop");

    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, "\n");
    assert_eq!(result.stderr, "");
}

#[test]
fn show_mutations() {
    let fixture = Fixture::new()
        .with_file(
            "bough.config.toml",
            "\
base_root_dir = \".\"
include = [\"src/**\"]
exclude = []

[lang.js]
include = [\"**/*.js\"]
exclude = []

[test]
cmd = \"echo test\"
",
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
