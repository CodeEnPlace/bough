use chrono::{DateTime, Utc};
use facet::Facet;

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
