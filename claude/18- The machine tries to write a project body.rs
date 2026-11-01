//! ============================================================
//! DONE: — IMPLEMENTED IN THE LAST CODE (NOT YET VERIFIED)
//! ============================================================
//!
//! NOTE: nothing below has been compiled or run yet. Run `cargo check`, then
//! `cargo test`, and fix what the compiler finds before trusting it.
//!
//! CORE APPLICATION
//! [x] Ratatui terminal interface.
//! [x] Modular architecture using a shared Module interface.
//! [x] Tab-based navigation between modules.
//! [x] Shared logging system (history.jsonl; every tab writes to it).
//! [x] JSON-based persistent storage (written to a temp file and renamed, and a
//! file that can't be read is kept as <name>.corrupt instead of crashing).
//! [x] Fuzzy search across all tabs, with #tag search.
//! [x] Shared interface for listing items, viewing details, and actions.
//! [x] Generic Record / ListModule: add, edit, delete, views and tag filter are
//! written once for every tab instead of once per tab.
//! [x] Lists can be grouped under headings.
//!
//! TODO TAB, GROUPS AND FOCUS
//! [x] Todos belong to a group: main, quick (easily done), steady (needs a little
//! work every day), resurface (comes back every day), secondary, serotonin.
//! [x] The todo tab opens on "focus" (hides secondary and serotonin) and says how
//! many are hidden. `v` cycles focus / all / one group at a time.
//! [x] Todos older than a week show their age.
//! [x] Resurface todos reopen themselves the day after you finish them.
//! [x] Done tab: newest first, with views today / week / all.
//!
//! TAGS
//! [x] #tags on todos, habits, dailies, dictionary pages, events and diary lines.
//! [x] `t` filters a tab by one tag. Search "#tag" finds a tag across all tabs.
//!
//! CUBES TAB
//! [x] Open todos shown as blocks with a size (m) and a time on the timeline (s).
//! [x] Placed cubes are painted on the day bar for their day (green).
//!
//! DAILY TAB
//! [x] Dailies repeat on chosen weekdays: daily, weekdays, weekend, mon-fri, mon,wed...
//! [x] Duration and specifications; can be put on the day bar at a start time (s).
//! [x] Crossing midnight works (sleep 23:00 for 7 hours shows on both days).
//! [x] Today's dailies are listed first; ticking one off counts for today only.
//!
//! HABIT TAB (BUILD AND QUIT)
//! [x] Habits to build ("at least N a day") and habits to quit ("at most N a day").
//! [x] + counts one today, - takes one back; missed days count as zero by themselves.
//! [x] Streak, best streak, last 30 days, last 14 days at a glance.
//!
//! AGENDA, CALENDAR AND DAY BAR
//! [x] Agenda: upcoming events by day, views upcoming / past / all.
//! [x] Events can run past midnight (end earlier than start = ends the next day).
//! [x] Calendar: a month grid; the day bar shows the SELECTED day.
//! [x] Day bar: 24 hours, passed time shaded, planned time coloured by kind
//! (event, daily, cube), "now" marked, legend underneath.
//!
//! DIARY
//! [x] One line per Enter press, timestamped, grouped by day, #words become tags.
//!
//! AI TAB (ASKS BEFORE EVERY STEP)
//! [x] The assistant only proposes; every proposal waits for your yes or no.
//! [x] Yes runs the action like a key press on the target tab and logs it; No is
//! remembered and never asked again. proposals.json keeps every answer.
//! [x] Built-in rules: ask to mark a cube done once its time is over; ask to move a
//! main todo to secondary after 30 days.
//! [x] Advisor trait: the one place a local model plugs in.
//!
//! ============================================================
//! TODO: — PLANNED FEATURES
//! ============================================================
//!
//! 0. CHECK THE LAST CODE FIRST
//! [ ] Run cargo check and fix compilation errors (nothing here has been compiled).
//! [ ] Run cargo test; add tests for the day-bar painting and the AI flow.
//! [ ] Try a real day: sleep across midnight, a habit streak, a resurfacing todo.
//!
//! 1. TIMELINE AND ACTIVITY CUBES
//! [ ] Build the main timeline as the central application view.
//! [ ] Display activity names and relevant information inside blocks.
//! [ ] Support overlapping activities and scheduling conflicts.
//! [ ] Preserve historical timelines for past days (the bar shows past days'
//! plans, but not what really happened).
//! [ ] Cubes that cross midnight listed on both days in the cubes tab.
//!
//! 2. DRAG-AND-DROP INTERACTION
//! [ ] Support mouse interaction with timeline blocks.
//! [ ] Drag activities to different times.
//! [ ] Resize blocks to change activity duration.
//! [ ] Move activities between dates.
//! [ ] Update the underlying schedule when a block moves.
//! [ ] Display clear feedback when activities conflict.
//!
//! 3. AUTOMATIC COMPLETION AND TIME TRACKING
//! [ ] Check the current time when the application starts (ticks run now; they
//! only reopen resurface todos and ask the AI questions).
//! [ ] Detect activities whose scheduled end times have passed (done for cubes:
//! the AI asks; not yet for events and dailies).
//! [ ] Apply each activity's completion rules; automatically move eligible
//! activities from Todo to Done without asking, when the user allows it.
//! [ ] Process missed transitions after the application was closed.
//! [ ] Avoid completing the same activity more than once.
//! [ ] Distinguish time-based completion from activities requiring manual
//! confirmation.
//! [ ] Handle activities crossing midnight.
//!
//! Example:
//! Sleep is scheduled from 23:00 on June 3 until 06:00 on June 4.
//! When the app opens after 06:00 on June 4, it recognizes the end
//! of the sleep period and moves that occurrence to Done, preserving
//! its original scheduled date and times.
//!
//! 4. INTERCONNECTED MODULES
//! [ ] Establish a shared underlying activity model (todos, cubes, dailies and
//! events are still four separate kinds of entry).
//! [ ] Ensure modules reference shared activities instead of duplicating them.
//! [ ] Define how changes in one module affect other modules.
//! [ ] Connect Todo and Done to the timeline (done through cubes).
//! [ ] Connect Agenda and Calendar to shared scheduling data (they share
//! events.json; dailies and cubes are not in the agenda yet).
//! [ ] Connect Habit tracking to scheduled and completed activities (a daily
//! that is ticked off should count a habit).
//! [ ] Allow diary entries to reference relevant activities when useful.
//! [ ] Keep module views synchronized when an activity changes.
//! [ ] Maintain consistent IDs, dates, and history across modules.
//!
//! 5. HABIT SYSTEM
//! [ ] Weekly targets and schedules (three times a week, not every day).
//! [ ] Time-of-day targets, such as waking at 06:00 (record the actual time).
//! [ ] Distinguish skipped habits from missed ones.
//! [ ] Connect habits to relevant timeline activities.
//! [ ] Weekly consistency view.
//!
//! 6. TODO LIST CALM
//! [ ] Snooze a todo until a date, so it leaves the list and comes back.
//! [ ] A tag / group overview screen with counts.
//! [ ] Bulk actions: move all quick todos, tag several at once.
//! [ ] Show only what fits today by comparing cube sizes with free time.
//!
//! 7. LOCAL AI THAT KNOWS YOUR WRITING
//! [ ] Run a local, non-commercial model on the machine; nothing leaves it.
//! [ ] Train or prompt it on the diary, todos and history.jsonl (your own writing).
//! [ ] Implement Advisor for it, so its ideas arrive as yes/no questions.
//! [ ] Learn from proposals.json (every yes and no is a labelled example).
//! [ ] More rules and ideas: suggest groups and tags for new todos, suggest a time
//! slot for a cube, notice a habit slipping, draft diary lines from the day.
//! [ ] A "why" the AI can explain, and a way to tell it "never ask about this".
//!
//! 8. DIARY INTEGRATION
//! [ ] Associate diary entries with dates and relevant activities.
//! [ ] Review diary entries alongside the timeline.
//! [ ] Optionally link diary entries to habits or tasks.
//! [ ] Edit or delete a diary line.
//!
//! 9. PERSISTENCE AND RELIABILITY
//! [ ] Encrypt data at rest ("blackbox").
//! [ ] Recover correctly after closing and restarting the app.
//! [ ] Prevent duplicate completion records.
//! [ ] Test persistence and automatic state transitions.
//! [ ] Split the single file into modules (core, records, tabs, ui) once it settles.
//!
//! 10. FINAL INTEGRATION AND TESTING
//! [ ] Define the responsibilities and data relationships of all modules.
//! [ ] Implement central timeline and cube rendering.
//! [ ] Implement drag-and-drop scheduling.
//! [ ] Implement automatic completion rules.
//! [ ] Test interactions across connected modules.
//! [ ] Test realistic daily usage, including sleep, habits, and missed schedules.
//!
//! 11. FIND USERS SCATTERED LISTS AND GET THEM DONE.
//! [ ] Get a list of users online saves and melt them done.
//!
