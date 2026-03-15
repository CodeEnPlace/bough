use bough_core::{LanguageId, Mutant, MutantKind, Mutation, PhaseOutcome, Point, Span, State, Status, Twig};
use bough_typed_hash::TypedHashable;
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
pub(crate) const STRING: &str = "\x1b[33m"; //yellow

pub(crate) const PATH: &str = "\x1b[34m"; // blue
pub(crate) const WORKSPACE: &str = "\x1b[36m"; // cyan
pub(crate) const MUTATION: &str = "\x1b[33m"; // yellow
pub(crate) const MUTANT: &str = "\x1b[35m"; // magenta
pub(crate) const LANG: &str = "\x1b[35m"; //purple

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
            Format::Terse => {
                let t = self.terse();
                debug_assert!(
                    !t.contains('\n'),
                    "terse() output must not contain newlines"
                );
                t
            }
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

impl Render for Status {
    fn markdown(&self) -> String {
        self.verbose()
    }

    fn terse(&self) -> String {
        match self {
            Status::Caught | Status::CaughtByTests(_) => format!("{CAUGHT}caught{RESET}"),
            Status::Timeout => format!("{TIMEOUT}timeout{RESET}"),
            Status::Missed => format!("{MISSED}missed{RESET}"),
        }
    }

    fn verbose(&self) -> String {
        match self {
            Status::Caught => format!("{CAUGHT}caught{RESET}"),
            Status::CaughtByTests(tests) => {
                let ids: Vec<_> = tests.iter().map(|t| facet_json::to_string(t).unwrap()).collect();
                format!("{CAUGHT}caught by [{}]{RESET}", ids.join(", "))
            }
            Status::Timeout => format!("{TIMEOUT}timeout{RESET}"),
            Status::Missed => format!("{MISSED}missed{RESET}"),
        }
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for Twig {
    fn markdown(&self) -> String {
        format!("{PATH}{}{RESET}", self.path().display())
    }

    fn terse(&self) -> String {
        self.markdown()
    }

    fn verbose(&self) -> String {
        self.markdown()
    }

    fn json(&self) -> String {
        format!(r#""{}""#, self.path().display())
    }
}

impl Render for Point {
    fn markdown(&self) -> String {
        format!("{}:{}", self.line() + 1, self.col() + 1)
    }

    fn terse(&self) -> String {
        self.markdown()
    }

    fn verbose(&self) -> String {
        self.markdown()
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for Span {
    fn markdown(&self) -> String {
        format!("{} - {}", self.start().markdown(), self.end().markdown())
    }

    fn terse(&self) -> String {
        self.markdown()
    }

    fn verbose(&self) -> String {
        self.markdown()
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for State {
    fn markdown(&self) -> String {
        let m = self.mutation();
        let hash = m.hash().expect("hash");
        let status = match self.status() {
            Some(s) => s.markdown(),
            None => format!("{NOT_RUN}not run{RESET}"),
        };
        format!(
            "{MUTATION}{hash}{RESET} {} {} {} {status} {} -> {STRING}{}{RESET}",
            m.mutant().lang().markdown(),
            m.mutant().twig().markdown(),
            m.mutant().span().markdown(),
            m.mutant().kind().markdown(),
            m.subst(),
        )
    }

    fn terse(&self) -> String {
        let m = self.mutation();
        let hash = m.hash().expect("hash");
        let status = match self.status() {
            Some(s) => s.terse(),
            None => format!("{NOT_RUN}not run{RESET}"),
        };
        format!(
            "{MUTATION}{hash}{RESET} {} {} {} {status} {} -> {STRING}{}{RESET}",
            m.mutant().lang().terse(),
            m.mutant().twig().terse(),
            m.mutant().span().terse(),
            m.mutant().kind().terse(),
            m.subst(),
        )
    }

    fn verbose(&self) -> String {
        let m = self.mutation();
        let hash = m.hash().expect("hash");
        let status = match self.status() {
            Some(s) => s.verbose(),
            None => format!("{NOT_RUN}not run{RESET}"),
        };
        format!(
            "{MUTATION}{hash}{RESET} {} {} {} {status} {} -> {STRING}{}{RESET}",
            m.mutant().lang().verbose(),
            m.mutant().twig().verbose(),
            m.mutant().span().verbose(),
            m.mutant().kind().verbose(),
            m.subst(),
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for Vec<State> {
    fn markdown(&self) -> String {
        let header = "| Hash | Lang | File | Span | Status | Kind | Subst |\n| --- | --- | --- | --- | --- | --- | --- |";
        let rows = self
            .iter()
            .map(|s| {
                let m = s.mutation();
                let hash = m.hash().expect("hash");
                let status = match s.status() {
                    Some(st) => st.markdown(),
                    None => format!("{NOT_RUN}not run{RESET}"),
                };
                format!(
                    "| `{hash}` | {} | {} | {} | {status} | {:?} | `{}` |",
                    m.mutant().lang().markdown(),
                    m.mutant().twig().path().display(),
                    m.mutant().span().markdown(),
                    m.mutant().kind(),
                    m.subst(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{header}\n{rows}")
    }

    fn terse(&self) -> String {
        self.iter()
            .map(|s| s.terse())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let list = self
            .iter()
            .map(|s| s.verbose())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{TITLE}States:{RESET}\n{list}")
    }

    fn json(&self) -> String {
        let items = self
            .iter()
            .map(|s| s.json())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    }
}

impl Render for Mutation {
    fn markdown(&self) -> String {
        let hash = self.hash().expect("hash");
        format!(
            "{MUTATION}{hash}{RESET} {} -> {STRING}{}{RESET}",
            self.mutant().markdown(),
            self.subst(),
        )
    }

    fn terse(&self) -> String {
        let hash = self.hash().expect("hash");
        format!(
            "{MUTATION}{hash}{RESET} {} -> {STRING}{}{RESET}",
            self.mutant().terse(),
            self.subst(),
        )
    }

    fn verbose(&self) -> String {
        let hash = self.hash().expect("hash");
        format!(
            "{MUTATION}{hash}{RESET} {} -> {STRING}{}{RESET}",
            self.mutant().verbose(),
            self.subst(),
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for Vec<Mutation> {
    fn markdown(&self) -> String {
        let header = "| Hash | Lang | File | Span | Kind | Subst |\n| --- | --- | --- | --- | --- | --- |";
        let rows = self
            .iter()
            .map(|m| {
                let hash = m.hash().expect("hash");
                format!(
                    "| `{hash}` | {} | {} | {} | {:?} | `{}` |",
                    m.mutant().lang().markdown(),
                    m.mutant().twig().path().display(),
                    m.mutant().span().markdown(),
                    m.mutant().kind(),
                    m.subst(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{header}\n{rows}")
    }

    fn terse(&self) -> String {
        self.iter()
            .map(|m| m.terse())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let list = self
            .iter()
            .map(|m| m.verbose())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{TITLE}Mutations:{RESET}\n{list}")
    }

    fn json(&self) -> String {
        let items = self
            .iter()
            .map(|m| m.json())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    }
}

impl Render for Mutant {
    fn markdown(&self) -> String {
        format!(
            "{} {} {} {}",
            self.lang().markdown(),
            self.twig().markdown(),
            self.span().markdown(),
            self.kind().markdown(),
        )
    }

    fn terse(&self) -> String {
        format!(
            "{} {} {} {}",
            self.lang().terse(),
            self.twig().terse(),
            self.span().terse(),
            self.kind().terse(),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{} {} {} {}",
            self.lang().verbose(),
            self.twig().verbose(),
            self.span().verbose(),
            self.kind().verbose(),
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for Vec<Mutant> {
    fn markdown(&self) -> String {
        let header = "| Lang | File | Span | Kind |\n| --- | --- | --- | --- |";
        let rows = self
            .iter()
            .map(|m| {
                format!(
                    "| {} | {} | {} | {} |",
                    m.lang().markdown(),
                    m.twig().path().display(),
                    m.span().markdown(),
                    format!("{:?}", m.kind()),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{header}\n{rows}")
    }

    fn terse(&self) -> String {
        self.iter()
            .map(|m| m.terse())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let list = self
            .iter()
            .map(|m| m.verbose())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{TITLE}Mutants:{RESET}\n{list}")
    }

    fn json(&self) -> String {
        let items = self
            .iter()
            .map(|m| m.json())
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    }
}

impl Render for MutantKind {
    fn markdown(&self) -> String {
        format!("{MUTANT}{self:?}{RESET}")
    }

    fn terse(&self) -> String {
        format!("{MUTANT}{self:?}{RESET}")
    }

    fn verbose(&self) -> String {
        format!("{MUTANT}{self:?}{RESET}")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

impl Render for LanguageId {
    fn markdown(&self) -> String {
        self.verbose()
    }

    fn terse(&self) -> String {
        format!(
            "{LANG}{}{RESET}",
            match self {
                LanguageId::Javascript => "js",
                LanguageId::Typescript => "ts",
            }
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{LANG}{}{RESET}",
            match self {
                LanguageId::Javascript => "JavaScript",
                LanguageId::Typescript => "TypeScript",
            }
        )
    }

    fn json(&self) -> String {
        match self {
            LanguageId::Javascript => r#""js""#.to_string(),
            LanguageId::Typescript => r#""ts""#.to_string(),
        }
    }
}

pub struct BenchmarkTimesInBase {
    pub init: Option<std::time::Duration>,
    pub reset: Option<std::time::Duration>,
    pub test: std::time::Duration,
}

impl Render for BenchmarkTimesInBase {
    fn markdown(&self) -> String {
        let mut out = format!("{TITLE}# Benchmark Times (Base){RESET}\n");
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
            init, reset, self.test.as_secs_f64(),
        )
    }
}

impl Render for PhaseOutcome {
    fn markdown(&self) -> String {
        let stdout = String::from_utf8_lossy(self.stdout());
        let stderr = String::from_utf8_lossy(self.stderr());
        let mut out = format!(
            "- Exit: {}\n- Duration: {:.2}s\n- Timed out: {}",
            self.exit_code(),
            self.duration().as_secs_f64(),
            self.timed_out(),
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
            "exit={} {:.2}s{}",
            self.exit_code(),
            self.duration().as_secs_f64(),
            if self.timed_out() { " TIMED_OUT" } else { "" },
        )
    }

    fn verbose(&self) -> String {
        let stdout = String::from_utf8_lossy(self.stdout());
        let stderr = String::from_utf8_lossy(self.stderr());
        let mut out = format!(
            "Exit: {}\nDuration: {:.2}s\nTimed out: {}",
            self.exit_code(),
            self.duration().as_secs_f64(),
            self.timed_out(),
        );
        if !stdout.is_empty() {
            out.push_str(&format!("\n\n{TITLE}stdout{RESET}\n{stdout}"));
        }
        if !stderr.is_empty() {
            out.push_str(&format!("\n\n{TITLE}stderr{RESET}\n{stderr}"));
        }
        out
    }

    fn json(&self) -> String {
        let stdout = String::from_utf8_lossy(self.stdout());
        let stderr = String::from_utf8_lossy(self.stderr());
        format!(
            r#"{{"exit_code":{},"duration_secs":{:.3},"timed_out":{},"stdout":"{}","stderr":"{}"}}"#,
            self.exit_code(),
            self.duration().as_secs_f64(),
            self.timed_out(),
            stdout.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
            stderr.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_markdown() {
        assert_eq!(Noop.markdown(), "");
    }

    #[test]
    fn noop_terse() {
        assert_eq!(Noop.terse(), "");
    }

    #[test]
    fn noop_verbose() {
        assert_eq!(Noop.verbose(), "");
    }

    #[test]
    fn noop_json() {
        assert_eq!(Noop.json(), "null");
    }

    #[test]
    fn status_markdown() {
        let plain = Status::Caught.markdown().replace(CAUGHT, "").replace(RESET, "");
        assert_eq!(plain, "caught");
    }

    #[test]
    fn status_terse() {
        let caught = Status::Caught.terse();
        assert!(!caught.contains('\n'));
        let plain = caught.replace(CAUGHT, "").replace(RESET, "");
        assert_eq!(plain, "caught");

        let plain = Status::Missed.terse().replace(MISSED, "").replace(RESET, "");
        assert_eq!(plain, "missed");

        let plain = Status::Timeout.terse().replace(TIMEOUT, "").replace(RESET, "");
        assert_eq!(plain, "timeout");
    }

    #[test]
    fn status_verbose() {
        let plain = Status::Missed.verbose().replace(MISSED, "").replace(RESET, "");
        assert_eq!(plain, "missed");
    }

    #[test]
    fn status_json() {
        assert_eq!(Status::Caught.json(), r#""Caught""#);
        assert_eq!(Status::Missed.json(), r#""Missed""#);
    }

    #[test]
    fn vec_state_markdown() {
        let states = vec![make_state()];
        let out = states.markdown();
        assert!(out.contains("| Hash | Lang | File | Span | Status | Kind | Subst |"));
    }

    #[test]
    fn vec_state_terse() {
        let states = vec![make_state(), make_state()];
        assert_eq!(states.terse().lines().count(), 2);
    }

    #[test]
    fn vec_state_verbose() {
        let states = vec![make_state()];
        let plain = states.verbose().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("States:\n"));
    }

    #[test]
    fn vec_state_json() {
        let states = vec![make_state()];
        let out = states.json();
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
    }

    fn make_state() -> State {
        State::new(make_mutation())
    }

    #[test]
    fn state_markdown() {
        let out = make_state().markdown();
        let plain = out.replace(NOT_RUN, "").replace(RESET, "");
        assert!(plain.contains("not run"));
        assert!(plain.contains("src/main.ts"));
    }

    #[test]
    fn state_terse() {
        let out = make_state().terse();
        assert!(!out.contains('\n'));
    }

    #[test]
    fn state_verbose() {
        let out = make_state().verbose();
        let plain = out.replace(NOT_RUN, "").replace(RESET, "");
        assert!(plain.contains("not run"));
    }

    #[test]
    fn state_json() {
        let out = make_state().json();
        assert!(out.starts_with('{'));
        assert!(out.contains("mutation"));
    }

    fn make_mutation() -> Mutation {
        bough_core::MutationIter::new(&make_mutant()).next().unwrap()
    }

    fn make_mutant() -> Mutant {
        Mutant::new(
            LanguageId::Typescript,
            Twig::new("src/main.ts".into()).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
        )
    }

    #[test]
    fn mutation_markdown() {
        let m = make_mutation();
        let out = m.markdown();
        assert!(out.contains(m.subst()));
        assert!(out.contains("src/main.ts"));
    }

    #[test]
    fn mutation_terse() {
        let out = make_mutation().terse();
        assert!(!out.contains('\n'));
    }

    #[test]
    fn mutation_verbose() {
        let m = make_mutation();
        let out = m.verbose();
        assert!(out.contains(m.subst()));
    }

    #[test]
    fn mutation_json() {
        let out = make_mutation().json();
        assert!(out.starts_with('{'));
        assert!(out.contains("subst"));
    }

    #[test]
    fn vec_mutation_markdown() {
        let mutations = vec![make_mutation()];
        let out = mutations.markdown();
        assert!(out.contains("| Hash | Lang | File | Span | Kind | Subst |"));
    }

    #[test]
    fn vec_mutation_terse() {
        let mutations = vec![make_mutation(), make_mutation()];
        assert_eq!(mutations.terse().lines().count(), 2);
    }

    #[test]
    fn vec_mutation_verbose() {
        let mutations = vec![make_mutation()];
        let plain = mutations.verbose().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("Mutations:\n"));
    }

    #[test]
    fn vec_mutation_json() {
        let mutations = vec![make_mutation()];
        let out = mutations.json();
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
    }

    #[test]
    fn mutant_markdown() {
        let plain = make_mutant()
            .markdown()
            .replace(PATH, "")
            .replace(MUTANT, "")
            .replace(LANG, "")
            .replace(RESET, "");
        assert_eq!(plain, "TypeScript src/main.ts 1:1 - 3:4 StatementBlock");
    }

    #[test]
    fn mutant_terse() {
        let out = make_mutant().terse();
        assert!(!out.contains('\n'));
    }

    #[test]
    fn mutant_verbose() {
        let plain = make_mutant()
            .verbose()
            .replace(PATH, "")
            .replace(MUTANT, "")
            .replace(LANG, "")
            .replace(RESET, "");
        assert_eq!(plain, "TypeScript src/main.ts 1:1 - 3:4 StatementBlock");
    }

    #[test]
    fn mutant_json() {
        let json = make_mutant().json();
        assert!(json.starts_with('{'));
        assert!(json.contains(r#""lang":"ts""#));
    }

    #[test]
    fn vec_mutant_markdown() {
        let mutants = vec![make_mutant()];
        let out = mutants.markdown();
        assert!(out.contains("| Lang | File | Span | Kind |"));
        assert!(out.contains("src/main.ts"));
    }

    #[test]
    fn vec_mutant_terse() {
        let mutants = vec![make_mutant(), make_mutant()];
        let lines: Vec<_> = mutants.terse().lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn vec_mutant_verbose() {
        let mutants = vec![make_mutant(), make_mutant()];
        let plain = mutants
            .verbose()
            .replace(TITLE, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Mutants:\n"));
        assert_eq!(plain.lines().count(), 3);
    }

    #[test]
    fn vec_mutant_json() {
        let mutants = vec![make_mutant()];
        let out = mutants.json();
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
    }

    #[test]
    fn twig_markdown() {
        let twig = Twig::new("src/main.ts".into()).unwrap();
        let plain = twig.markdown().replace(PATH, "").replace(RESET, "");
        assert_eq!(plain, "src/main.ts");
    }

    #[test]
    fn twig_terse() {
        let twig = Twig::new("src/main.ts".into()).unwrap();
        assert!(!twig.terse().contains('\n'));
    }

    #[test]
    fn twig_verbose() {
        let twig = Twig::new("src/main.ts".into()).unwrap();
        let plain = twig.verbose().replace(PATH, "").replace(RESET, "");
        assert_eq!(plain, "src/main.ts");
    }

    #[test]
    fn twig_json() {
        let twig = Twig::new("src/main.ts".into()).unwrap();
        assert_eq!(twig.json(), r#""src/main.ts""#);
    }

    #[test]
    fn point_markdown() {
        assert_eq!(Point::new(0, 0, 0).markdown(), "1:1");
        assert_eq!(Point::new(9, 4, 100).markdown(), "10:5");
    }

    #[test]
    fn point_terse() {
        assert_eq!(Point::new(0, 0, 0).terse(), "1:1");
    }

    #[test]
    fn point_verbose() {
        assert_eq!(Point::new(0, 0, 0).verbose(), "1:1");
    }

    #[test]
    fn point_json() {
        assert_eq!(Point::new(0, 5, 10).json(), r#"{"line":0,"col":5,"byte":10}"#);
    }

    #[test]
    fn span_markdown() {
        let span = Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20));
        assert_eq!(span.markdown(), "1:1 - 3:4");
    }

    #[test]
    fn span_terse() {
        let span = Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20));
        assert_eq!(span.terse(), "1:1 - 3:4");
    }

    #[test]
    fn span_verbose() {
        let span = Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20));
        assert_eq!(span.verbose(), "1:1 - 3:4");
    }

    #[test]
    fn span_json() {
        let span = Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20));
        assert_eq!(
            span.json(),
            r#"{"start":{"line":0,"col":0,"byte":0},"end":{"line":2,"col":3,"byte":20}}"#
        );
    }

    #[test]
    fn mutant_kind_markdown() {
        use bough_core::MutantKind;
        let plain = MutantKind::StatementBlock.markdown().replace(MUTANT, "").replace(RESET, "");
        assert_eq!(plain, "StatementBlock");
    }

    #[test]
    fn mutant_kind_terse() {
        use bough_core::MutantKind;
        let out = MutantKind::Condition.terse();
        assert!(!out.contains('\n'));
        let plain = out.replace(MUTANT, "").replace(RESET, "");
        assert_eq!(plain, "Condition");
    }

    #[test]
    fn mutant_kind_verbose() {
        use bough_core::MutantKind;
        let plain = MutantKind::DictDecl.verbose().replace(MUTANT, "").replace(RESET, "");
        assert_eq!(plain, "DictDecl");
    }

    #[test]
    fn mutant_kind_json() {
        use bough_core::MutantKind;
        assert_eq!(MutantKind::StatementBlock.json(), r#""StatementBlock""#);
    }

    #[test]
    fn language_id_markdown() {
        let js = LanguageId::Javascript.markdown().replace(LANG, "").replace(RESET, "");
        let ts = LanguageId::Typescript.markdown().replace(LANG, "").replace(RESET, "");
        assert_eq!(js, "JavaScript");
        assert_eq!(ts, "TypeScript");
    }

    #[test]
    fn language_id_terse() {
        let js = LanguageId::Javascript.terse().replace(LANG, "").replace(RESET, "");
        let ts = LanguageId::Typescript.terse().replace(LANG, "").replace(RESET, "");
        assert_eq!(js, "js");
        assert_eq!(ts, "ts");
    }

    #[test]
    fn language_id_verbose() {
        let js = LanguageId::Javascript.verbose().replace(LANG, "").replace(RESET, "");
        let ts = LanguageId::Typescript.verbose().replace(LANG, "").replace(RESET, "");
        assert_eq!(js, "JavaScript");
        assert_eq!(ts, "TypeScript");
    }

    #[test]
    fn language_id_json() {
        assert_eq!(LanguageId::Javascript.json(), r#""js""#);
        assert_eq!(LanguageId::Typescript.json(), r#""ts""#);
    }

    fn make_benchmark() -> BenchmarkTimesInBase {
        BenchmarkTimesInBase {
            init: Some(std::time::Duration::from_millis(1500)),
            reset: None,
            test: std::time::Duration::from_millis(2000),
        }
    }

    #[test]
    fn benchmark_markdown() {
        let plain = make_benchmark().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.contains("Init: 1.50s"));
        assert!(!plain.contains("Reset"));
        assert!(plain.contains("Test: 2.00s"));
    }

    #[test]
    fn benchmark_terse() {
        let out = make_benchmark().terse();
        assert!(!out.contains('\n'));
        assert_eq!(out, "init=1.50s test=2.00s");
    }

    #[test]
    fn benchmark_verbose() {
        let plain = make_benchmark().verbose().replace(TITLE, "").replace(RESET, "");
        assert!(plain.contains("Init:  1.50s"));
        assert!(plain.contains("Test:  2.00s"));
    }

    #[test]
    fn benchmark_json() {
        let out = make_benchmark().json();
        assert!(out.contains(r#""reset_secs":null"#));
        assert!(out.contains(r#""test_secs":2.000"#));
    }

    fn make_phase_outcome() -> PhaseOutcome {
        PhaseOutcome::new_for_test(
            0,
            std::time::Duration::from_millis(1234),
            false,
            b"hello\n".to_vec(),
            vec![],
        )
    }

    #[test]
    fn phase_outcome_markdown() {
        let out = make_phase_outcome().markdown();
        assert!(out.contains("Exit: 0"));
        assert!(out.contains("```\nhello\n\n```"));
    }

    #[test]
    fn phase_outcome_terse() {
        let out = make_phase_outcome().terse();
        assert!(!out.contains('\n'));
        assert!(out.contains("exit=0"));
        assert!(out.contains("1.23s"));
    }

    #[test]
    fn phase_outcome_verbose() {
        let out = make_phase_outcome().verbose();
        assert!(out.contains("Exit: 0"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn phase_outcome_json() {
        let out = make_phase_outcome().json();
        assert!(out.contains(r#""exit_code":0"#));
        assert!(out.contains(r#""timed_out":false"#));
    }
}
