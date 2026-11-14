# OpenPaint repository rules

This file is the shared entry point for agents and maintainers and sets the default way of working. Explicit instructions from the author in the current task take precedence.

## Reading order

1. Your brief from the main session, when there is one: it names what to read, what is already established and, for a part of a package, the contract and the files that are yours.
2. `docs/ROADMAP.md`: the package's lines; do only your part of it.
3. `docs/STATUS.md` and `docs/ARCHITECTURE.md`: only the parts the brief or the slice's crates call for.
4. The affected sources. Every file is under 400 lines: read whole files with Read, locate with Grep; a module's `//!` says what it holds.
5. `docs/DECISIONS.md`: only when changing an existing decision or when you need the "why".

Do not rebuild the big picture from source; the module map is in ARCHITECTURE. Do not dig through git history for progress; progress is in STATUS.

## Packages

- Size first (author, 2026-11-13: what they fear is time and tokens spent on process instead of the product, not slowness). A feature of one concept in one flow is built by ONE agent end to end from its mock or walk-through: the code, its tests, its walk-through scenario, the gate once, its lines in the documents, the commit. A contract and parts side by side are for work whose building alone takes hours and can be separated: their fixed cost was about ninety minutes on S124, four times the building. One thing a feature: no deletion, clean-up or tool rides along.
- The author accepts at a version, not at a feature (2026-11-13). Between versions the main session advances on its own, installs builds on the tablet for its own measurements, and writes to the author at a milestone or for what is theirs to decide. Nothing is cut thin for a quick look: what is built is built properly, once.
- Work runs in packages (author, 2026-11-11): a package is one result the author can try on the tablet; its lines in ROADMAP say what they will see and what it deletes. What replaces a flow is built as its end state, and the old flow, its tests and its scenarios are deleted in the same package: no commit in between has to keep the old product true, and nothing is migrated step by step.
- A package has four steps. Contract: a design, read-only, fixes the public types and signatures the parts meet at and gives each part files of its own; it is committed as a skeleton that compiles. Parts: up to three builders side by side, each in a worktree of its own, running `cargo check` and its crate's tests only, never the suite or the gate. Integration: one agent merges the parts, wires what crosses them, writes the walk-through scenarios, deletes what the package replaces, runs the suite and the gate once and writes the documents once. Try: a build on the tablet, measured against the build before it, five lines for the author, the package's cost read from the agents' transcripts.
- The proof follows the change. What replaces a flow: its pure tests, its new walk-through scenarios looked at, the tablet's measurements, the author's try. A refactor of code that stays: every screenshot byte for byte against a run of the commit before. A prune: the gate. The author's library is the hard limit throughout: a backup first, a changed shape proved on a copy.
- An agent does its brief (a design, a part, an integration, a small slice outside any package) and ends; it does not start the next. A slice outside a package still passes the gate and records itself.
- The author follows a progress board the main session keeps outside the repository; the four documents are the agents'.
- Run `git status --short --branch` first; never touch pre-existing uncommitted changes.
- ROADMAP defines the scope. Problems outside it go to STATUS "Known limits" or ROADMAP "Candidates"; do not fix them in passing.
- Stopping to ask the author is fine. Finish the parts that do not depend on the answer first, then list concrete options with a recommendation.
- A choice that only makes the interface consistent with what the author asked for is made, not listed as a decision for them.

## Reading and editing

- Read source with Read; search with Grep / Glob. Do not read source through cat, sed or grep commands.
- Edit source with Edit; use Write for new files or full rewrites. `sed` is fine where it is quicker (one-line changes, renames, hash replacements; author, 2026-10-04, after a trial in S30); no Python or heredoc scripts.
- Deletes name every path exactly (author, 2026-11-07): many at once is fine when each path is spelt out, from a listing looked at first or a tool that computes and prints them, or one named folder by its quoted absolute path; never shell wildcards or globs, never `cargo clean` or `git clean`, and never anything under `artifacts/device/backups/`.
- No .py / .sh / .ps1 files in the repository (`cargo repo-check` rejects them); tooling is Rust in `xtask`, the practice-pack builder in `tools/packs` (a workspace of its own outside the gate: `cargo test --manifest-path tools/packs/Cargo.toml` when it changes).
- Bash is for cargo, git and running programs only.

## Three verification tiers, in order

| Tier | Command | When |
| --- | --- | --- |
| 1 | `cargo check -p <crate>` | after every coherent edit; seconds |
| 2 | `cargo test -p <crate> [filter]` | the tests this slice touches |
| 3 | `cargo gate` | once before the commit: fmt, clippy, all tests (every test binary at once, `xtask/src/test_run.rs`), repo-check; about 2.5 min |

- Scenarios: `cargo scenario <name>` replays one; `cargo scenarios` replays all, each in a process of its own, as many at once as the machine has threads (`--jobs N`): about 3.5 min for 82, one line each in name order.
- A refactor of code that stays takes its baseline first: `cargo scenarios` at HEAD, `artifacts/scenarios` copied aside, compared at the end in one command (`git diff --no-index --stat`). A change in a pure crate that nothing shown reaches yet runs the gate and the scenarios its code can reach, not the suite. On the dev machine the suite runs with `--jobs 8`: twelve at once lose practice scenarios to memory.
- Start `cargo scenarios` in the background as soon as the code is done, and write the documents while it runs. Run the gate and the suite alone: each uses every core, and anything heavy beside them (another test run, clippy) makes tests time out or run out of memory (S104).
- Tests wait for a worker, a fill or motion by wall time, a hang guard of two minutes, never by a count of frames; every gesture whose meaning depends on time (a hold, a drop, a tap, QuickShape) runs on the held clock (`fix_clock`), its samples stamped on it: the runtime tests' `Hand` (`support/hand.rs`, `frames.rs`) and the scenario runner hold it, frames waiting for a worker at the same time (S106).
- A fix after review reruns the scenarios it can reach plus the gate; the whole suite only when shared drawing or layout code changed.
- When tier 3 fails, go back to tier 1 to fix; do not rerun tier 3 repeatedly.
- Read the exit code. Never judge a pass from gate output piped through `tail` or `grep`.
- Fix clippy findings in code; no `#[allow]`. A justified exception is `#[expect(lint, reason = "...")]`.
- Device verification is a separate tier, done only on the slices ROADMAP marks. If it did not run, STATUS says `not_run`, never "pass".
- Scenario steps name the controls they touch (`{ "tap": { "at": "Undo" } }`, see `crates/devtools/src/target.rs`); only places on the canvas are pixels. The runner fails a step that touches a control by pixels and prints the name to use. A control's name is its English in either language (D100); a scenario runs in English unless it sets `"system_language": "chinese"`.
- Every word the interface shows goes in the UI's text tables (`crates/ui/src/text/`), English and Chinese together; `cargo repo-check` refuses words written elsewhere.

## Device chores: these commands only, never hand-typed adb

- `cargo device-idle [--minutes N]`: whether the tablet is free, read-only: the screen's state, the app in front, the minutes since the last touch, pen or key. Exit 0 when the screen is off or nothing touched it for 10 minutes (N), 1 otherwise. Every device routine starts with it instead of asking the author: proceed on 0; on 1 wait about 5 minutes and probe again. The agent's own injected gestures count as touches.
- `cargo device-backup [name]`: the author's library and practice packs to `artifacts/device/backups/<name>.tar`, listed. Always first after `device-idle`.
- `cargo device-check [label] [--no-install]`: wake, build and install, launch, screenshot to `artifacts/device/<label>.png`, the app's logcat errors; fails on a panic.
- `cargo device-input pen x,y,... [pressure]` | `finger x,y[,x,y] [x,y[,x,y] [ms]]` | `hover x,y [ms]`: one injected gesture (debug build), screen woken first.
- `cargo device-restore <name>`: the library and packs as found saved first to `backups/<name>-found-<time>.tar` (listed), then the backup put back in place (a pack the app kept after the backup removed by its exact path) and compared byte for byte. Always last, after removing test artworks in the gallery.
- `cargo device-clean`: the agent's test files (`/sdcard/{Pictures,Download}/openpaint-test`, `/sdcard/openpaint-*.mp4`), nothing else.
- `cargo device-reset`: the app's data cleared (`pm clear`), every file it held listed; refuses unless a backup was written the same day (UTC). Only where a slice says so.
- `cargo device-smoke`: the installed debug build in about 20 s, one line per step: a test artwork it makes, a stroke of each tool, undo, a layer, a fill, Share PNG, reopen, its deletion, the app's log; never another artwork. Between backup and restore (it changes the toolbox).
- `cargo test-art pile|a4|a4-plain|kinds|session [n]`: a test artwork as an OpenPaint file under `artifacts/test-art/`, the same bytes every run; `cargo device-push <file>` puts it (or a picture, or a `.pack`) into the test folder; `cargo device-push --download <file>` into `/sdcard/Download/` itself, for the author to pick (a pack left for them; no chore removes it).
- `cargo device-input open-file <name>`: that pushed file imported and opened from the gallery as a new test artwork, its id printed; never draw test artworks by injection. Delete it by taps at the end. `<name>.pack`: that pushed practice pack imported as Import practice pack… would (the restore removes it).
- `cargo device-input practice-test start [lesson <skill> [level] | first | copy | pomodoro [seconds]] | end`: `start` leaves any sheet for the home and starts a test lesson beside the author's: bare, the lesson Start would start next; `lesson <skill> [level]` that skill's (its key in a-z and '-': `line`, `points-in-space`…; level 1 to 3, else the learner's); `first` the first lesson; `copy` a long colour copy of a pack's picture; `pomodoro [seconds]` the next lesson with a Pomodoro's block of that many seconds (60 by default; refused while a block is under way), to see the time's end, End and the block's end within a minute; its name kept in `artifacts/device/practice-test` (refused while one is kept); `end` removes that lesson or copy, its record and sheet, the test sheets the chain laid after it and the blocks begun since, and nothing else. Practice slices walk only this one, never the author's lessons or copies; end it before the restore.
- `cargo device-screens [n]`: the tablet's newest n screenshots (5 by default; `Pictures/Screenshots` and `DCIM/Screenshots`) copied to `artifacts/device/screens/` and listed with their times, nothing on the tablet changed; look at the author's only as far as the slice asks.
- `cargo device-frames [since] [--lines] | --clear`: the debug build's frames in real use (a line a stretch of motion in the app's `files/frames.txt`) read and summed up by hand and by screen, nothing changed; `since` a local `YYYY-MM-DD[THH:MM]`; `--clear` empties the file.
- `cargo device-log [n]`: the app's own last n log lines (its tag, every level; 40 by default), read-only, nothing on the tablet changed.
- `cargo device-input bench open-cold|open|zoom|pan|strokes|session|export|touch|undo|adjust|ui|practice[@fast]`: a scenario on the artwork open, its times and counts (`touch`: a hand arriving mid-fill, event to its first frame; `ui`: each popover, the colour wheel, a slider and the editor's column, frames drawn against the vsyncs wanting one; `practice`: on a practice sheet at a line) (paced as a person sees them; `@fast` as fast as the GPU takes frames); `home` presses home without instrumentation.

## Commits

- Conventional Commits, scope is the crate name, slice id at the end: `feat(core): artwork value type (S2)`.
- One commit a part and one for the integration, the package's id with the part's letter (`S122b`); a slice outside a package is one commit. Stage file by file and review `git diff --cached --stat` before committing; never `git add -A`.
- No commit while the gate fails. Push, rebase and amend only on the author's explicit instruction.
- The version moves when a build goes to the author, never for a slice (author, 2026-11-09; D79).

## Documents: four files, hard limits, enforced by repo-check

| File | Limit | Contains | Never contains |
| --- | --- | --- | --- |
| docs/STATUS.md | 150 lines, 40,000 characters | current facts: working features, each said once and briefly; each verification's latest result with its date; known limits; not_run items; decisions pending from the author, in full | history, process, plans, earlier results |
| docs/ROADMAP.md | 250 lines, 40,000 characters | product definition, stages, each slice's goal and done criteria; finished slices as one line each, at most about 300 characters: id, date, version, the gist, its decision | implementation detail, results |
| docs/ARCHITECTURE.md | 300 lines, 69,000 characters | crate map, dependency direction, core types, threading and error model, enforced rules | migration plans, audit findings |
| docs/DECISIONS.md | 300 lines, 89,000 characters | decisions that constrain future code and are not self-evident; each at most 15 lines: date, decision, reason, consequences | research reports, long option comparisons |

- No line of the four documents is longer than 1,500 characters: one fact a line, never a page. `cargo repo-check` refuses a document past its lines or characters and a line past 1,500, naming the file, the line and the limit (`xtask/src/check/rules.rs`, S105).
- No other markdown in the repository (README.md at most 60 lines and this file at most 150 lines excepted). No CHANGELOG: history is `git log`. No ADR directory, no research directory, no local notes directory.
- At the end of a package, in its integration (or of a slice outside one, or of a feature an agent builds alone, which writes its own lines): rewrite STATUS as the new current facts (delete stale sentences, do not append); move the package to "Done" in ROADMAP as one line; touch DECISIONS only when a new decision was made. A part writes no document: it hands over its lines.
- Archiving: when a document exceeds its limit, delete useless content first; if it still exceeds, move finished slices or superseded decisions wholesale to `docs/archive/<name>-<YYYY-MM>.md`. The archive is written, never read, and has no limit.
- Research, audits and option comparisons are not written as files. Conclusions go to DECISIONS (at most 15 lines) or the commit message; the process stays in the session.

## Where output goes

Agent output belongs in `crates/` product code and tests. The following count as waste and are not done by default: modules or abstractions reserved for future features, generic plugin systems, cargo features in product crates, second implementation paths or silent fallbacks, scripts, reports, notes, one record per step.

- Design from the best end state (author, 2026-10-16): code and module structure follow the design as it should be, never bent by history or compatibility; restructure when a new design wants it; what a slice replaces goes in that slice or in the cleanup slice ROADMAP names; a reader reads only the newest shape of what it stores; a changed shape is converted once, on the one library and the packs that exist, by a step in an interim build proved on the newest backup, and that step is deleted by the next slice (S116); a library or file of another format is refused, never read.
- Reasoning effort is set by the author at session start: high for design and trade-offs, low for splitting files, updating call sites and fixing compile errors.
- At most 3 subagents at a time (author, 2026-10-22), no nesting, no reviewer swarms; a package's parts work in their own worktrees on files of their own. One heavy run at a time on the dev machine (the suite, the gate, a device build). Do in the main session what can be done there. Builders run on the cheaper model; a hard design may use the main session's (author, 2026-11-09).
- Structure stays a module's own (author, 2026-11-07): before a package's contract, the main session reads the last footprints; when one feature spreads over more than three crates or a central list every feature must edit keeps growing, a structural slice goes into ROADMAP before more features. Rules that can be checked go into repo-check, not into reviews.
- A file past its lines is split by ownership into a folder (`mod.rs` and its parts), never into `#[path]` siblings named by a prefix: a size limit is no design. Those there are move when their crate is next touched.
- Code rules (enforced by repo-check and clippy): .rs files at most 400 lines; functions at most 60 lines; dependencies only point down (see ARCHITECTURE); core never mentions GPU, UI, file system or threads; `Result<_, String>` only at platform boundaries and in xtask; no `cfg(feature)` in product crates; every crate root has `#![forbid(unsafe_code)]` (JNI in `apps/android` excepted, and the harness, which denies it but for its counting allocator, S51).

## Design from the user inward

A feature is an interaction first and code second. For every package or slice that changes what the user sees or does:

1. Before writing code, write the walk-through as the user: goal, entry point, gestures, what appears, success, failure, cancel, undo, what is there after reopen. Put it in the slice's scenario file (`tests/scenarios/<slice>.json`, from S5 on) and in the commit message; never in a document.
2. Derive the intents and the projection from the walk-through, then the core commands, then rendering. Do not build the function first and attach a UI afterwards.
3. Finish by replaying the walk-through through the harness, looking at the screenshots it writes, and checking the UI checklist in ARCHITECTURE section 10. A slice whose walk-through is not a passing scenario is not done.
4. In the handover, tell the author in five lines what to try on the tablet.

## Handover note

At the end, state: what was done, what was verified (commands and results), what was not done, what the author must decide, and the footprint: the crates and files touched, the central places every such feature must edit, any structural friction met. Do not restate the documents.
