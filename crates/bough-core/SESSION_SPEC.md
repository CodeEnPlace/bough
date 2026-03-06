## Session

core[session.init]
`Session::<Config>::new(config: Config) -> Result<Self, _>`

core[session.mutations_in_base]
`Session::mutations_in_base -> &HashSet<Mutation>` returns all the possible mutations found in the base

core[session.mutations_on_disk]
`Session::mutations_on_disk -> Iter<&MutationState>` returns all mutations
