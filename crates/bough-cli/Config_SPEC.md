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

cli[config.base-root-path+2]
`Config::get_base_root_path` should be set from a value in the config file

cli[config.base-root-path.relative-from-file]
base_root_path should be absolutized to be relative from the config file's location, just before validation

cli[config.base-root-path.wherever]
base_root_path should be absolutized correctly, depending on where the config file was sourced from.
