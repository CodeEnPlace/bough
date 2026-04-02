use crate::language::LanguageDriver;
use crate::mutant::Mutant;
use bough_typed_hash::TypedHashable;
use tracing::trace;

pub struct MutationIter<'a> {
    mutant: &'a Mutant,
    subs: std::vec::IntoIter<String>,
}

impl<'a> MutationIter<'a> {
    pub fn new(mutant: &'a Mutant, driver: &dyn LanguageDriver) -> Self {
        let subs = driver.substitutions(mutant.kind());
        trace!(lang = ?mutant.lang(), kind = ?mutant.kind(), substitutions = subs.len(), "creating mutation iter");
        Self {
            mutant,
            subs: subs.into_iter(),
        }
    }

    pub fn mutant(&self) -> &Mutant {
        self.mutant
    }
}

impl<'a> Iterator for MutationIter<'a> {
    type Item = Mutation;

    fn next(&mut self) -> Option<Self::Item> {
        let subst = self.subs.next()?;
        Some(Mutation {
            mutant: self.mutant.clone(),
            subst,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, facet::Facet, TypedHashable)]
pub struct Mutation {
    pub mutant: Mutant,
    pub subst: String,
}

impl Mutation {
    pub fn mutant(&self) -> &Mutant {
        &self.mutant
    }

    pub fn subst(&self) -> &str {
        &self.subst
    }

    pub fn apply_to_complete_src_string(&self, src: &str) -> String {
        let start = self.mutant.span().start().byte();
        let end = self.mutant.span().end().byte();
        format!("{}{}{}", &src[..start], self.subst, &src[end..])
    }
}

