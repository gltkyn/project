//! sync: carries entries written in the phone app to the PC.
//!
//! The phone app appends entries, as TOML, to
//!     <phone>/Android/data/dev.dortyuzlu.diary/files/outbox.toml
//!
//!     [[entry]]
//!     id = "1761457260123"
//!     module = "diary"          # or "todo", "timetable"
//!     text = "..."
//!     at = "2026-10-26 05:41"
//!
//! When the phone is plugged in with USB debugging on, this program pulls that file and
//! appends every entry it has not imported before to <data folder>/inbox.jsonl, which the
//! TUI picks up (same inbox the browser panel uses).
//!
//! It never changes anything on the phone. Which entries are already imported is kept by
//! id in <data folder>/phone.seen, so a crash or a repeated sync can't import one twice.
//! (If a crash happens between the inbox write and the seen write, one entry can arrive
//! twice; none can be lost.)
//!
//! `sync`          one run, then exit.
//! `sync --watch`  keep running, sync whenever a phone appears and once a minute after
//!                 (i3:  exec --no-startup-id /path/to/sync --watch ).

use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
    time::Duration,
};

const REMOTE_FILE: &str = "/sdcard/Android/data/dev.dortyuzlu.diary/files/outbox.toml";

fn data_dir() -> PathBuf {
    match std::env::var_os("DORTYUZLU_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join("dortyuzlu"),
            None => PathBuf::from("."),
        },
    }
}

fn adb(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("adb")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run adb: {}", e))
}

fn phone_connected() -> bool {
    match adb(&["get-state"]) {
        Ok(out) => out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "device",
        Err(_) => false,
    }
}

fn remote_exists(path: &str) -> bool {
    let test = format!("[ -f {} ] && echo yes", path);
    match adb(&["shell", &test]) {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "yes",
        Err(_) => false,
    }
}

fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(format!("{}\n", line).as_bytes())
}

// Returns how many new entries arrived.
fn sync_once() -> Result<usize, String> {
    if !remote_exists(REMOTE_FILE) {
        return Ok(0); // nothing written on the phone yet
    }

    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let local = dir.join("phone.pulled.toml");
    let local_text = local.to_string_lossy().to_string();
    let pulled = adb(&["pull", REMOTE_FILE, &local_text])?;
    if !pulled.status.success() {
        return Err(format!("pull failed: {}", String::from_utf8_lossy(&pulled.stderr).trim()));
    }

    let text = fs::read_to_string(&local).map_err(|e| e.to_string())?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("the phone's outbox.toml is not valid TOML: {}", e))?;
    let entries = match table.get("entry").and_then(|v| v.as_array()) {
        Some(list) => list.clone(),
        None => return Ok(0),
    };

    let seen_path = dir.join("phone.seen");
    let mut seen: HashSet<String> = fs::read_to_string(&seen_path)
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .collect();

    let inbox = dir.join("inbox.jsonl");
    let rejected = dir.join("sync.rejected.jsonl");
    let mut count = 0;

    for entry in entries {
        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if id.is_empty() {
            // no id means we cannot tell it from an old one; keep it aside for a human
            let raw = serde_json::to_string(&entry).unwrap_or_default();
            append_line(&rejected, &raw).map_err(|e| e.to_string())?;
            continue;
        }
        if seen.contains(&id) {
            continue;
        }

        let module = entry.get("module").and_then(|v| v.as_str());
        let body = entry.get("text").and_then(|v| v.as_str());
        let at = entry.get("at").and_then(|v| v.as_str()).unwrap_or("");

        match (module, body) {
            (Some(module), Some(body)) => {
                let line = serde_json::json!({ "module": module, "text": body, "at": at });
                append_line(&inbox, &line.to_string())
                    .map_err(|e| format!("could not write {}: {}", inbox.display(), e))?;
                count += 1;
            }
            _ => {
                let raw = serde_json::to_string(&entry).unwrap_or_default();
                append_line(&rejected, &raw).map_err(|e| e.to_string())?;
            }
        }
        // remembered only after the entry is safely on the PC
        append_line(&seen_path, &id).map_err(|e| e.to_string())?;
        seen.insert(id);
    }

    let _ = fs::remove_file(&local);
    Ok(count)
}

fn main() {
    let watch = std::env::args().any(|a| a == "--watch");
    if !watch {
        if !phone_connected() {
            eprintln!("no phone connected (is USB debugging on and adb authorised?)");
            std::process::exit(1);
        }
        match sync_once() {
            Ok(n) => println!("{} new entries from the phone", n),
            Err(e) => {
                eprintln!("sync failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // watch mode: sync each time the phone newly appears, and again every minute while it stays
    let mut was_connected = false;
    let mut idle_polls = 0u32;
    loop {
        let connected = phone_connected();
        if connected && (!was_connected || idle_polls >= 12) {
            idle_polls = 0;
            match sync_once() {
                Ok(0) => {}
                Ok(n) => println!("{} new entries from the phone", n),
                Err(e) => eprintln!("sync failed: {}", e),
            }
        }
        idle_polls += 1;
        was_connected = connected;
        sleep(Duration::from_secs(5));
    }
}
