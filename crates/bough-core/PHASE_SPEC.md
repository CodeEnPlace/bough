## Phase

core[phase.root]
Phase holds &Root

core[phase.pwd]
Phase holds a twig called pwd

core[phase.env]
Phase holds a HashMap<String,String> of env vars to apply

core[phase.cmd]
Phase::cmd is a Vec<String> that will be run as a sub process

core[phase.timeout]
Phase::timeout is a TimeoutConfig

core[phase.run]
`Phase::run() -> Result<_,_>` runs Phase::cmd

core[phase.run.pwd]
Phase::run runs the command in File { root, pwd }

core[phase.run.env]
Phase::run runs the command with the specified env vars

core[phase.run.timeout]
Phase::run stops the command if it extends the provided timeout

core[phase.run.timeout.absolute]
Phase::run stops the command if it extends the provided absolute timeout

core[phase.run.timeout.relative]
Phase::run takes an optional Duration. it stops the command if this Duration is defined, and it extends the provided relative `timeout * provided_duration`

core[phase.out]
`Phase::run -> Result<PhaseOutcome , _>`

core[phase.out.stdio]
PhaseOutcome should retain all stdout and stderr from the command

core[phase.out.exit]
PhaseOutcome should contain the exit code of the command, non-zero should return Ok(), not Err

core[phase.out.duration]
PhaseOutcome should contain the duration time of the command

