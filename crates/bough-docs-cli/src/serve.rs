use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use tiny_http::{Header, Response, Server};

const OUT_DIR: &str = "target/bough-docs-site";

pub fn serve(port: u16) {
    let addr = format!("0.0.0.0:{port}");
    let server = Arc::new(Server::http(&addr).expect("failed to start server"));
    eprintln!("serving at http://localhost:{port}");

    let generation = Arc::new((Mutex::new(0u64), Condvar::new()));
    let watched_path: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));

    let watcher_gen = generation.clone();
    let watcher_path = watched_path.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: notify_debouncer_mini::DebounceEventResult| {
            let Ok(events) = res else { return };
            let target = watcher_path.lock().unwrap();
            let Some(target) = target.as_ref() else {
                return;
            };
            let matched = events.iter().any(|e| e.path == *target);
            if !matched {
                return;
            }
            let (lock, cvar) = &*watcher_gen;
            let mut count = lock.lock().unwrap();
            *count += 1;
            cvar.notify_all();
        },
    )
    .expect("failed to create file watcher");

    debouncer
        .watcher()
        .watch(Path::new(OUT_DIR), RecursiveMode::Recursive)
        .expect("failed to watch output dir");

    let root = Path::new(OUT_DIR);
    let pool_size = 4;
    let mut handles = Vec::new();

    for _ in 0..pool_size {
        let server = server.clone();
        let generation = generation.clone();
        let watched_path = watched_path.clone();

        handles.push(std::thread::spawn(move || loop {
            let Ok(request) = server.recv() else {
                continue;
            };

            let url_path = request.url().trim_start_matches('/');
            let url_path = url_path.split('?').next().unwrap_or(url_path);

            if url_path == "js/dev-server-refresh.js" {
                let (lock, cvar) = &*generation;
                let current = *lock.lock().unwrap();
                let _guard = cvar
                    .wait_while(lock.lock().unwrap(), |g| *g == current)
                    .unwrap();

                let header =
                    Header::from_bytes("Content-Type", "application/javascript").unwrap();
                let _ = request
                    .respond(Response::from_string("window.location.reload()").with_header(header));
                continue;
            }

            let file_path = if url_path.is_empty() {
                root.join("index.html")
            } else {
                let candidate = root.join(url_path);
                if candidate.is_dir() {
                    candidate.join("index.html")
                } else if candidate.exists() {
                    candidate
                } else {
                    candidate.join("index.html")
                }
            };

            let Ok(content) = fs::read(&file_path) else {
                let _ = request.respond(Response::from_string("404").with_status_code(404));
                continue;
            };

            let content_type = match file_path.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                _ => "application/octet-stream",
            };

            if content_type == "text/html; charset=utf-8" {
                eprintln!("  {}", request.url());
                *watched_path.lock().unwrap() = fs::canonicalize(&file_path).ok();
            }

            let content = if content_type == "text/html; charset=utf-8" {
                let mut html = String::from_utf8_lossy(&content).into_owned();
                html.push_str("\n<script>fetch('/js/dev-server-refresh.js').then(()=>location.reload())</script>");
                html.into_bytes()
            } else {
                content
            };

            let header = Header::from_bytes("Content-Type", content_type).unwrap();
            let _ = request.respond(Response::from_data(content).with_header(header));
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    drop(debouncer);
}
