use bough_cli_test::{TestPlan, cmd};

mod jj {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[vcs]
kind = "jj"
rev = "bough"

[dirs]
working = "./work"

[runner]
pwd = "."
"#,
            )
            .file("work/.keep", "")
    }

    #[test]
    fn make_and_list() {
        let dir = plan().setup();

        cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
        cmd!(
            dir,
            "bough --output-style verbose workspace list",
            "1 workspaces",
            "{!id_1} {?ws_path}",
        );
    }

    #[test]
    fn make_two_and_list() {
        let dir = plan().setup();

        cmd!(dir, "bough workspace make", "created workspace at {!ws_path_1}");
        cmd!(dir, "bough workspace make", "created workspace at {!ws_path_2}");

        assert_ne!(ws_path_1, ws_path_2);

        cmd!(
            dir,
            "bough --output-style verbose workspace list",
            "2 workspaces",
            "{!id_1} {?ws_path_1}",
            "{!id_2} {?ws_path_2}",
        );
    }

    #[test]
    fn make_and_drop() {
        let dir = plan().setup();

        cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
        cmd!(
            dir,
            "bough --output-style verbose workspace list",
            "{!id_1} {?ws_path}",
        );
        cmd!(dir, "bough workspace drop {?id_1}", "dropped workspace {?id_1}");
        cmd!(dir, "bough workspace list", "0 workspaces");
    }

    #[test]
    fn list_empty() {
        let dir = plan().setup();
        cmd!(dir, "bough workspace list", "0 workspaces");
    }
}

mod no_vcs {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[vcs]
kind = "none"

[dirs]
working = "./work"

[runner]
pwd = "."
"#,
            )
            .file("src/app.js", "export const x = 1;")
            .file("work/.keep", "")
    }

    #[test]
    fn make_copies_files() {
        let dir = plan().setup();

        cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
        cmd!(dir, "ls {?ws_path}/src/app.js", "{?ws_path}/src/app.js");
    }

    #[test]
    fn list_empty() {
        let dir = plan().setup();
        cmd!(dir, "bough workspace list", "0 workspaces");
    }
}
