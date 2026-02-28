use std::collections::HashMap;

use crate::WorkspaceId;
use crate::config::Config;
use crate::workspace::Workspace;

pub enum Error {}
pub struct Session {
    config: Config,
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl Session {
    pub fn new(config: Config) -> Result<Self, Error> {
        let workspaces = HashMap::new();
        let me = Self { config, workspaces };

        Ok(me)
    }
}
