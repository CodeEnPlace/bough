use bough_cli_test::{TestPlan, cmd, cmd_err};

mod discovery {
    use super::*;

    #[test]
    fn finds_bough_config_toml() {
        let dir = TestPlan::new()
            .config("[runner]\npwd = \".\"")
            .setup();

        cmd!(dir, "bough --output-style json show config", "{!json}");
    }

    #[test]
    fn no_config_errors() {
        let dir = TestPlan::new().setup();
        cmd_err!(dir, "bough show config", "no config file found");
    }
}

mod overrides {
    use super::*;

    #[test]
    fn inline_config_overrides_file() {
        let dir = TestPlan::new()
            .config("parallelism = 1")
            .setup();

        cmd!(
            dir,
            "bough --config parallelism=42 --output-style json show config",
            "{!json}",
        );

        assert!(json.contains("\"parallelism\":42"));
    }
}

mod output_styles {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new().config("parallelism = 3")
    }

    #[test]
    fn json_output() {
        let dir = plan().setup();
        cmd!(dir, "bough --output-style json show config", "{!json}");
        assert!(json.contains("\"parallelism\":3"));
    }

    #[test]
    fn yaml_output() {
        let dir = plan().setup();
        cmd!(dir, "bough --output-style yaml show config", "parallelism: 3");
    }

    #[test]
    fn toml_output() {
        let dir = plan().setup();
        cmd!(dir, "bough --output-style toml show config", "parallelism = 3");
    }
}
