//! sync: carries diary lines written on the phone to the PC.
//!
//! The phone app appends one JSON line per entry to
//!     /sdcard/dortyuzlu/outbox.jsonl      {"module":"diary","text":"...","at":"YYYY-MM-DD HH:MM"}
//! (the same shape the browser panel writes). When the phone is plugged in
//! with USB debugging on, this program:
//!   1. renames outbox.jsonl to outbox.sending.jsonl ON THE PHONE (atomic, so lines the
//!      app writes meanwhile go to a fresh outbox.jsonl and are never lost),
//!   2. pulls that file,
//!   3. appends the valid lines to <data folder>/inbox.jsonl (the TUI takes them from there),
//!   4. only then deletes the file on the phone.
//! A crash between 3 and 4 can deliver the same lines twice; it can never lose any.
//!
//! `sync`          one run, then exit.
//! `sync --watch`  keep running and sync whenever a phone appears (put this in i3:
//!                 exec --no-startup-id /path/to/sync --watch ).

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
    thread::sleep,
    time::Duration,
};

const REMOTE_DIR: &str = "/sdcard/dortyuzlu";

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

fn append_line(path: &PathBuf, line: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(format!("{}\n", line).as_bytes())
}

// Returns how many lines arrived.
fn sync_once() -> Result<usize, String> {
    let outbox = format!("{}/outbox.jsonl", REMOTE_DIR);
    let sending = format!("{}/outbox.sending.jsonl", REMOTE_DIR);

    // a leftover from an interrupted run is delivered first; otherwise rotate the outbox
    if !remote_exists(&sending) {
        if !remote_exists(&outbox) {
            return Ok(0);
        }
        let moved = adb(&["shell", "mv", &outbox, &sending])?;
        if !moved.status.success() {
            return Err("could not rename the outbox on the phone".into());
        }
    }

    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let local = dir.join("phone.pulling.jsonl");
    let local_text = local.to_string_lossy().to_string();
    let pulled = adb(&["pull", &sending, &local_text])?;
    if !pulled.status.success() {
        return Err(format!("pull failed: {}", String::from_utf8_lossy(&pulled.stderr).trim()));
    }

    let text = fs::read_to_string(&local).map_err(|e| e.to_string())?;
    let inbox = dir.join("inbox.jsonl");
    let rejected = dir.join("sync.rejected.jsonl");
    let mut count = 0;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let valid = serde_json::from_str::<serde_json::Value>(line)
            .map(|v| v["module"].is_string() && v["text"].is_string())
            .unwrap_or(false);
        let target = if valid { &inbox } else { &rejected };
        append_line(target, line).map_err(|e| format!("could not write {}: {}", target.display(), e))?;
        if valid {
            count += 1;
        }
    }

    // everything is safely on the PC: now the phone's copy can go
    let removed = adb(&["shell", "rm", &sending])?;
    if !removed.status.success() {
        return Err("lines saved, but could not delete the phone's sent file (they may arrive twice)".into());
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
            Ok(n) => println!("{} entries from the phone", n),
            Err(e) => {
                eprintln!("sync failed: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // watch mode: try each time the phone newly appears, and again every minute while it stays
    let mut was_connected = false;
    let mut idle_polls = 0u32;
    loop {
        let connected = phone_connected();
        if connected && (!was_connected || idle_polls >= 12) {
            idle_polls = 0;
            match sync_once() {
                Ok(0) => {}
                Ok(n) => println!("{} entries from the phone", n),
                Err(e) => eprintln!("sync failed: {}", e),
            }
        }
        idle_polls += 1;
        was_connected = connected;
        sleep(Duration::from_secs(5));
    }
}
