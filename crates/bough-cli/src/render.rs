use bough_core::config::Config;
use bough_core::{BinaryOpKind, Language, Mutation, MutationKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Terse,
    Verbose,
    Markdown,
    Json,
    Toml,
    Yaml,
}

impl Default for Style {
    fn default() -> Self {
        Style::Terse
    }
}

pub trait Render {
    fn render_value(&self) -> serde_value::Value;
    fn render_terse(&self) -> String;
    fn render_verbose(&self) -> String;
    fn render_markdown(&self, depth: u8) -> String;

    fn render(&self, style: &Style, no_color: bool, depth: u8) {
        let output = match style {
            Style::Terse => self.render_terse(),
            Style::Verbose => self.render_verbose(),
            Style::Markdown => self.render_markdown(depth),
            Style::Json => {
                serde_json::to_string(&self.render_value()).expect("failed to serialize")
            }
            Style::Toml => {
                toml::to_string_pretty(&self.render_value()).expect("failed to serialize")
            }
            Style::Yaml => {
                serde_yaml::to_string(&self.render_value()).expect("failed to serialize")
            }
        };
        let output = if no_color { strip_ansi(&output) } else { output };
        print!("{output}");
    }
}

pub fn color(code: &str, text: &str) -> String {
    format!("{code}{text}\x1b[0m")
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

impl Render for BinaryOpKind {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        self.label().to_string()
    }

    fn render_verbose(&self) -> String {
        self.label().to_string()
    }

    fn render_markdown(&self, _depth: u8) -> String {
        self.label().to_string()
    }
}

impl Render for MutationKind {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        match self {
            MutationKind::StatementBlock => "empty statement block".to_string(),
            MutationKind::BinaryOp(op) => format!("binary operator {}", op.render_terse()),
            MutationKind::Condition => "condition".to_string(),
        }
    }

    fn render_verbose(&self) -> String {
        match self {
            MutationKind::StatementBlock => "empty statement block".to_string(),
            MutationKind::BinaryOp(op) => format!("binary operator {}", op.render_verbose()),
            MutationKind::Condition => "condition".to_string(),
        }
    }

    fn render_markdown(&self, depth: u8) -> String {
        match self {
            MutationKind::StatementBlock => "empty statement block".to_string(),
            MutationKind::Condition => "condition".to_string(),
            MutationKind::BinaryOp(op) => {
                format!("binary operator {}", op.render_markdown(depth))
            }
        }
    }
}

impl<L: Language> Render for Mutation<L>
where
    L::Kind: Clone + Into<MutationKind> + Serialize,
{
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        let kind: MutationKind = self.mutant.kind.clone().into();
        let path = self.mutant.src.path.display();
        let span = &self.mutant.span;
        let loc = format!(
            "{}:{}-{}:{}",
            span.start.line + 1,
            span.start.char + 1,
            span.end.line + 1,
            span.end.char + 1,
        );
        format!(
            "{} at {} → {}\n",
            color("\x1b[1m", &kind.render_terse()),
            color("\x1b[36m", &format!("{path}:{loc}")),
            color("\x1b[32m", &self.replacement),
        )
    }

    fn render_verbose(&self) -> String {
        let kind: MutationKind = self.mutant.kind.clone().into();
        let path = self.mutant.src.path.display();
        let span = &self.mutant.span;
        let loc = format!(
            "{}:{}-{}:{}",
            span.start.line + 1,
            span.start.char + 1,
            span.end.line + 1,
            span.end.char + 1,
        );
        format!(
            "{} at {}\nhash: {}\nreplacement: {}\n",
            color("\x1b[1m", &kind.render_verbose()),
            color("\x1b[36m", &format!("{path}:{loc}")),
            self.mutant.src.hash,
            color("\x1b[32m", &self.replacement),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let kind: MutationKind = self.mutant.kind.clone().into();
        let path = self.mutant.src.path.display();
        let span = &self.mutant.span;
        let loc = format!(
            "{}:{}-{}:{}",
            span.start.line + 1,
            span.start.char + 1,
            span.end.line + 1,
            span.end.char + 1,
        );
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let tag = L::code_tag();
        format!(
            "{heading} Mutation\n\n\
             **Kind:** {}\n\n\
             **File:** `{path}`\n\n\
             **Location:** {loc}\n\n\
             **Replacement:**\n```{tag}\n{}\n```\n",
            kind.render_markdown(depth + 1),
            self.replacement,
        )
    }
}

impl Render for Config {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!("{self:#?}")
    }

    fn render_verbose(&self) -> String {
        format!("{self:#?}")
    }

    fn render_markdown(&self, _depth: u8) -> String {
        format!("```\n{self:#?}\n```")
    }
}
