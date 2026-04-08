use bough_config::SessionConfig;
use bough_core::mutant::TwigMutantsIter;
use bough_core::{Mutant, Mutation, MutationIter};
use bough_dirs::{Base, Work};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::{info, trace, warn};

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
    let twigs: Vec<_> = base.mutant_twigs().collect();
    let num_twigs = twigs.len();
    let pool = rayon_pool(config);
    let threads = pool.current_num_threads();
    let busy_nanos = AtomicU64::new(0);
    let wall_start = Instant::now();
    let result: Vec<_> = pool.install(|| {
        twigs
            .par_iter()
            .flat_map(|(language_id, twig)| {
                let t0 = Instant::now();
                trace!(lang = ?language_id, twig = %twig.path().display(), "scanning twig for mutants");
                let out = match TwigMutantsIter::new(
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
                };
                busy_nanos.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
                out
            })
            .collect()
    });
    let wall = wall_start.elapsed();
    let busy = std::time::Duration::from_nanos(busy_nanos.load(Ordering::Relaxed));
    let ideal = wall.as_secs_f64() * threads as f64;
    let utilization = if ideal > 0.0 { busy.as_secs_f64() / ideal } else { 0.0 };
    info!(
        twigs = num_twigs,
        threads,
        wall_ms = wall.as_millis() as u64,
        busy_ms = busy.as_millis() as u64,
        utilization = format!("{:.1}%", utilization * 100.0),
        "mutants parsing profile"
    );
    result
}

pub fn mutations(
    base: &Base,
    config: &impl SessionConfig,
) -> Vec<std::io::Result<Mutation>> {
    let ms = mutants(base, config);
    let num_mutants = ms.len();
    let pool = rayon_pool(config);
    let threads = pool.current_num_threads();
    let busy_nanos = AtomicU64::new(0);
    let wall_start = Instant::now();
    let result: Vec<_> = pool.install(|| {
        ms.into_par_iter()
            .flat_map(|r| {
                let t0 = Instant::now();
                let out = match r {
                    Ok(m) => {
                        let driver = crate::language::driver_for_lang(m.lang());
                        MutationIter::new(&m, driver.as_ref())
                            .map(Ok)
                            .collect::<Vec<_>>()
                    }
                    Err(e) => vec![Err(e)],
                };
                busy_nanos.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
                out
            })
            .collect()
    });
    let wall = wall_start.elapsed();
    let busy = std::time::Duration::from_nanos(busy_nanos.load(Ordering::Relaxed));
    let ideal = wall.as_secs_f64() * threads as f64;
    let utilization = if ideal > 0.0 { busy.as_secs_f64() / ideal } else { 0.0 };
    info!(
        mutants = num_mutants,
        threads,
        wall_ms = wall.as_millis() as u64,
        busy_ms = busy.as_millis() as u64,
        utilization = format!("{:.1}%", utilization * 100.0),
        "mutations expansion profile"
    );
    result
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
