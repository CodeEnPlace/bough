## Config

cli[config.include.at-least-one]
cli must have at least 1 include path

cli[config.exclude.from-vcs-ignore]
Config::get_base_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

cli[config.lang.exclude.from-vcs-ignore]
Config::get_lang_exclude-globs must include all globs from all vcs ignore files that would be picked up by a vcs in base dir

cli[config.exclude.bough-dir]
Config::get_base_exclude_globs must include a matcher for the current bough_dir

cli[config.lang.exclude.bough-dir]
Config::get_lang_exclude_globs must include a matcher for the current bough_dir
