use bough_config::SessionConfig;
use bough_core::mutant::TwigMutantsIter;
use bough_core::{Mutant, Mutation, MutationIter};
use bough_dirs::{Base, Work};
use rayon::prelude::*;
use tracing::{trace, warn};

fn rayon_pool(config: &impl SessionConfig) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads().max(1) as usize)
        .build()
        .expect("build rayon pool")
}

pub fn mutants(
    base: &Base,
    config: &impl SessionConfig,
) -> Vec<std::io::Result<Mutant>> {
    let mut twigs: Vec<_> = base.mutant_twigs().collect();
    // LPT scheduling: process largest files first so they start on fresh threads
    // rather than ending up as stragglers that stretch wall time.
    twigs.sort_by_cached_key(|(_, twig)| {
        let path = bough_fs::File::new(base, twig).resolve();
        std::cmp::Reverse(std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0))
    });
    let pool = rayon_pool(config);
    pool.install(|| {
        twigs
            .par_iter()
            .with_min_len(1)
            .with_max_len(1)
            .flat_map(|(language_id, twig)| {
                trace!(lang = ?language_id, twig = %twig.path().display(), "scanning twig for mutants");
                match TwigMutantsIter::new(
                    *language_id,
                    base,
                    twig,
                    crate::language::driver_for_lang(*language_id),
                ) {
                    Ok(iter) => {
                        let iter = base
                            .skip_queries_for(*language_id)
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
            .collect()
    })
}

pub fn mutations(
    base: &Base,
    config: &impl SessionConfig,
) -> Vec<std::io::Result<Mutation>> {
    let ms = mutants(base, config);
    let pool = rayon_pool(config);
    pool.install(|| {
        ms.into_par_iter()
            .flat_map(|r| match r {
                Ok(m) => {
                    let driver = crate::language::driver_for_lang(m.lang());
                    MutationIter::new(&m, driver.as_ref())
                        .map(Ok)
                        .collect::<Vec<_>>()
                }
                Err(e) => vec![Err(e)],
            })
            .collect()
    })
}

pub fn run_test_in_base(
    base: &Base,
    config: &impl bough_config::SessionConfig,
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
    config: &impl bough_config::SessionConfig,
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
    config: &impl bough_config::SessionConfig,
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
    config: &impl bough_config::SessionConfig,
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
    config: &impl bough_config::SessionConfig,
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
    config: &impl bough_config::SessionConfig,
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
