//! panel: axum server on :3000. Writes only to inbox.jsonl; TUI drains it.

use std::io::Write;
use std::process::ExitCode;

use axum::{
    body::Bytes,
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use chrono::Local;
use serde::Deserialize;

use crate::{data_path, InboxLine};

const PAGE: &str = include_str!("panel/page.html");
const CSS: &str = include_str!("panel/style.css");
const JS: &str = include_str!("panel/app.js");

pub fn spawn_background(addr: String) -> Result<(), String> {
    std::thread::Builder::new()
        .name("panel".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2).enable_all().build()
                .expect("tokio");
            rt.block_on(async move {
                match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => {
                        println!("panel on http://{}", addr);
                        if let Err(e) = axum::serve(l, router()).await {
                            eprintln!("panel: {}", e);
                        }
                    }
                    Err(e) => eprintln!("panel: bind {} failed: {}", addr, e),
                }
            });
        })
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn run_foreground(addr: String) -> ExitCode {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => { eprintln!("tokio: {}", e); return ExitCode::FAILURE; }
    };
    let result = rt.block_on(async move {
        let l = tokio::net::TcpListener::bind(&addr).await?;
        println!("panel on http://{}", addr);
        axum::serve(l, router()).await
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("panel: {}", e); ExitCode::FAILURE }
    }
}

fn router() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/style.css", get(style))
        .route("/app.js", get(script))
        .route("/add", post(add_entry))
        .route("/health", get(health))
}

async fn index() -> Html<&'static str> { Html(PAGE) }
async fn style() -> impl IntoResponse { ([(header::CONTENT_TYPE, "text/css")], CSS) }
async fn script() -> impl IntoResponse { ([(header::CONTENT_TYPE, "text/javascript")], JS) }
async fn health() -> &'static str { "ok\n" }

#[derive(Deserialize)]
struct AddBody {
    #[serde(default = "def_diary")] module: String,
    text: String,
}

fn def_diary() -> String { "diary".into() }

async fn add_entry(body: Bytes) -> StatusCode {
    let raw = String::from_utf8_lossy(&body).trim().to_string();
    if raw.is_empty() { return StatusCode::BAD_REQUEST; }
    let (module, text) = if let Ok(j) = serde_json::from_str::<AddBody>(&raw) {
        (j.module, j.text)
    } else {
        ("diary".to_string(), raw)
    };
    let entry = InboxLine {
        module, text,
        at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    let path = data_path("inbox.jsonl");
    match (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(f, "{}", serde_json::to_string(&entry).unwrap())?;
        Ok(())
    })() {
        Ok(()) => StatusCode::OK,
        Err(e) => { eprintln!("inbox: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}
