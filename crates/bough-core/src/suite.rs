use crate::config::SuiteConfig;

pub struct Suite<'a> {
    pub config: &'a SuiteConfig,
}

impl<'a> Suite<'a> {
    pub fn new(config: &'a SuiteConfig) -> Self {
        Self { config }
    }
}
