use crate::io::Action;
use bough_session::Session;
use crate::steps::{run_in_workspace, CommandReport};

pub fn run(session: &Session, workspace: &str) -> (Vec<Action>, Option<CommandReport>) {
    let report = run_in_workspace(
        session,
        workspace,
        &Some(session.commands.test.clone()),
        "test",
    );
    (vec![], report)
}
