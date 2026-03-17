use std::fs;
use std::path::{Path, PathBuf};

use comrak::{markdown_to_html, Options};

const DOCS_DIR: &str = "docs";
const OUT_DIR: &str = "target/bough-docs-site";

struct Page {
    title: String,
    slug: String,
    body_html: String,
}

fn parse_frontmatter(content: &str) -> (&str, Vec<(&str, &str)>) {
    let Some(rest) = content.strip_prefix("---\n") else {
        return (content, Vec::new());
    };
    let Some(end) = rest.find("\n---\n") else {
        return (content, Vec::new());
    };
    let fm_block = &rest[..end];
    let body = &rest[end + 5..];
    let fields = fm_block
        .lines()
        .filter_map(|line| {
            let (key, val) = line.split_once(':')?;
            Some((key.trim(), val.trim()))
        })
        .collect();
    (body, fields)
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

fn render_page(page: &Page) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body {{ max-width: 48rem; margin: 2rem auto; padding: 0 1rem; font-family: system-ui, sans-serif; line-height: 1.6; }}
pre {{ background: #f4f4f4; padding: 1rem; overflow-x: auto; border-radius: 4px; }}
code {{ font-size: 0.9em; }}
nav {{ margin-bottom: 2rem; }}
nav a {{ margin-right: 1rem; }}
</style>
</head>
<body>
{body}
</body>
</html>"#,
        title = page.title,
        body = page.body_html,
    )
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

    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.autolink = true;
    opts.render.unsafe_ = true;

    let pages: Vec<Page> = md_files
        .iter()
        .map(|path| {
            let content = fs::read_to_string(path).expect("failed to read md file");
            let (body, fields) = parse_frontmatter(&content);

            let title = fields
                .iter()
                .find(|(k, _)| *k == "title")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| {
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

            let body_html = markdown_to_html(body, &opts);

            Page {
                title,
                slug,
                body_html,
            }
        })
        .collect();

    for page in &pages {
        let out_path = if page.slug.is_empty() {
            out_dir.join("index.html")
        } else {
            let dir = out_dir.join(&page.slug);
            fs::create_dir_all(&dir).expect("failed to create page dir");
            dir.join("index.html")
        };

        let html = render_page(page);
        fs::write(&out_path, html).expect("failed to write html");
        eprintln!("  wrote {}", out_path.display());
    }

    eprintln!("built {} pages into {}", pages.len(), out_dir.display());
}
