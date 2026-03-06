## Session

core[session.init]
`Session::<Config>::new(config: Config) -> Result<Self, _>`

core[session.init.state.attach]
`Session::new` will create a mutations_state pointing at `Config::get_bough_state_dir() + "/state"`

core[session.init.state.get]
`Session::get_state()` returns ref to mutations_state

core[session.init.state.add-missing]
`Session::new` will add any mutations_in_base that are missing to mutations_state, defaulting `outcome` to `None`

core[session.init.state.remove-stale]
`Session::new` will remove any mutations_state that are missing from mutations_in_base
