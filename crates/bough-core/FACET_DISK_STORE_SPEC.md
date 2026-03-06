## FacetDiskStore

core[fds.type]
`FacetDiskStore<Key: Display + FromStr, Val: Facet>`

core[fds.keys]
`FacetDiskStore::keys` iterates all keys that have been inserted into it

core[fds.get]
`FacetDiskStore::get(&key) -> Option<&val>` returns val by key

core[fds.set]
`FacetDiskStore::get(key, val) -> Result<(), _>` inserts val by key

core[fds.remove]
`FacetDiskStore::remove(key) -> Option<Val>` removes val by key

core[fds.new]
`FacetDiskStore::new(dir: PathBuf) -> Self` points fds at a specified directory

core[fds.files]
fds should store all vals on disk at `$dir/$key.json` in json format

core[fds.live]
fds should handle all actions by reading/writing disk, and not cache in memory.

core[fds.live.intercepted]
if the directory is altered by some other process/function, the most up-to-date state should be refected by fds

core[fds.live.startup]
fds should include files that were on disk before it was created
