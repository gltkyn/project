//! panel: the browser input panel.
//!
//! Open http://localhost:3000 and type. Plain text is a diary line;
//! "todo: ..." is a todo; "timetable: 14:00-15:30 Dentist" is an agenda event.
//! Todos take the same quick-entry words as the TUI:  todo: Buy milk #errands @quick
//!
//! This program never touches the TUI's data files. It only appends one line per
//! entry to <data folder>/inbox.jsonl; the TUI picks the lines up within a couple
//! of seconds (or when it next starts) and adds them through its own tabs.
//! The data folder is ~/dortyuzlu, or DORTYUZLU_DIR if that is set.
//!
//! Only this page may post to it: a request must come to localhost and carry the
//! header X-Dortyuzlu. A web page on another site can't send that header without
//! the browser asking us first (a CORS preflight), and we never say yes.
//! A browser extension that posts here must send the header too:
//!     fetch("http://127.0.0.1:3000/add", { method: "POST", headers: { "X-Dortyuzlu": "1" }, body: text })

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Html,
    routing::{get, post},
    Router,
};
use chrono::Local;
use serde::Serialize;

const MAX_LEN: usize = 20_000;

const HTML: &str = r##"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>dörtyüzlü</title>
    <style>
        body {
            margin: 0;
            background: #1e1e2e;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            height: 100vh;
            font-family: monospace;
            color: #cdd6f4;
        }

        .container {
            width: 500px;
            display: flex;
            flex-direction: column;
            gap: 12px;
        }

        .hint {
            font-size: 12px;
            color: #585b70;
            text-align: center;
        }

        #status {
            font-size: 12px;
            text-align: center;
            min-height: 1em;
        }

        input {
            width: 100%;
            padding: 12px;
            background: #181825;
            color: #cdd6f4;
            border: 2px solid #313244;
            border-radius: 8px;
            outline: none;
            font-size: 16px;
            font-family: inherit;
        }

        input:focus {
            border-color: #89b4fa;
        }
    </style>
</head>

<body>
    <div class="container">
        <input id="input" autofocus placeholder="Write diary, todo: [task], or timetable: [time-range event]...">
        <div id="status"></div>
        <div class="hint">diary (default) | todo: buy milk #errands @quick | timetable: 14:00-15:30 dentist</div>
    </div>

    <script>
        const input = document.getElementById("input");
        const status = document.getElementById("status");

        function show(message, bad) {
            status.textContent = message;
            status.style.color = bad ? "#f38ba8" : "#a6e3a1";
        }

        input.addEventListener("keydown", async (event) => {
            if (event.key !== "Enter" || !input.value.trim()) return;
            try {
                const reply = await fetch("/add", {
                    method: "POST",
                    headers: { "X-Dortyuzlu": "1" },
                    body: input.value
                });
                const message = await reply.text();
                if (reply.ok) {
                    input.value = "";
                    show(message, false);
                } else {
                    show(message, true); // keep the text so nothing typed is lost
                }
            } catch (error) {
                show("the panel server is not reachable", true);
            }
        });
    </script>
</body>
</html>
"##;

fn data_dir() -> PathBuf {
    match std::env::var_os("DORTYUZLU_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join("dortyuzlu"),
            None => PathBuf::from("."),
        },
    }
}

// One line of inbox.jsonl. The TUI reads exactly this shape.
#[derive(Serialize)]
struct InboxLine<'a> {
    module: &'a str,
    text: &'a str,
    at: String, // "YYYY-MM-DD HH:MM" it was typed
}

// "todo: buy milk" -> ("todo", "buy milk"); no prefix means diary
fn split_module(content: &str) -> (&'static str, &str) {
    for (prefix, module) in [("todo:", "todo"), ("timetable:", "timetable"), ("diary:", "diary")] {
        if let Some(rest) = content.strip_prefix(prefix) {
            return (module, rest.trim());
        }
    }
    ("diary", content)
}

// Must be addressed to localhost (stops DNS-rebinding tricks) and carry our header.
fn from_our_page(headers: &HeaderMap) -> bool {
    let local = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map_or(false, |h| h.starts_with("127.0.0.1:") || h.starts_with("localhost:"));
    local && headers.contains_key("x-dortyuzlu")
}

fn append_to_inbox(line: &InboxLine) -> std::io::Result<()> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string(line)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("inbox.jsonl"))?;
    // one write call, so two entries sent at once can't be torn into each other
    file.write_all(format!("{}\n", json).as_bytes())
}

async fn index() -> Html<&'static str> {
    Html(HTML)
}

async fn add_entry(headers: HeaderMap, body: String) -> (StatusCode, String) {
    if !from_our_page(&headers) {
        return (StatusCode::FORBIDDEN, "refused: not sent from the panel".to_string());
    }
    let content = body.trim();
    if content.is_empty() {
        return (StatusCode::BAD_REQUEST, "nothing to save".to_string());
    }
    if content.len() > MAX_LEN {
        return (StatusCode::PAYLOAD_TOO_LARGE, "too long".to_string());
    }
    let (module, text) = split_module(content);
    if text.is_empty() {
        return (StatusCode::BAD_REQUEST, format!("nothing after \"{}:\"", module));
    }

    let line = InboxLine {
        module,
        text,
        at: Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    match append_to_inbox(&line) {
        Ok(()) => (StatusCode::OK, format!("sent to {}", module)),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("could not save: {}", error)),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/add", post(add_entry));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("could not bind 127.0.0.1:3000 (is another panel already running?)");

    println!("dörtyüzlü panel at http://localhost:3000");
    println!("entries go to {}", data_dir().join("inbox.jsonl").display());

    axum::serve(listener, app).await.expect("server stopped");
}
