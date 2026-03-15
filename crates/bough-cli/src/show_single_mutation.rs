use std::ops::DerefMut;

use bough_core::{LanguageId, Session, State};
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};

use crate::config::Config;
use crate::render::{
    BOLD, HASH, LANG, PATH, RESET, STRING, TITLE, Render, fmt_mutation_terse, fmt_mutation_verbose,
    fmt_outcome_at_from_state, fmt_status_from_state, mutation_hash,
};

pub struct ShowSingleMutation {
    pub state: State,
    pub before: String,
    pub after: String,
    pub lang: LanguageId,
}

impl ShowSingleMutation {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, hash: &str) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan");
        let mutation = find_mutation_by_hash(hash, mutations);
        let lang = mutation.mutant().lang();
        let file_path = bough_core::File::new(base, mutation.mutant().twig()).resolve();
        let file_src = std::fs::read_to_string(&file_path).expect("read source file");
        let (before, ctx_span) = mutation
            .mutant()
            .get_contextual_fragment(base, 3)
            .expect("context fragment");
        let mutated_src = mutation.apply_to_complete_src_string(&file_src);
        let original_len =
            mutation.mutant().span().end().byte() - mutation.mutant().span().start().byte();
        let subst_len = mutation.subst().len();
        let end_byte = if subst_len >= original_len {
            ctx_span.end().byte() + (subst_len - original_len)
        } else {
            ctx_span.end().byte() - (original_len - subst_len)
        };
        let after = &mutated_src[ctx_span.start().byte()..end_byte];
        let mutation_hash = mutation.hash().expect("hashing should not fail");
        let state = session
            .get_state()
            .get(&mutation_hash)
            .expect("state not found for mutation");
        Box::new(Self {
            state,
            before,
            after: after.to_string(),
            lang,
        })
    }

    fn lang_tag(&self) -> &'static str {
        match self.lang {
            LanguageId::Javascript => "javascript",
            LanguageId::Typescript => "typescript",
        }
    }
}

fn find_mutation_by_hash(hash: &str, mutations: Vec<bough_core::Mutation>) -> bough_core::Mutation {
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

impl Render for ShowSingleMutation {
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
