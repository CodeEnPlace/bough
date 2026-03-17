use std::fs;
use std::path::{Path, PathBuf};

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

            items.push((idx, TocEntry { title, href, children }));
        } else if path.extension().is_some_and(|e| e == "md") {
            let stem = path.file_stem().unwrap_or_default();
            if stem == "index" {
                continue;
            }

            let content = fs::read_to_string(&path).expect("failed to read md file");
            let (fm, _) = parse_md(&content);

            let title = fm.title.unwrap_or_else(|| stem.to_string_lossy().into_owned());
            let idx = fm.idx.unwrap_or(i64::MAX);

            let rel = path.strip_prefix(docs_dir).unwrap().with_extension("");
            let href = format!("/{}/", rel.to_string_lossy());

            items.push((idx, TocEntry { title, href, children: Vec::new() }));
        }
    }

    items.sort_by_key(|(idx, _)| *idx);
    items.into_iter().map(|(_, entry)| entry).collect()
}

fn render_toc_entries(entries: &[TocEntry], current_slug: &str) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let mut out = String::from("<ol>");
    for entry in entries {
        let entry_slug = entry.href.trim_matches('/');
        let id = entry_slug.replace('/', "-");
        let active = if entry_slug == current_slug { " class=\"active\"" } else { "" };
        out.push_str(&format!("<li id=\"toc-{id}\"{active}><a href=\"{}\">{}</a>", entry.href, entry.title));
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
        <link href="https://codeenplace.dev/light.color.css" rel="stylesheet" media="(prefers-color-scheme:light)" />
        <link href="https://codeenplace.dev/dark.color.css" rel="stylesheet" media="(prefers-color-scheme:dark)" />
                        
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <title>{title}</title>
    </head>

    <body>
        <nav id="toc">{toc}</nav>
        <main>
            <h1>{title}</h1>
            {body}
        </main>
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

pub fn build() {
    let docs_dir = Path::new(DOCS_DIR);
    let out_dir = Path::new(OUT_DIR);

    if out_dir.exists() {
        fs::remove_dir_all(out_dir).expect("failed to clean output dir");
    }
    fs::create_dir_all(out_dir).expect("failed to create output dir");

    let md_files = collect_md_files(docs_dir);
    if md_files.is_empty() {
        eprintln!("no markdown files found in {}", docs_dir.display());
        return;
    }

    let toc = build_toc(docs_dir, docs_dir);

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

            let rel = path.strip_prefix(docs_dir).unwrap();
            let slug = if rel.file_stem().is_some_and(|s| s == "index") {
                rel.parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                rel.with_extension("").to_string_lossy().into_owned()
            };

            let body_html = markdown_to_html(&body, &opts);

            Page { title, slug, body_html }
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
