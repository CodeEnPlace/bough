## Config

bough[config.include.at-least-one]
cli must have at least 1 include path

bough[config.exclude.from-vcs-ignore]
Config::get_base_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

bough[config.lang.exclude.from-vcs-ignore]
Config::get_lang_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

bough[config.exclude.from-vcs-dir]
Config::get_base_exclude-globs must include all vcs dirs

bough[config.lang.exclude.from-vcs-dir]
Config::get_lang_exclude-globs must include all vcs dirs

bough[config.exclude.bough-dir]
Config::get_base_exclude_globs must include a matcher for the current bough_dir

bough[config.lang.exclude.bough-dir]
Config::get_lang_exclude_globs must include a matcher for the current bough_dir

bough[config.lang.exclude.derived]
Config::get_lang_exclude_globs must include all exclude paths from Config::get_base_exclude_globs

bough[config.lang.include.derived]
Config::get_lang_include_globs must not include include paths from Config::get_base_include_globs

bough[config.base-root-path+2]
`Config::get_base_root_path` should be set from a value in the config file (Config::base_root_path: PathBuf)

bough[config.base-root-path.default]
Config::base_root_path is required, no default

bough[config.base-root-path.absolutized-relative-to-file]
Once the config has been resolved and parsed, the base_root_path should be absolutized relative to the directory containing the resolved config file.

Eg, if a config file contained `base_root_path = "./qux"`, and was found in `/foo/bar/config.toml`, the resolved value should be /foo/bar/qux.

If it was located in `/foo/bar/.config/bough.toml`, that same value would be resolved to /foo/bar/.config/qux, to achieve the same effect you would have to update the file to instead contain "../qux"

bough[config.base-root-path.relative-via-figue]
the location of the resolved config should be extracted from the figue processing, not calculated separately
