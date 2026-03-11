use facet::Facet;

#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Factor {
    /// How many authors have touched this file
    FileAuthorCount,
    /// Severity of the mutation operator (e.g. removing a null check vs flipping a comparator)
    MutationSeverity,
    /// How many mutations have a span that includes this mutation
    OuterMissedMutations,
    /// How many other surviving mutants exist in the same function
    SiblingMissedMutations,
    /// How many distinct mutation operator types survive in the same function
    SiblingOperatorDiversity,
    /// How deep into the tree-sitteer node graph is this mutation
    TSNodeDepth,
    /// How many times has this file been modified in version control
    VcsFileChurn,
    /// How recently was this line touched
    VcsLineBlameRecency,
}
