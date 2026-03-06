## FacetDiskStore

bough[fds.type]
`FacetDiskStore<Key: Display + FromStr, Val: Facet>`

bough[fds.keys]
`FacetDiskStore::keys` iterates all keys that have been inserted into it

bough[fds.keys.invalid]
if `FacetDiskStore::keys` finds an invalidly named file, it should be skipped

bough[fds.get]
`FacetDiskStore::get(&key) -> Option<&val>` returns val by key

bough[fds.get.invalid]
if `FacetDiskStore::get(&key)` finds an invalid on-disk file, it should return None

bough[fds.set]
`FacetDiskStore::get(key, val) -> Result<(), _>` inserts val by key

bough[fds.remove]
`FacetDiskStore::remove(key) -> Option<Val>` removes val by key

bough[fds.new]
`FacetDiskStore::new(dir: PathBuf) -> Self` points fds at a specified directory

bough[fds.new.mkdir]
if dir doesn't exist, it should be created

bough[fds.files]
fds should store all vals on disk at `$dir/$key.json` in json format

bough[fds.live]
fds should handle all actions by reading/writing disk, and not cache in memory.

bough[fds.live.intercepted]
if the directory is altered by some other process/function, the most up-to-date state should be refected by fds

bough[fds.live.startup]
fds should include files that were on disk before it was created
