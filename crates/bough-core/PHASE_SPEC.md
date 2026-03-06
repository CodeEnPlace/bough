## Phase

bough[phase.root]
Phase holds &Root

bough[phase.pwd]
Phase holds a twig called pwd

bough[phase.env]
Phase holds a HashMap<String,String> of env vars to apply

bough[phase.cmd]
Phase::cmd is a Vec<String> that will be run as a sub process

bough[phase.timeout]
Phase::timeout is a TimeoutConfig

bough[phase.run]
`Phase::run() -> Result<_,_>` runs Phase::cmd

bough[phase.run.pwd]
Phase::run runs the command in File { root, pwd }

bough[phase.run.env]
Phase::run runs the command with the specified env vars

bough[phase.run.timeout]
Phase::run stops the command if it extends the provided timeout

bough[phase.run.timeout.absolute]
Phase::run stops the command if it extends the provided absolute timeout

bough[phase.run.timeout.relative]
Phase::run takes an optional Duration. it stops the command if this Duration is defined, and it extends the provided relative `timeout * provided_duration`

bough[phase.out]
`Phase::run -> Result<PhaseOutcome , _>`

bough[phase.out.stdio]
PhaseOutcome should retain all stdout and stderr from the command

bough[phase.out.exit]
PhaseOutcome should contain the exit code of the command, non-zero should return Ok(), not Err

bough[phase.out.duration]
PhaseOutcome should contain the duration time of the command

