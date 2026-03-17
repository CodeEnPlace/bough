use std::fs;
use std::path::Path;

use tiny_http::{Header, Response, Server};

const OUT_DIR: &str = "target/bough-docs-site";

pub fn serve(port: u16) {
    let addr = format!("0.0.0.0:{port}");
    let server = Server::http(&addr).expect("failed to start server");
    eprintln!("serving at http://localhost:{port}");

    let root = Path::new(OUT_DIR);

    for request in server.incoming_requests() {
        let url_path = request.url().trim_start_matches('/');
        let url_path = url_path.split('?').next().unwrap_or(url_path);

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

        let header = Header::from_bytes("Content-Type", content_type).unwrap();
        let _ = request.respond(Response::from_data(content).with_header(header));
    }
}
