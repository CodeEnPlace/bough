use facet::Facet;

use crate::mutation::Mutation;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "PascalCase")]
#[repr(u8)]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationOutcome {
    outcome: Option<Outcome>,
    // outcome_at: chrono::DateTime,
    mutation: Mutation,
}

// impl HashInto for MutationOutcome {
//     fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
//         self.mutation.hash_into(state)
//     }
// }
