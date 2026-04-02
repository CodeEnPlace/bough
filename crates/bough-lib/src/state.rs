use chrono::{DateTime, Utc};
use facet::Facet;
use tracing::trace;

use bough_core::Mutation;
use crate::test_id::TestId;

#[derive(Facet, Clone, PartialEq)]
#[repr(u8)]
pub enum Status {
    Caught,
    CaughtByTests(Vec<TestId>),
    Timeout,
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
        if let Some(outcome) = &self.outcome {
            if outcome.status == status {
                return;
            }
        }

        //TODO if the status isn't changing, this shouldn't update anything
        // so we don't get `at` churn
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
