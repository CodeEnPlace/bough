use bough_core::mutant::TwigMutantsIter;
use bough_core::{Mutant, Mutation, MutationIter};
use bough_dirs::{Base, Work};
use tracing::{trace, warn};

pub fn mutants(base: &Base) -> impl Iterator<Item = std::io::Result<Mutant>> + '_ {
    base.mutant_twigs().flat_map(|(language_id, twig)| {
        trace!(lang = ?language_id, twig = %twig.path().display(), "scanning twig for mutants");
        match TwigMutantsIter::new(
            language_id,
            base,
            &twig,
            crate::language::driver_for_lang(language_id),
        ) {
            Ok(iter) => {
                let iter = base
                    .skip_queries_for(language_id)
                    .iter()
                    .fold(iter, |iter, q| iter.with_skip_query(q));
                iter.map(|bm| Ok(bm.into_mutant())).collect::<Vec<_>>()
            }
            Err(e) => {
                warn!(lang = ?language_id, twig = %twig.path().display(), error = %e, "failed to scan twig for mutants");
                vec![Err(e)]
            }
        }
    })
}

pub fn mutations(base: &Base) -> impl Iterator<Item = std::io::Result<Mutation>> + '_ {
    mutants(base).flat_map(|r| match r {
        Ok(m) => {
            let driver = crate::language::driver_for_lang(m.lang());
            MutationIter::new(&m, driver.as_ref())
                .map(Ok)
                .collect::<Vec<_>>()
        }
        Err(e) => vec![Err(e)],
    })
}

pub fn run_test_in_base(
    base: &Base,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    crate::phase::run_phase_in_base(
        base,
        &config.get_test_cmd(),
        config.get_test_pwd(),
        config.get_test_env(),
        Some(config.get_test_timeout(reference_duration)),
    )
}

pub fn run_init_in_base(
    base: &Base,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    let cmd = config
        .get_init_cmd()
        .ok_or(crate::phase::Error::NoCmdConfigured)?;
    crate::phase::run_phase_in_base(
        base,
        &cmd,
        config.get_init_pwd(),
        config.get_init_env(),
        Some(config.get_init_timeout(reference_duration)),
    )
}

pub fn run_reset_in_base(
    base: &Base,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    let cmd = config
        .get_reset_cmd()
        .ok_or(crate::phase::Error::NoCmdConfigured)?;
    crate::phase::run_phase_in_base(
        base,
        &cmd,
        config.get_reset_pwd(),
        config.get_reset_env(),
        Some(config.get_reset_timeout(reference_duration)),
    )
}

pub fn run_test_in_workspace(
    workspace: &Work,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    crate::phase::run_phase_in_workspace(
        workspace,
        &config.get_test_cmd(),
        config.get_test_pwd(),
        config.get_test_env(),
        config.get_test_timeout(reference_duration),
    )
}

pub fn run_init_in_workspace(
    workspace: &Work,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    let cmd = config
        .get_init_cmd()
        .ok_or(crate::phase::Error::NoCmdConfigured)?;
    crate::phase::run_phase_in_workspace(
        workspace,
        &cmd,
        config.get_init_pwd(),
        config.get_init_env(),
        config.get_init_timeout(reference_duration),
    )
}

pub fn run_reset_in_workspace(
    workspace: &Work,
    config: &impl crate::session::Config,
    reference_duration: Option<chrono::Duration>,
) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
    let cmd = config
        .get_reset_cmd()
        .ok_or(crate::phase::Error::NoCmdConfigured)?;
    crate::phase::run_phase_in_workspace(
        workspace,
        &cmd,
        config.get_reset_pwd(),
        config.get_reset_env(),
        config.get_reset_timeout(reference_duration),
    )
}
