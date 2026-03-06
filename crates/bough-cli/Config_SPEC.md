## Config

cli[config.include.at-least-one]
cli must have at least 1 include path

cli[config.exclude.from-vcs-ignore]
Config::get_base_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

cli[config.lang.exclude.from-vcs-ignore]
Config::get_lang_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

cli[config.exclude.from-vcs-dir]
Config::get_base_exclude-globs must include all vcs dirs

cli[config.lang.exclude.from-vcs-dir]
Config::get_lang_exclude-globs must include all vcs dirs

cli[config.exclude.bough-dir]
Config::get_base_exclude_globs must include a matcher for the current bough_dir

cli[config.lang.exclude.bough-dir]
Config::get_lang_exclude_globs must include a matcher for the current bough_dir

cli[config.lang.exclude.derived]
Config::get_lang_exclude_globs must include all exclude paths from Config::get_base_exclude_globs

cli[config.lang.include.derived]
Config::get_lang_include_globs must not include include paths from Config::get_base_include_globs

cli[config.base-root-path]
`Config::get_base_root_path` should resolve to the correct dir

cli[config.base-root-path.sub]
`Config::get_base_root_path` should resolve correctly when the config file was located in a sub directory

cli[config.base-root-path.parent]
`Config::get_base_root_path` should resolve correctly when the config file was located in a parent directory

cli[config.base-root-path.parent.sub]
`Config::get_base_root_path` should resolve correctly when the config file was located in the sub dir of a parent directory
