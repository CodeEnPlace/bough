use chrono::{DateTime, Utc};
use facet::Facet;
use tracing::trace;

use crate::{mutation::Mutation, test_id::TestId};

#[derive(Facet)]
#[repr(u8)]
pub enum Status {
    Caught,
    CaughtByTests(Vec<TestId>),
    Missed,
}

#[derive(Facet)]
struct Outcome {
    status: Status,
    at: DateTime<Utc>,
}

#[derive(Facet)]
pub struct State {
    mutation: Mutation,
    outcome: Option<Outcome>,
}

impl State {
    pub fn new(mutation: Mutation) -> Self {
        trace!(subst = mutation.subst(), "creating new state entry");
        Self {
            mutation,
            outcome: None,
        }
    }

    pub fn mutation(&self) -> &Mutation {
        &self.mutation
    }

    pub fn has_outcome(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn set_outcome(&mut self, status: Status) {
        self.outcome = Some(Outcome {
            status,
            at: Utc::now(),
        });
    }

    pub fn status(&self) -> Option<&Status> {
        self.outcome.as_ref().map(|o| &o.status)
    }

    pub fn outcome_at(&self) -> Option<DateTime<Utc>> {
        self.outcome.as_ref().map(|o| o.at)
    }
}
