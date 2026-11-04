---

## What changed from the previous version

**Structural:**

- "Two binaries" → "one binary, panel is a thread"
- Panel CLI flags documented (`--no-panel`, `--no-tui`, `--headless`, `--status`, `--stop`)
- New "STATUS MARKERS" section (`[x]` / `[~]` / `[ ]` / `[!]`)
- New "WHAT IS ACTUALLY DONE" section — honest separation of "written but unverified" from "working"
- New "THE SCHEDULING MODEL" section — the cube attributes, negotiation flow, tier learning, quitting habits
- New "THE BUS — status of every edge" table — 5/18 works, named
- Named compile-risks section instead of vague "fix whatever `cargo check` says"
- Sequencing updated: added "Bus edge fixes" and "Phase 2 — negotiation" and "Phase 5 — flip views" as distinct steps
- Decisions log expanded from 5 entries to ~20, grouped by theme

**Cut from previous version:**

- "Two binaries" (stale)
- The old aspiration for `Activity` as "todos, dailies, and events each store their own time" — now it's a specific file with tests
- Empty sequencing row "Verify" as if it were a task — now it's the gate with named risks
- Vague "Fix whatever cargo check reports" — now named risks

**Added to previous version:**

- The scheduling model (the biggest addition — this is the heart of the app)
- The bus edge status table
- Tier learning, waiting queue, check-in flow
- The check-in stub status (`checkins.jsonl` writes, nothing reads)
- The 13 remaining bus edge fixes as a concrete task list
- The decisions log entries for everything we locked in
