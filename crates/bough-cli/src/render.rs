use std::path::PathBuf;

use bough_core::{LanguageId, Mutation, MutationHash, State, Status};
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};
use facet::Facet;

use crate::config::{Cli, Config, Format};

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub(crate) const RESET: &str = "\x1b[0m";
pub(crate) const BOLD: &str = "\x1b[1m";

pub(crate) const TITLE: &str = "\x1b[33m"; // yellow
pub(crate) const PATH: &str = "\x1b[34m"; // blue
pub(crate) const HASH: &str = "\x1b[33m"; // yellow
pub(crate) const LANG: &str = "\x1b[35m"; //purple
pub(crate) const STRING: &str = "\x1b[33m"; //yellow
pub(crate) const CAUGHT: &str = "\x1b[32m"; // green
pub(crate) const MISSED: &str = "\x1b[31m"; // red
pub(crate) const TIMEOUT: &str = "\x1b[31m"; // red
pub(crate) const NOT_RUN: &str = "\x1b[33m"; // yellow

pub trait Render {
    fn markdown(&self) -> String;
    fn terse(&self) -> String;
    fn verbose(&self) -> String;
    fn json(&self) -> String;

    fn render(&self, cli: &Cli) -> String {
        let out = match cli.format {
            Format::Terse => self.terse(),
            Format::Verbose => self.verbose(),
            Format::Markdown => self.markdown(),
            Format::Json => self.json(),
        };
        if cli.color() { out } else { strip_ansi(&out) }
    }
}

#[derive(Facet)]
pub struct Noop;
impl Render for Noop {
    fn markdown(&self) -> String {
        String::new()
    }

    fn terse(&self) -> String {
        String::new()
    }

    fn verbose(&self) -> String {
        String::new()
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}



fn mutation_hash(m: &Mutation) -> String {
    let hash = m.hash().expect("hashing should not fail");
    format!("{hash}")
}

fn fmt_status_colored(state: &State) -> String {
    match state.status() {
        Some(Status::Caught) | Some(Status::CaughtByTests(_)) => {
            format!("{CAUGHT}Caught {RESET}")
        }
        Some(Status::Timeout) => format!("{TIMEOUT}Timeout{RESET}"),
        Some(Status::Missed) => format!("{MISSED}Missed {RESET}"),
        None => format!("{NOT_RUN}Not Run{RESET}"),
    }
}

fn fmt_mutation_terse(s: &State) -> String {
    let m = s.mutation();
    let mutant = m.mutant();
    format!(
        "{HASH}{}{RESET} {} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
        mutation_hash(m),
        fmt_status_colored(s),
        mutant.twig().path().display(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.kind(),
        m.subst(),
    )
}

fn fmt_mutation_markdown_row(m: &Mutation) -> String {
    let mutant = m.mutant();
    format!(
        "| `{}` | {} | {:?} | {:?} | {}:{}-{}:{} | `{}` |",
        mutation_hash(m),
        mutant.twig().path().display(),
        mutant.lang(),
        mutant.kind(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.span().end().line() + 1,
        mutant.span().end().col() + 1,
        m.subst(),
    )
}

const MARKDOWN_TABLE_HEADER: &str =
    "| Hash | File | Lang | Kind | Span | Subst |\n| --- | --- | --- | --- | --- | --- |";

fn fmt_mutation_markdown_table(mutations: &[Mutation]) -> String {
    let rows: Vec<_> = mutations.iter().map(fmt_mutation_markdown_row).collect();
    format!("{MARKDOWN_TABLE_HEADER}\n{}", rows.join("\n"))
}

fn fmt_mutation_verbose(s: &State) -> String {
    let m = s.mutation();
    let mutant = m.mutant();
    format!(
        "{HASH}{}{RESET} {} {PATH}{}{RESET} [{LANG}{:?}{RESET}] {:?} @ {}:{}-{}:{} → {STRING}\"{}\"{RESET}",
        mutation_hash(m),
        fmt_status_colored(s),
        mutant.twig().path().display(),
        mutant.lang(),
        mutant.kind(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.span().end().line() + 1,
        mutant.span().end().col() + 1,
        m.subst(),
    )
}

fn fmt_status_from_state(state: &State) -> &'static str {
    match state.status() {
        None => "pending",
        Some(Status::Caught) | Some(Status::CaughtByTests(_)) => "caught",
        Some(Status::Missed) => "missed",
        Some(Status::Timeout) => "timed out",
    }
}

fn fmt_outcome_at_from_state(state: &State) -> String {
    match state.outcome_at() {
        None => "-".to_string(),
        Some(at) => at.format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

#[derive(Facet)]
pub struct AllMutations(pub Vec<State>);
impl Render for AllMutations {
    fn markdown(&self) -> String {
        let mutations: Vec<_> = self.0.iter().map(|s| s.mutation().clone()).collect();
        format!(
            "# All Mutations\n\n{} total\n\n{}",
            self.0.len(),
            fmt_mutation_markdown_table(&mutations),
        )
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(fmt_mutation_terse)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(fmt_mutation_verbose)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct LangMutations(pub LanguageId, pub Vec<State>);
impl Render for LangMutations {
    fn markdown(&self) -> String {
        let mutations: Vec<_> = self.1.iter().map(|s| s.mutation().clone()).collect();
        format!(
            "# {:?} Mutations\n\n{} total\n\n{}",
            self.0,
            self.1.len(),
            fmt_mutation_markdown_table(&mutations),
        )
    }

    fn terse(&self) -> String {
        self.1
            .iter()
            .map(fmt_mutation_terse)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        self.1
            .iter()
            .map(fmt_mutation_verbose)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct FileMutations(pub LanguageId, pub PathBuf, pub Vec<State>);
impl Render for FileMutations {
    fn markdown(&self) -> String {
        let mutations: Vec<_> = self.2.iter().map(|s| s.mutation().clone()).collect();
        format!(
            "# Mutations in {}\n\n{} total\n\n{}",
            self.1.display(),
            self.2.len(),
            fmt_mutation_markdown_table(&mutations),
        )
    }

    fn terse(&self) -> String {
        self.2
            .iter()
            .map(fmt_mutation_terse)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        self.2
            .iter()
            .map(fmt_mutation_verbose)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

pub fn find_mutation_by_hash(hash: &str, mutations: Vec<Mutation>) -> Mutation {
    let unvalidated = UnvalidatedHash::new(hash.to_string());
    let hashes: Vec<_> = mutations
        .iter()
        .map(|m| m.hash().expect("hashing should not fail"))
        .collect();
    let matched = unvalidated
        .validate(&hashes)
        .expect("hash resolution failed");
    let matched_bytes = matched.as_bytes();
    mutations
        .into_iter()
        .find(|m| m.hash().unwrap().as_bytes() == matched_bytes)
        .unwrap()
}

pub struct SingleMutation {
    pub state: State,
    pub before: String,
    pub after: String,
    pub lang: LanguageId,
}

impl SingleMutation {
    fn lang_tag(&self) -> &'static str {
        match self.lang {
            LanguageId::Javascript => "javascript",
            LanguageId::Typescript => "typescript",
        }
    }
}

impl Render for SingleMutation {
    fn markdown(&self) -> String {
        let m = self.state.mutation();
        let mutant = m.mutant();
        let status = fmt_status_from_state(&self.state);
        let at = fmt_outcome_at_from_state(&self.state);
        let tag = self.lang_tag();
        format!(
            "{TITLE}# Mutation{RESET}\n\n\
            - {BOLD}Hash:{RESET} {HASH}{}{RESET}\n\
            - {BOLD}File:{RESET} {PATH}{}{RESET}\n\
            - {BOLD}Lang:{RESET} {LANG}{:?}{RESET}\n\
            - {BOLD}Kind:{RESET} {:?}\n\
            - {BOLD}Span:{RESET} {}:{}-{}:{}\n\
            - {BOLD}Subst:{RESET} {STRING}{}{RESET}\n\
            - {BOLD}Status:{RESET} {}\n\
            - {BOLD}At:{RESET} {}\n\n\
            {TITLE}## Before{RESET}\n\n```{}\n{}\n```\n\n\
            {TITLE}## After{RESET}\n\n```{}\n{}\n```",
            mutation_hash(m),
            mutant.twig().path().display(),
            mutant.lang(),
            mutant.kind(),
            mutant.span().start().line() + 1,
            mutant.span().start().col() + 1,
            mutant.span().end().line() + 1,
            mutant.span().end().col() + 1,
            m.subst(),
            status,
            at,
            tag,
            self.before,
            tag,
            self.after,
        )
    }

    fn terse(&self) -> String {
        fmt_mutation_terse(&self.state)
    }

    fn verbose(&self) -> String {
        let status = fmt_status_from_state(&self.state);
        let at = fmt_outcome_at_from_state(&self.state);
        format!(
            "{}\nStatus: {} ({})\n\n--- before ---\n{}\n\n--- after ---\n{}",
            fmt_mutation_verbose(&self.state),
            status,
            at,
            self.before,
            self.after,
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(&self.state).unwrap()
    }
}

pub struct TendState {
    pub added: Vec<bough_core::MutationHash>,
    pub removed: Vec<bough_core::MutationHash>,
}

fn hash_list_json(hashes: &[bough_core::MutationHash]) -> String {
    let items: Vec<String> = hashes.iter().map(|h| format!("\"{h}\"")).collect();
    format!("[{}]", items.join(","))
}

impl Render for TendState {
    fn markdown(&self) -> String {
        format!(
            "# Tend State\n\n- Added: {}\n- Removed: {}",
            self.added.len(),
            self.removed.len(),
        )
    }

    fn terse(&self) -> String {
        format!("+{} -{}", self.added.len(), self.removed.len())
    }

    fn verbose(&self) -> String {
        let mut out = format!(
            "{TITLE}Tend State{RESET}\n\nAdded: {}, Removed: {}",
            self.added.len(),
            self.removed.len(),
        );
        for h in &self.added {
            out.push_str(&format!("\n  {HASH}+{h}{RESET}"));
        }
        for h in &self.removed {
            out.push_str(&format!("\n  {HASH}-{h}{RESET}"));
        }
        out
    }

    fn json(&self) -> String {
        format!(
            r#"{{"added":{},"removed":{}}}"#,
            hash_list_json(&self.added),
            hash_list_json(&self.removed),
        )
    }
}

pub struct TendWorkspaces {
    pub workspace_ids: Vec<bough_core::WorkspaceId>,
}

impl Render for TendWorkspaces {
    fn markdown(&self) -> String {
        let list = self
            .workspace_ids
            .iter()
            .map(|id| format!("- `{id}`"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "# Tend Workspaces\n\n{} total\n\n{list}",
            self.workspace_ids.len()
        )
    }

    fn terse(&self) -> String {
        self.workspace_ids
            .iter()
            .map(|id| format!("{HASH}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn verbose(&self) -> String {
        let list = self
            .workspace_ids
            .iter()
            .map(|id| format!("  {HASH}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}Tend Workspaces{RESET}\n\n{} total\n\n{list}",
            self.workspace_ids.len(),
        )
    }

    fn json(&self) -> String {
        let items: Vec<String> = self
            .workspace_ids
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect();
        format!("[{}]", items.join(","))
    }
}

pub struct InitWorkspace {
    pub workspace_id: bough_core::WorkspaceId,
    pub outcome: bough_core::PhaseOutcome,
}

fn fmt_phase_outcome_terse(outcome: &bough_core::PhaseOutcome) -> String {
    format!(
        "exit={} duration={:.2}s{}",
        outcome.exit_code(),
        outcome.duration().as_secs_f64(),
        if outcome.timed_out() {
            " TIMED_OUT"
        } else {
            ""
        },
    )
}

fn fmt_phase_outcome_verbose(outcome: &bough_core::PhaseOutcome) -> String {
    let stdout = String::from_utf8_lossy(outcome.stdout());
    let stderr = String::from_utf8_lossy(outcome.stderr());
    let mut out = format!(
        "Exit: {}\nDuration: {:.2}s\nTimed out: {}",
        outcome.exit_code(),
        outcome.duration().as_secs_f64(),
        outcome.timed_out(),
    );
    if !stdout.is_empty() {
        out.push_str(&format!("\n\n{TITLE}stdout{RESET}\n{stdout}"));
    }
    if !stderr.is_empty() {
        out.push_str(&format!("\n\n{TITLE}stderr{RESET}\n{stderr}"));
    }
    out
}

fn fmt_phase_outcome_json(outcome: &bough_core::PhaseOutcome) -> String {
    let stdout = String::from_utf8_lossy(outcome.stdout());
    let stderr = String::from_utf8_lossy(outcome.stderr());
    format!(
        r#"{{"exit_code":{},"duration_secs":{:.3},"timed_out":{},"stdout":"{}","stderr":"{}"}}"#,
        outcome.exit_code(),
        outcome.duration().as_secs_f64(),
        outcome.timed_out(),
        stdout
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n"),
        stderr
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n"),
    )
}

impl Render for InitWorkspace {
    fn markdown(&self) -> String {
        let stdout = String::from_utf8_lossy(self.outcome.stdout());
        let stderr = String::from_utf8_lossy(self.outcome.stderr());
        let mut out = format!(
            "# Init Workspace `{}`\n\n- Exit: {}\n- Duration: {:.2}s\n- Timed out: {}",
            self.workspace_id,
            self.outcome.exit_code(),
            self.outcome.duration().as_secs_f64(),
            self.outcome.timed_out(),
        );
        if !stdout.is_empty() {
            out.push_str(&format!("\n\n## stdout\n\n```\n{stdout}\n```"));
        }
        if !stderr.is_empty() {
            out.push_str(&format!("\n\n## stderr\n\n```\n{stderr}\n```"));
        }
        out
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {}",
            self.workspace_id,
            fmt_phase_outcome_terse(&self.outcome),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Init Workspace{RESET} {HASH}{}{RESET}\n\n{}",
            self.workspace_id,
            fmt_phase_outcome_verbose(&self.outcome),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","outcome":{}}}"#,
            self.workspace_id,
            fmt_phase_outcome_json(&self.outcome),
        )
    }
}

pub struct ResetWorkspace {
    pub workspace_id: bough_core::WorkspaceId,
    pub outcome: bough_core::PhaseOutcome,
}

impl Render for ResetWorkspace {
    fn markdown(&self) -> String {
        let stdout = String::from_utf8_lossy(self.outcome.stdout());
        let stderr = String::from_utf8_lossy(self.outcome.stderr());
        let mut out = format!(
            "# Reset Workspace `{}`\n\n- Exit: {}\n- Duration: {:.2}s\n- Timed out: {}",
            self.workspace_id,
            self.outcome.exit_code(),
            self.outcome.duration().as_secs_f64(),
            self.outcome.timed_out(),
        );
        if !stdout.is_empty() {
            out.push_str(&format!("\n\n## stdout\n\n```\n{stdout}\n```"));
        }
        if !stderr.is_empty() {
            out.push_str(&format!("\n\n## stderr\n\n```\n{stderr}\n```"));
        }
        out
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {}",
            self.workspace_id,
            fmt_phase_outcome_terse(&self.outcome),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Reset Workspace{RESET} {HASH}{}{RESET}\n\n{}",
            self.workspace_id,
            fmt_phase_outcome_verbose(&self.outcome),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","outcome":{}}}"#,
            self.workspace_id,
            fmt_phase_outcome_json(&self.outcome),
        )
    }
}

pub struct TestMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
    pub status: &'static str,
    pub duration: std::time::Duration,
}

impl Render for TestMutation {
    fn markdown(&self) -> String {
        format!(
            "# Test Mutation `{}` in `{}`\n\n- Status: {}\n- Duration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status,
            self.duration.as_secs_f64(),
        )
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {HASH}{}{RESET} {} {:.2}s",
            self.workspace_id,
            self.mutation_hash,
            self.status,
            self.duration.as_secs_f64(),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Test Mutation{RESET} {HASH}{}{RESET} in {HASH}{}{RESET}\n\nStatus: {}\nDuration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status,
            self.duration.as_secs_f64(),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","mutation_hash":"{}","status":"{}","duration_secs":{:.3}}}"#,
            self.workspace_id,
            self.mutation_hash,
            self.status,
            self.duration.as_secs_f64(),
        )
    }
}

pub struct BenchmarkTimesInBase {
    pub init: Option<std::time::Duration>,
    pub reset: Option<std::time::Duration>,
    pub test: std::time::Duration,
}

impl Render for BenchmarkTimesInBase {
    fn markdown(&self) -> String {
        let mut out = "# Benchmark Times (Base)\n".to_string();
        if let Some(d) = self.init {
            out.push_str(&format!("\n- Init: {:.2}s", d.as_secs_f64()));
        }
        if let Some(d) = self.reset {
            out.push_str(&format!("\n- Reset: {:.2}s", d.as_secs_f64()));
        }
        out.push_str(&format!("\n- Test: {:.2}s", self.test.as_secs_f64()));
        out
    }

    fn terse(&self) -> String {
        let mut parts = Vec::new();
        if let Some(d) = self.init {
            parts.push(format!("init={:.2}s", d.as_secs_f64()));
        }
        if let Some(d) = self.reset {
            parts.push(format!("reset={:.2}s", d.as_secs_f64()));
        }
        parts.push(format!("test={:.2}s", self.test.as_secs_f64()));
        parts.join(" ")
    }

    fn verbose(&self) -> String {
        let mut out = format!("{TITLE}Benchmark Times (Base){RESET}\n");
        if let Some(d) = self.init {
            out.push_str(&format!("\n  Init:  {:.2}s", d.as_secs_f64()));
        }
        if let Some(d) = self.reset {
            out.push_str(&format!("\n  Reset: {:.2}s", d.as_secs_f64()));
        }
        out.push_str(&format!("\n  Test:  {:.2}s", self.test.as_secs_f64()));
        out
    }

    fn json(&self) -> String {
        let init = match self.init {
            Some(d) => format!("{:.3}", d.as_secs_f64()),
            None => "null".to_string(),
        };
        let reset = match self.reset {
            Some(d) => format!("{:.3}", d.as_secs_f64()),
            None => "null".to_string(),
        };
        format!(
            r#"{{"init_secs":{},"reset_secs":{},"test_secs":{:.3}}}"#,
            init,
            reset,
            self.test.as_secs_f64(),
        )
    }
}

pub struct ApplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl Render for ApplyMutation {
    fn markdown(&self) -> String {
        format!(
            "# Apply Mutation\n\n- Workspace: `{}`\n- Mutation: `{}`",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {HASH}{}{RESET}",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Apply Mutation{RESET} {HASH}{}{RESET} to {HASH}{}{RESET}",
            self.mutation_hash, self.workspace_id,
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","mutation_hash":"{}"}}"#,
            self.workspace_id, self.mutation_hash,
        )
    }
}

pub struct UnapplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl Render for UnapplyMutation {
    fn markdown(&self) -> String {
        format!(
            "# Unapply Mutation\n\n- Workspace: `{}`\n- Mutation: `{}`",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {HASH}{}{RESET}",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Unapply Mutation{RESET} {HASH}{}{RESET} from {HASH}{}{RESET}",
            self.mutation_hash, self.workspace_id,
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","mutation_hash":"{}"}}"#,
            self.workspace_id, self.mutation_hash,
        )
    }
}

pub struct FindBestMutations(pub Vec<(MutationHash, State, f64)>);

impl Render for FindBestMutations {
    fn markdown(&self) -> String {
        let mut out = format!("# Find Best Mutations\n\n{} selected\n\n", self.0.len());
        out.push_str("| # | Score | Hash | File | Location | Kind | Subst |\n");
        out.push_str("|---|-------|------|------|----------|------|-------|\n");
        for (i, (hash, state, score)) in self.0.iter().enumerate() {
            let m = state.mutation();
            let mutant = m.mutant();
            out.push_str(&format!(
                "| {} | {:.2} | `{}` | {} | {}:{} | {:?} | `{}` |\n",
                i + 1,
                score,
                hash,
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                mutant.kind(),
                m.subst(),
            ));
        }
        out
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|(_, state, score)| {
                let m = state.mutation();
                let mutant = m.mutant();
                format!(
                    "{HASH}{}{RESET} {:.2} {} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
                    mutation_hash(m),
                    score,
                    fmt_status_colored(state),
                    mutant.twig().path().display(),
                    mutant.span().start().line() + 1,
                    mutant.span().start().col() + 1,
                    mutant.kind(),
                    m.subst(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let mut out = format!("{TITLE}Find Best Mutations{RESET} ({} selected)\n", self.0.len());
        for (i, (hash, state, score)) in self.0.iter().enumerate() {
            let m = state.mutation();
            let mutant = m.mutant();
            out.push_str(&format!(
                "\n  {HASH}#{}{RESET} score={:.2} {HASH}{}{RESET} {} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
                i + 1,
                score,
                hash,
                fmt_status_colored(state),
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                mutant.kind(),
                m.subst(),
            ));
        }
        out
    }

    fn json(&self) -> String {
        let items: Vec<String> = self.0.iter().map(|(hash, state, score)| {
            let m = state.mutation();
            let mutant = m.mutant();
            format!(
                r#"{{"hash":"{}","score":{},"file":"{}","line":{},"col":{},"kind":"{}","subst":"{}"}}"#,
                hash,
                score,
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                format!("{:?}", mutant.kind()),
                m.subst(),
            )
        }).collect();
        format!("[{}]", items.join(","))
    }
}

impl Render for Config {
    fn markdown(&self) -> String {
        format!(
            "# Bough Config

```json
{}
```",
            facet_json::to_string(self).unwrap()
        )
    }

    fn terse(&self) -> String {
        facet_json::to_string(self).unwrap()
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Bough Config{RESET}\n\n{}",
            facet_json::to_string(self).unwrap()
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}
