## Session

bough[session.init+2]
`Session::<Config>::new(config: Config) -> Result<Self, _>`

bough[session.init.state.attach]
`Session::new` will create a mutations_state pointing at `Config::get_bough_state_dir() + "/state"`

bough[session.init.state.get]
`Session::get_state()` returns ref to mutations_state

bough[session.tend.state.add-missing]
`Session::tend_add_missing_states -> Iter<MutationHash>` will add any mutations_in_base that are missing to mutations_state, defaulting `outcome` to `None`, it returns the ones it added

bough[session.tend.state.remove-stale]
`Session::tend_remove_stale_states -> Iter<MutationHash>` will remove any mutations_state that are missing from mutations_in_base, it returns the ones it removed.

bough[session.tend.workspaces]
`Session::tend_workspaces(desired_count) -> Iter<WorkspaceId>` tends workspaces and returns the set of existing ones when it's done

bough[session.tend.workspaces.bind]
`Session::tend_workspaces` binds to existing workspaces

bough[session.tend.workspaces.bind.validate-unchanged.rm]
if the bound workspace fails validate_unchanged, it is removed from disk

bough[session.tend.workspaces.bind.validate-unchanged.forget]
if the bound workspace fails validate_unchanged, the Workspace struct is dropped

bough[session.tend.workspaces.new]
After binding to clean existing dirs, new ones are created until desired_count is reached

bough[session.tend.workspaces.surplus]
If more exist on disk than are desired, the surplus are removed
