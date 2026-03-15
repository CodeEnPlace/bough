use std::ops::DerefMut;

use bough_core::{LanguageId, Session, State};
use bough_typed_hash::{TypedHashable, UnvalidatedHash};

use crate::config::Config;
use crate::render::{BOLD, TITLE, RESET, Render};

// TODO: decompose into smaller types (e.g. CodeDiff) and delegate Render to them
pub struct ShowSingleMutation {
    pub state: State,
    pub before: String,
    pub after: String,
    pub lang: LanguageId,
}

impl ShowSingleMutation {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, hash: &str) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let mutation = session.resolve_mutation(UnvalidatedHash::new(hash.to_string())).expect("resolve mutation");
        let base = session.base();
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

impl Render for ShowSingleMutation {
    fn markdown(&self) -> String {
        let tag = self.lang_tag();
        format!(
            "{TITLE}# Mutation{RESET}\n\n\
            {}\n\n\
            {TITLE}## Before{RESET}\n\n```{tag}\n{}\n```\n\n\
            {TITLE}## After{RESET}\n\n```{tag}\n{}\n```",
            self.state.markdown(),
            self.before,
            self.after,
        )
    }

    fn terse(&self) -> String {
        self.state.terse()
    }

    fn verbose(&self) -> String {
        format!(
            "{}\n\n{BOLD}Before:{RESET}\n{}\n\n{BOLD}After:{RESET}\n{}",
            self.state.verbose(),
            self.before,
            self.after,
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(&self.state).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_core::*;

    fn fixture() -> ShowSingleMutation {
        let mutant = Mutant::new(
            LanguageId::Typescript,
            Twig::new("src/main.ts".into()).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
        );
        let mutation = MutationIter::new(&mutant).next().unwrap();
        ShowSingleMutation {
            state: State::new(mutation),
            before: "const x = 1;".to_string(),
            after: "const x = 2;".to_string(),
            lang: LanguageId::Typescript,
        }
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown()
            .replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# Mutation"));
        assert!(plain.contains("```typescript\nconst x = 1;\n```"));
        assert!(plain.contains("```typescript\nconst x = 2;\n```"));
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(!out.contains('\n'));
    }

    #[test]
    fn verbose() {
        let plain = fixture().verbose()
            .replace(BOLD, "").replace(RESET, "");
        assert!(plain.contains("Before:\nconst x = 1;"));
        assert!(plain.contains("After:\nconst x = 2;"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('{'));
        assert!(out.contains("mutation"));
    }
}


