use bough_cli_test::{TestPlan, cmd};

#[test]
fn no_args_shows_usage() {
    let dir = TestPlan::new().setup();
    cmd!(dir, "bough", "", "Usage");
}

#[test]
fn completions_bash() {
    let dir = TestPlan::new().setup();
    cmd!(dir, "bough completions bash", "complete");
}
