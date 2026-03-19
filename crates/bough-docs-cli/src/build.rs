use std::fs;
use std::path::{Path, PathBuf};

use arborium::Highlighter;
use comrak::{Options, markdown_to_html};
use serde::Deserialize;

const DOCS_DIR: &str = "docs";
const OUT_DIR: &str = "target/bough-docs-site";

#[derive(Deserialize, Default)]
struct Frontmatter {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    idx: Option<i64>,
}

#[derive(Debug)]
struct TocEntry {
    title: String,
    href: String,
    children: Vec<TocEntry>,
}

struct Page {
    title: String,
    slug: String,
    body_html: String,
}

fn parse_md(content: &str) -> (Frontmatter, String) {
    match markdown_frontmatter::parse::<Frontmatter>(content) {
        Ok((fm, body)) => (fm, body.to_owned()),
        Err(_) => (Frontmatter::default(), content.to_owned()),
    }
}

fn build_toc(dir: &Path, docs_dir: &Path) -> Vec<TocEntry> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut items: Vec<(i64, TocEntry)> = Vec::new();

    let mut dir_entries: Vec<_> = entries.flatten().collect();
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in dir_entries {
        let path = entry.path();

        if path.is_dir() {
            let index_path = path.join("index.md");
            let (fm, _) = if index_path.exists() {
                let content = fs::read_to_string(&index_path).expect("failed to read index.md");
                parse_md(&content)
            } else {
                (Frontmatter::default(), String::new())
            };

            let title = fm.title.unwrap_or_else(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
            let idx = fm.idx.unwrap_or(i64::MAX);

            let rel = path.strip_prefix(docs_dir).unwrap();
            let href = format!("/{}/", rel.to_string_lossy());
            let children = build_toc(&path, docs_dir);

            items.push((
                idx,
                TocEntry {
                    title,
                    href,
                    children,
                },
            ));
        } else if path.extension().is_some_and(|e| e == "md") {
            let stem = path.file_stem().unwrap_or_default();
            if stem == "index" {
                continue;
            }

            let content = fs::read_to_string(&path).expect("failed to read md file");
            let (fm, _) = parse_md(&content);

            let title = fm
                .title
                .unwrap_or_else(|| stem.to_string_lossy().into_owned());
            let idx = fm.idx.unwrap_or(i64::MAX);

            let rel = path.strip_prefix(docs_dir).unwrap().with_extension("");
            let href = format!("/{}/", rel.to_string_lossy());

            items.push((
                idx,
                TocEntry {
                    title,
                    href,
                    children: Vec::new(),
                },
            ));
        }
    }

    items.sort_by_key(|(idx, _)| *idx);
    items.into_iter().map(|(_, entry)| entry).collect()
}

fn highlight_code_blocks(html: &str, hl: &mut Highlighter) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(pre_start) = rest.find("<pre><code class=\"language-") {
        out.push_str(&rest[..pre_start]);
        rest = &rest[pre_start..];

        let Some(lang_start) = rest.find("language-") else {
            out.push_str(rest);
            return out;
        };
        let after_prefix = &rest[lang_start + 9..];
        let Some(lang_end) = after_prefix.find('"') else {
            out.push_str(rest);
            return out;
        };
        let lang = &after_prefix[..lang_end];

        let Some(code_start) = rest
            .find('>')
            .and_then(|i| rest[i + 1..].find('>').map(|j| i + 1 + j + 1))
        else {
            out.push_str(rest);
            return out;
        };
        let code_body = &rest[code_start..];
        let Some(code_end) = code_body.find("</code></pre>") else {
            out.push_str(rest);
            return out;
        };

        let raw_code = &code_body[..code_end];
        let decoded = raw_code
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'");

        match hl.highlight(lang, &decoded) {
            Ok(highlighted) => {
                out.push_str("<pre><code>");
                out.push_str(&highlighted);
                out.push_str("</code></pre>");
            }
            Err(_) => {
                out.push_str(&rest[..code_start + code_end + 13]);
            }
        }

        rest = &code_body[code_end + 13..];
    }

    out.push_str(rest);
    out
}

fn render_toc_entries(entries: &[TocEntry], current_slug: &str) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut out = String::from("<ol>");
    for entry in entries {
        let entry_slug = entry.href.trim_matches('/');
        let id = entry_slug.replace('/', "-");
        let active = if entry_slug == current_slug {
            " class=\"active\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<li id=\"toc-{id}\"{active}><a href=\"{}\">{}</a>",
            entry.href, entry.title
        ));
        if !entry.children.is_empty() {
            out.push_str(&render_toc_entries(&entry.children, current_slug));
        }
        out.push_str("</li>");
    }
    out.push_str("</ol>");
    out
}

fn render_page(page: &Page, toc_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <link href="https://codeenplace.dev/fonts.css" rel="stylesheet" />
        <link href="https://codeenplace.dev/base.css" rel="stylesheet" />
        <link href="/layout.css" rel="stylesheet" />
        <link href="https://codeenplace.dev/light.color.css" rel="stylesheet" media="(prefers-color-scheme:light)" />
        <link href="https://codeenplace.dev/dark.color.css" rel="stylesheet" media="(prefers-color-scheme:dark)" />
                        
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <title>{title}</title>
        <style>
            pre code {{ display: block; padding: 1rem; overflow-x: auto; border-radius: 4px; background: var(--col-bright-bg); color: var(--col-fg); }}
            a-k, a-kc, a-kd, a-ke, a-kf, a-ki, a-km, a-ko, a-kp, a-kr, a-kt, a-ky {{ color: var(--col-red); }}
            a-f, a-fb, a-fc, a-fd, a-fm {{ color: var(--col-blue); }}
            a-s, a-sc, a-se, a-sp, a-ss, a-st {{ color: var(--col-green); }}
            a-co, a-cn {{ color: var(--col-yellow); }}
            a-c, a-cb, a-cd, a-ch, a-cs {{ color: var(--col-bright-black); font-style: italic; }}
            a-o {{ color: var(--col-orange); }}
            a-t, a-tb, a-td, a-te, a-tf, a-tg, a-tl, a-tq, a-tr, a-tt, a-tu, a-tx {{ color: var(--col-cyan); }}
            a-at {{ color: var(--col-purple); }}
            a-v, a-vb, a-vm, a-vp {{ color: var(--col-fg); }}
            a-n {{ color: var(--col-yellow); }}
            a-l, a-m {{ color: var(--col-yellow); }}
            a-p, a-pb, a-pd, a-pp, a-pr, a-ps {{ color: var(--col-bright-fg); }}
            a-in, a-ex {{ color: var(--col-bright-red); }}
            a-em {{ font-weight: bold; }}
            a-dr, a-rp, a-rx {{ color: var(--col-orange); }}
            pre code del {{ text-decoration: none; border-bottom: 2px solid var(--col-red); }}
            pre code ins {{ text-decoration: none; border-bottom: 2px solid var(--col-green); }}
        </style>
    </head>

    <body>
        <nav id="toc">{toc}</nav>
        <main>
            <h1>{title}</h1>
            {body}
        </main>
        <script>
            (function() {{
                var prefetched = {{}};
                function prefetch(url) {{
                    if (prefetched[url] || url === location.pathname) return;
                    prefetched[url] = true;
                    var l = document.createElement("link");
                    l.rel = "prefetch";
                    l.href = url;
                    document.head.appendChild(l);
                }}
                document.addEventListener("pointerenter", function(e) {{
                    var a = e.target.closest("a[href^='/']");
                    if (a) prefetch(a.getAttribute("href"));
                }}, true);
                document.addEventListener("focusin", function(e) {{
                    var a = e.target.closest("a[href^='/']");
                    if (a) prefetch(a.getAttribute("href"));
                }});
            }})();
        </script>
    </body>
</html>"#,
        title = page.title,
        body = page.body_html,
        toc = toc_html,
    )
}

fn collect_md_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_md_files(&path));
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
    files.sort();
    files
}

const CORPUS_DIR: &str = "corpus";

fn highlight_with_diff(
    hl: &mut Highlighter,
    lang: &str,
    source: &str,
    diff_start: usize,
    diff_end: usize,
    diff_tag: &str,
) -> String {
    use arborium::advanced::html_escape;

    let spans = hl.highlight_spans(lang, source).unwrap_or_default();
    let source = source.trim_end_matches('\n');

    let mut sorted = spans;
    sorted.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));

    // (pos, is_start, tag) — None tag means diff marker
    let mut events: Vec<(usize, bool, Option<String>)> = Vec::new();

    for span in &sorted {
        if let Some(tag) = arborium_theme::tag_for_capture(&span.capture) {
            let t = format!("a-{tag}");
            events.push((span.start as usize, true, Some(t.clone())));
            events.push((span.end as usize, false, Some(t)));
        }
    }

    events.push((diff_start, true, None));
    events.push((diff_end, false, None));

    events.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut out = String::from("<pre><code>");
    let mut pos = 0;
    let mut syntax_stack: Vec<String> = Vec::new();

    for (ep, is_start, tag) in &events {
        let ep = (*ep).min(source.len());
        if ep > pos {
            out.push_str(&html_escape(&source[pos..ep]));
            pos = ep;
        }
        match tag {
            None => {
                if let Some(t) = syntax_stack.last() {
                    out.push_str(&format!("</{t}>"));
                }
                if *is_start {
                    out.push_str(&format!("<{diff_tag}>"));
                } else {
                    out.push_str(&format!("</{diff_tag}>"));
                }
                if let Some(t) = syntax_stack.last() {
                    out.push_str(&format!("<{t}>"));
                }
            }
            Some(t) => {
                if *is_start {
                    out.push_str(&format!("<{t}>"));
                    syntax_stack.push(t.clone());
                } else {
                    out.push_str(&format!("</{t}>"));
                    if let Some(idx) = syntax_stack.iter().rposition(|s| s == t) {
                        syntax_stack.remove(idx);
                    }
                }
            }
        }
    }

    if pos < source.len() {
        out.push_str(&html_escape(&source[pos..]));
    }

    out.push_str("</code></pre>");
    out
}

#[derive(Deserialize)]
struct CorpusMutation {
    mutant: CorpusMutant,
    subst: String,
}

#[derive(Deserialize)]
struct CorpusMutant {
    kind: serde_json::Value,
    subst_span: CorpusSpan,
}

#[derive(Deserialize)]
struct CorpusSpan {
    start: CorpusPoint,
    end: CorpusPoint,
}

#[derive(Deserialize)]
struct CorpusPoint {
    byte: usize,
}

struct CorpusExample {
    source: String,
    span_start: usize,
    span_end: usize,
    substitutions: Vec<String>,
    ratio: f64,
}

fn kind_key(kind: &serde_json::Value) -> String {
    match kind {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(m) => {
            let (k, v) = m.iter().next().unwrap();
            format!("{k}:{}", v.as_str().unwrap_or(""))
        }
        _ => kind.to_string(),
    }
}

fn make_mutation_reference(lang: &bough_core::LanguageId, hl: &mut Highlighter) -> String {
    use std::collections::BTreeMap;
    use std::fmt::Write;

    let corpus_dir = Path::new(CORPUS_DIR).join(lang.corpus_dir_name());
    let ext = lang.file_extension();

    let mut best: BTreeMap<String, CorpusExample> = BTreeMap::new();

    if let Ok(case_dirs) = fs::read_dir(&corpus_dir) {
        for case_entry in case_dirs.flatten() {
            let case_path = case_entry.path();
            if !case_path.is_dir() {
                continue;
            }

            let base_path = case_path.join(format!("base.{ext}"));
            let Ok(source) = fs::read_to_string(&base_path) else {
                continue;
            };
            let file_len = source.len();
            if file_len == 0 {
                continue;
            }

            let mut mutations_by_kind: BTreeMap<String, (serde_json::Value, usize, usize, Vec<String>)> = BTreeMap::new();

            let Ok(entries) = fs::read_dir(&case_path) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.to_string_lossy().ends_with(".mutation.json") {
                    continue;
                }
                let Ok(json) = fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(mutation) = serde_json::from_str::<CorpusMutation>(&json) else {
                    continue;
                };

                let key = kind_key(&mutation.mutant.kind);
                let entry = mutations_by_kind.entry(key).or_insert_with(|| {
                    (
                        mutation.mutant.kind.clone(),
                        mutation.mutant.subst_span.start.byte,
                        mutation.mutant.subst_span.end.byte,
                        Vec::new(),
                    )
                });
                entry.3.push(mutation.subst);
            }

            for (key, (_, span_start, span_end, subs)) in mutations_by_kind {
                let span_len = span_end.saturating_sub(span_start);
                let ratio = span_len as f64 / file_len as f64;

                if best.get(&key).is_none_or(|prev| ratio > prev.ratio) {
                    best.insert(key, CorpusExample {
                        source: source.clone(),
                        span_start,
                        span_end,
                        substitutions: subs,
                        ratio,
                    });
                }
            }
        }
    }

    let mut out = format!("+++\ntitle = \"{}\"\n+++\n\n", lang.display_name());

    for kind in bough_core::MutantKind::all_variants() {
        let subs = lang.substitutions(&kind);
        if subs.is_empty() {
            continue;
        }

        let _ = writeln!(out, "## {}\n", kind.heading());

        let key = kind_to_key(&kind);
        if let Some(example) = best.get(&key) {
            let before_html = highlight_with_diff(
                hl, ext, &example.source,
                example.span_start, example.span_end, "del",
            );
            let _ = writeln!(out, "### Before\n\n{before_html}\n");
            let _ = writeln!(out, "### After\n");
            for sub in &example.substitutions {
                let mutated = format!(
                    "{}{}{}",
                    &example.source[..example.span_start],
                    sub,
                    &example.source[example.span_end..],
                );
                let ins_start = example.span_start;
                let ins_end = example.span_start + sub.len();
                let after_html = highlight_with_diff(
                    hl, ext, &mutated,
                    ins_start, ins_end, "ins",
                );
                let _ = writeln!(out, "{after_html}\n");
            }
        }
    }

    out
}

fn kind_to_key(kind: &bough_core::MutantKind) -> String {
    use bough_core::mutant::*;
    match kind {
        MutantKind::StatementBlock => "StatementBlock".into(),
        MutantKind::Condition => "Condition".into(),
        MutantKind::BinaryOp(k) => format!("BinaryOp:{k:?}"),
        MutantKind::Assign(k) => format!("Assign:{k:?}"),
        MutantKind::ArrayDecl(k) => format!("ArrayDecl:{k:?}"),
        MutantKind::Literal(k) => format!("Literal:{k:?}"),
        MutantKind::DictDecl => "DictDecl".into(),
        MutantKind::TupleDecl => "TupleDecl".into(),
        MutantKind::Assert => "Assert".into(),
        MutantKind::UnaryNot => "UnaryNot".into(),
        MutantKind::OptionalChain(k) => format!("OptionalChain:{k:?}"),
        MutantKind::SwitchCase => "SwitchCase".into(),
    }
}

pub fn build() {
    let docs_dir = Path::new(DOCS_DIR);
    let out_dir = Path::new(OUT_DIR);

    if out_dir.exists() {
        fs::remove_dir_all(out_dir).expect("failed to clean output dir");
    }
    fs::create_dir_all(out_dir).expect("failed to create output dir");

    fs::write(out_dir.join("layout.css"), include_str!("layout.css")).expect("failed to write layout.css");

    let generated_dir = Path::new("target/bough-docs-generated");
    if generated_dir.exists() {
        fs::remove_dir_all(generated_dir).expect("failed to clean generated dir");
    }
    let reference_dir = generated_dir.join("config");
    fs::create_dir_all(&reference_dir).expect("failed to create generated config dir");
    let reference_md = format!(
        "+++\ntitle = \"Configuration Reference\"\n+++\n\n{}",
        crate::facet_reference::make_facet_reference::<bough_cli::config::Config>()
    );
    fs::write(reference_dir.join("reference.md"), reference_md)
        .expect("failed to write config reference");

    let mut hl = Highlighter::new();

    let mutations_dir = generated_dir.join("mutations");
    fs::create_dir_all(&mutations_dir).expect("failed to create generated mutations dir");
    for lang in bough_core::LanguageId::ALL {
        let md = make_mutation_reference(lang, &mut hl);
        fs::write(mutations_dir.join(format!("{}.md", lang.slug())), md)
            .expect("failed to write mutation reference");
    }

    let mut md_files = collect_md_files(docs_dir);
    md_files.extend(collect_md_files(generated_dir));
    if md_files.is_empty() {
        eprintln!("no markdown files found in {}", docs_dir.display());
        return;
    }

    let mut toc = build_toc(docs_dir, docs_dir);
    for gen_entry in build_toc(generated_dir, generated_dir) {
        if let Some(existing) = toc.iter_mut().find(|e| e.href == gen_entry.href) {
            existing.children.extend(gen_entry.children);
        } else {
            toc.push(gen_entry);
        }
    }

    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.autolink = true;
    opts.render.unsafe_ = true;

    let pages: Vec<Page> = md_files
        .iter()
        .map(|path| {
            let content = fs::read_to_string(path).expect("failed to read md file");
            let (fm, body) = parse_md(&content);

            let title = fm.title.unwrap_or_else(|| {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });

            let rel = path
                .strip_prefix(docs_dir)
                .or_else(|_| path.strip_prefix(generated_dir))
                .unwrap();
            let slug = if rel.file_stem().is_some_and(|s| s == "index") {
                rel.parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                rel.with_extension("").to_string_lossy().into_owned()
            };

            let body_html = markdown_to_html(&body, &opts);
            let body_html = highlight_code_blocks(&body_html, &mut hl);

            Page {
                title,
                slug,
                body_html,
            }
        })
        .collect();

    for page in &pages {
        let toc_html = render_toc_entries(&toc, &page.slug);

        let out_path = if page.slug.is_empty() {
            out_dir.join("index.html")
        } else {
            let dir = out_dir.join(&page.slug);
            fs::create_dir_all(&dir).expect("failed to create page dir");
            dir.join("index.html")
        };

        let html = render_page(page, &toc_html);
        fs::write(&out_path, html).expect("failed to write html");
        eprintln!("  wrote {}", out_path.display());
    }

    eprintln!("built {} pages into {}", pages.len(), out_dir.display());
}
