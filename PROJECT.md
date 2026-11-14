# Life-OS: project brief and module plan

A starter written 2026-11-09 from one long chat. Edit anything. Start every new chat with this
file plus only the file(s) that chat will touch.

## 1. What it is (settled so far)

- A personal, local-first life manager in Rust, for Arch Linux. Terminal UI (ratatui) today.
  Data lives in `~/project/` (`PROJECT_DIR` overrides it). 
<!-- I have not decided on the name -->
- Tabs are views of the same data, not separate programs. todo, done and cubes all look at
  `todos.json`. 
<!-- - The difference being that cube has a metadata about it: In which hours, Is it daily or weekly etc. -->
- The diary is the front door. Lines such as `todo: buy milk`, `finished laundry`, `smoked 3`
  and `14:00-15:30 Dentist` turn into questions for the AI.
<!-- convinience of the user makes a big difference so the mobile app should have a "adder" that user can use with trust to its security and the app should be opening so fast(user should add entries fast) and The ui should be basic. All for the same reason: convinience. -->
- The AI only suggests. Every suggestion waits for a yes or no. What was done is always
  reported. A suggestion whose reason has gone is withdrawn, never acted on.
  <!-- I need to find a way to make this less tedious for the user and make it fast too. Ai asking everything can be boring (for a good reason.) Maybe the user can give automaton directivess to his locak ai -->
- No language models and no "modern AI". Half of the AI is a hard-coded library that ships with
  the app (tables, not code), half is learning from the user's answers (counters).
  <!-- YES -->
- Wanted, not built yet: encryption at rest, notify module, timeline blocks, a phone version
  (try Termux first).
  <!-- I am thinking of making this into a only phone app tbh. But the ai will need ram to work. Lets pin this idea too. -->
- Principle: convenience and inconvenience are signals. Put things in less convenient places
  to take. In code that means private by default; widen deliberately.
  <!-- This is a developer note and less of a prompt. Dont take it that way  -->

## 2. The plan: hidden modules

A module is a decision it hides. Cut by "what might I change my mind about?", not by tab.
Sizes are today's lines in `main.rs`.

| Module | Hides | Offers | Must never know | Today's code |
|---|---|---|---|---|
| `clock` | the clock library, date and time formats, overnight spans | `today`, `parse_date`, `minutes_of`, `clock`, `span_on`, `parse_when`, `day_heading` | todos, the UI | section 1, dates (~40) |
| `storage` | where data lives, the file format, atomic saves, ids never reused, corrupt-file recovery | `Entry`, `Store<T>`, `Shared<T>`, `share`, `data_path` | what any entry means | section 2, store (~90) |
| `history` | the log format | `Logger`, `LogEvent`, `Kind` | why something was logged | section 2, log (~45) |
| `text` | how words are read: tags, `@group`, openers, intents, title matching, fuzzy match | `split_text`, `join_text`, `hashtags`, `parse_intents`, `parse_timetable`, `pick_title`, `habit_score`, `fuzzy_score`, `Intent` | stores, the UI, the AI | sections 1 (tags), 3, 11 (~250) |
| `kit` | how a generic tab works: lenses, views, add/edit/delete, search | `Module`, `Record`, `ListModule`, `Row`, `Action`, `Request` | the terminal | sections 4 and 5 (~300) |
| `things/` | each kind of thing's own rules (streaks, groups, overnight events) | `Todo`, `Habit`, `Daily`, `CalEvent`, `Page`, `DiaryLine` | the UI, the AI | sections 6 to 11 (~1100) |
| `ai/` | how suggestions are made, stored, asked and withdrawn | `AiModule`; inside: `protocol`, `curator`, `companion` | tab key bindings | section 13 (~450) |
| `views/` | the two tabs that are not plain lists | `CalendarModule`, `SettingsModule` | the terminal | sections 12 and 13b (~250) |
| `hub` | which modules exist, how they are wired, ticks, routing approved requests, cross-tab search | `Hub::new`, `tick`, `flush`, `search`, `add_to` | the terminal | `App::new`, `tick_all`, `flush`, `run_search`, `add_to`, `add_diary_line` |
| `inbox` | the browser panel's file protocol | `check_inbox` | the terminal | `InboxLine`, `check_inbox`, `take_inbox_entry`, `handle_checkin` |
| `tui` | ratatui and crossterm: drawing, keys, the run loop | `run` | file formats, rules | section 14 (~600) |
| `main.rs` | start-up only | `main` | everything else | section 15: `parse_cli`, `main`, `migrate_to_activities` |

Tests move with the code they test.

## 3. Direction of dependencies

Arrows point down only. A module may use what is below it, never what is above.

```
main  ->  tui, inbox  ->  hub  ->  ai, views  ->  things  ->  kit  ->  text, history, storage, clock
```

- `clock`, `storage`, `history` and `text` import nothing from the app.
- Only `tui` may mention ratatui or crossterm.
- Mobile-readiness test: remove ratatui from `Cargo.toml`. Everything except `tui` must still
  compile.
  <!-- i might do that later. -->

## 4. Known leaks (fix while splitting)

1. `kit` uses ratatui's `Color` (`Module::block_color`, `Record::COLOR`). Use its own small
   enum and let `tui` turn it into a colour.
2. `text` knows the domain: `Intent::Todo` holds a `Group`, and `best_habit` takes `&[Habit]`.
   Make `Intent` hold plain strings and move `best_habit` into `ai/curator`.
3. The AI presses keys on tabs by name (`module "todo"`, key `'x'`). Approvals should be
   operations on things (mark done, add todo, count habit, add event), not key presses. That
   also ends the case of an approved action lost because a tab is switched off.
4. The Curator receives every todo, habit and diary line.
5. The Companion writes into the diary by itself, which conflicts with "the AI only suggests".
6. `ALL_MODULES` in settings repeats the module list that `App::new` builds.

## 5. How to split, one step per chat

0. `cargo check` and `cargo test` are clean, then `git commit` and tag it `before-split`.
1. Move leaves first: `clock`, then `storage` and `history`, then `text`. Then `kit`, `things/`,
   `ai/`, `views/`, then `hub`, `inbox`, `tui`, and last `main.rs`.
2. For each one: create `src/x.rs`, move the code, put `pub(crate)` on what other files use, add
   `mod x;` and `pub(crate) use x::*;` to `main.rs`, and start the new file with `use crate::*;`.
   Run `cargo check`, then `cargo test`, then commit.
3. Only after everything has moved: remove the `use crate::*;` lines and the re-exports one module
   at a time, and write explicit imports. This is the step that makes them real modules. Whatever
   no other file imports stays private. Until then you only have files.

## 6. Yours to decide

1. How far may the AI reach? One narrow view per advisor, and is raw history out of reach?
<!-- Lets leave the ai as a todo()! -->
2. Do approvals become operations on things (leak 3)?
3. Split `things/` per kind now, or wait for the `activity` model that merges todo, daily and
   event? Suggested: split now, so the merge later replaces files and not the architecture.
4. What happens to the Companion: removed, read-only, or allowed to write the diary?
5. Module names. The ones above are placeholders.

## 7. Starter for the next chat

> Context: Life-OS, see PROJECT.md (attached). Task: step 1 of section 5, move `clock` out of
> `main.rs` into `src/clock.rs`. No behaviour change. Give me patches, not the whole file. I will
> run `cargo check` and paste the errors.
<!-- sounds good -->
