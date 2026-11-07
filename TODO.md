# Todos

[x] - Make ai write a skeleton.
[p] - use that to achieve a hand written 0.1.0
  [ ] - Has todo.
  [ ] - Uses ratatui.
  [ ] - Has localhost for diary
  [ ] - 
  [ ] - 
  [ ] - 
[ ] - Write 0.1.0
[ ] - Write a prototype and push it to github.
[ ] - Add todo to code.
[ ] - Add diary to code.
  [ ] - Add localhost 
  [ ] - Add to nvim greeter.
  [ ] - Add to mobile.
  [ ] - Add voice inputs.
  [ ] - Add etc...
[ ] - Add ratatui to code.
[ ] - Add a yz that handles all the underlying system to code.
[ ] - Code can add and mark todos.
[ ] - Code can take inputs from a single bar in in diary tab.
[ ] - Code has a timeline.
[ ] - IS able to take all your {} in.
  [ ] - Todos
  [ ] - bookmarks.
  [ ] - Notes.
  [ ] - etc.
[ ] - 

---

#
#
#
#
#

---

[x] - 0. Gate — verify what actually compiles/runs
[x] - Run `cargo check` / `cargo test` on current `src/main.rs` and record failures [!] Decide: `src/item.rs` and `src/activity.rs` are NOT declared as `mod` in `main.rs`
-> they are dead code right now. Keep, merge, or delete?
[ ] Decide which model is canonical: `Item` in `main.rs` vs `Item` in `item.rs` vs `Activity` in `activity.rs`
-> do not keep 3 competing models

### 1. Core model — one thing

[ ] Pick ONE struct: `Item { id, title, body, tags, kind, state, tier, throwable, when, minutes, floor, progress, parent_id, ai_notes, tier_locked, check_in, created }`
[ ] Pick ONE `Kind`: Task / Event / Daily{days} / HabitMake{target} / HabitQuit{target,replacement} / Note / Project
[ ] Pick ONE `State`: Open / Done{on} / Failed{on,why} / Waiting{for_date} / Deferred{until} / Skipped{on}
[ ] Pick ONE `Tier`: Requirement / Developing / Optional / Quitting
[ ] Pick ONE `Throwable`: Never / UnderPressure / Anytime
[ ] Remove duplicate definitions from `main.rs` once canonical file is chosen
[ ] Keep serde round-trip tests for every variant (already in `item.rs` — port them)

### 2. Storage — Store<T>

[ ] One generic `Store<T: Entry>` -> `items.json`, `diary.json`, `proposals.json`
[ ] Atomic write: write to `.tmp` then rename (already in `main.rs` — verify)
[ ] Corrupt-file handling: rename to `.corrupt` instead of crashing (already in `main.rs` — verify)
[ ] `history.jsonl` append-only log: who/what/when for every change
[ ] Decide data dir: `~/dortyuzlu/` vs `$DORTYUZLU_DIR` — document it

### 3. Diary -> Intents (front door)

[ ] `parse_intents(text)` : `todo: X`, `done X`, `finished X`, `14:00-15:30 X`, `title N`
[ ] Diary entry is raw input only — AI never edits diary directly
[ ] Every diary line gets an `id`, `date`, `intent`, `text`
[ ] Test: overnight entry e.g. `21:00-05:00 sleep` — how to enter? (your open question in `claude/06- note.md`)

### 4. Curator / AI — proposals only, user approves

[ ] `Curator::advise(items, diary, proposals) -> Vec<Proposal>`
[ ] Proposal kinds: create-task, complete-task, create-event, create-habit, defer, skip, tier-change
[ ] Proposal lifecycle: Pending -> Approved / Rejected, never auto-apply
[ ] Run curator: at startup, every 30s, and on `r` in AI tab
[ ] `apply_change(proposal)` only after `y`, discard after `n`
[ ] Dedupe proposals by stable key e.g. `diary-todo:{line_id}:{i}`

### 5. Views — filters over the same store, via one `View` trait

[ ] `trait View { name(), rows(core), detail(core,id), actions(), apply(core,action,input) }`
[ ] TodoView: open tasks only, grouped/sorted so list is not scary
[ ] Group by: easily-done / daily-work / resurface / secondary / serotonin
[ ] Tags + groups: Main / Steady / Resurface / Quick / Secondary / Serotonin
[ ] TimelineView: 24h day-bar, now-marker, events for selected date
[ ] DiaryView: free-text input, shows parsed intent hints
[ ] HabitsView: HabitMake + HabitQuit, streak / count_on(today), `+` / `-`
[ ] AiView: pending proposals first, answered below, `y` / `n` / `r`
[ ] Shared chrome: Tab / Left / Right switch, `j`/`k` move, bottom input line, history pane

### 6. Scheduling model — the heart (from `main.md`)

[ ] Cube attributes: `wants{date,start,minutes}`, `floor`, `tier`, `throwable`, `check_in`
[ ] Negotiation flow: AI proposes slot -> user yes/no -> place or defer
[ ] Waiting queue: `Waiting{for_date}` when no slot fits
[ ] Tier learning: repeated defer/skip -> suggest tier change (proposal only)
[ ] Quitting habits: HabitQuit with replacement item
[ ] Check-in flow: `checkins.jsonl` is written — nothing reads it yet -> implement reader or remove

### 7. Panel (web) <-> TUI bus

[ ] Axum panel on `127.0.0.1:3000`, single textarea, `POST /add`
[ ] Panel only writes `inbox.jsonl` — TUI polls every 2s and drains it
[ ] CLI flags: `--no-panel`, `--no-tui`, `--headless`, `--status`, `--stop` — verify all exist
[ ] Fix bus edges that don't work yet (your `main.md` says 5/18 work) — list each edge, fix one by one

### 8. Phone sync

[ ] Android app writes `outbox.toml` — no consumer in live Rust tree yet
[ ] Decide: add `toml` crate + reader in Rust, or drop phone sync for now
[ ] If kept: one-way phone -> desktop only, document it

### 9. Cleanup / structural

[ ] Archive `claude/` and `deepseek/` out of build — they are notes/history, not source
[ ] Split `src/main.rs` (1,388 lines) once model is frozen: `model.rs`, `store.rs`, `curator.rs`, `views/*.rs`, `app.rs`
[ ] Do NOT split before step 1 is decided — that was the "premature code" problem in your notes
[ ] Remove `RefCell` confusion / clarify ownership in `App { core, views, selected, tab, mode }`

### 10. Tests

[ ] Keep existing: item round-trip, habit_good_on, effective_minutes, parse_timetable
[ ] Add: parse_intents, curator dedupe, proposal apply/reject, inbox drain, store corrupt recovery
