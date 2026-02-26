use bough_cli_test::{TestPlan, cmd};

#[test]
fn no_args_shows_usage() {
    let dir = TestPlan::new().setup();
    cmd!(dir, "bough", "", "Usage: bough [OPTIONS] <COMMAND>");
}

#[test]
fn completions_bash() {
    let dir = TestPlan::new().setup();
    cmd!(dir, "bough completions bash", "complete -F _bough -o bashdefault -o default bough");
}
