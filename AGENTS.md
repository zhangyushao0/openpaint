# OpenPaint repository rules

This file is the shared entry point for agents and maintainers and sets the default way of working. Explicit instructions from the author in the current task take precedence.

## Reading order

1. `docs/STATUS.md`: what works now and what has been verified.
2. `docs/ROADMAP.md`: find the next slice; do only that slice.
3. `docs/ARCHITECTURE.md`: only the sections for the crates this slice touches.
4. The affected sources. Every file is under 400 lines: read whole files with Read, locate with Grep.
5. `docs/DECISIONS.md`: only when changing an existing decision or when you need the "why".

Do not rebuild the big picture from source; the module map is in ARCHITECTURE. Do not dig through git history for progress; progress is in STATUS.

## One session, one slice

- A session does exactly one slice from ROADMAP. When the gate passes, the commit exists and the documents are updated, the session ends. Do not start the next slice.
- Run `git status --short --branch` first; never touch pre-existing uncommitted changes.
- ROADMAP defines the scope. Problems outside it go to STATUS "Known limits" or ROADMAP "Candidates"; do not fix them in passing.
- Stopping to ask the author is fine. Finish the parts that do not depend on the answer first, then list concrete options with a recommendation.
- A choice that only makes the interface consistent with what the author asked for is made, not listed as a decision for them.

## Reading and editing

- Read source with Read; search with Grep / Glob. Do not read source through cat, sed or grep commands.
- Edit source with Edit; use Write for new files or full rewrites. `sed` is fine where it is quicker (one-line changes, renames, hash replacements; author, 2026-10-04, after a trial in S30); no Python or heredoc scripts.
- Deletes name every path exactly (author, 2026-11-07): many at once is fine when each path is spelt out, from a listing looked at first or a tool that computes and prints them, or one named folder by its quoted absolute path; never shell wildcards or globs, never `cargo clean` or `git clean`, and never anything under `artifacts/device/backups/`.
- No .py / .sh / .ps1 files in the repository; tooling is Rust in `xtask`. `cargo repo-check` rejects them.
- Bash is for cargo, git and running programs only.

## Three verification tiers, in order

| Tier | Command | When |
| --- | --- | --- |
| 1 | `cargo check -p <crate>` | after every coherent edit; seconds |
| 2 | `cargo test -p <crate> [filter]` | the tests this slice touches |
| 3 | `cargo gate` | once before the commit: fmt, clippy, all tests (every test binary at once, `xtask/src/test_run.rs`), repo-check; about 2.5 min |

- Scenarios: `cargo scenario <name>` replays one; `cargo scenarios` replays all, each in a process of its own, as many at once as the machine has threads (`--jobs N`): about 3 min for 72, one line each in name order.
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
- `cargo device-input practice-test start [stage [n]|copy]|end`: `start` leaves any sheet for the gallery and starts the practice session Today would start next (of `stage` when given, as its n-th session with `n`; `copy`: a long colour copy of a pack's picture), beside the author's, its name kept in `artifacts/device/practice-test` (refused while one is kept); `end` removes that session or copy, its record and sheet, and nothing else. Practice slices walk only this one, never the author's sessions or copies; end it before the restore.
- `cargo device-input practice-test start drill <kind>`: a test drill of that kind (`line`, `centre`, `next-face`…, as its tile's English in a-z and '-') beside the author's, kept and ended as a test session is (`practice-test end`).
- `cargo device-screens [n]`: the tablet's newest n screenshots (5 by default; `Pictures/Screenshots` and `DCIM/Screenshots`) copied to `artifacts/device/screens/` and listed with their times, nothing on the tablet changed; look at the author's only as far as the slice asks.
- `cargo device-input bench open-cold|open|zoom|pan|strokes|session|export|touch|undo|adjust|ui|practice[@fast]`: a scenario on the artwork open, its times and counts (`touch`: a hand arriving mid-fill, event to its first frame; `ui`: each popover, the colour wheel, a slider and the editor's column, frames drawn against the vsyncs wanting one; `practice`: on a practice sheet at a line) (paced as a person sees them; `@fast` as fast as the GPU takes frames); `home` presses home without instrumentation.

## Commits

- Conventional Commits, scope is the crate name, slice id at the end: `feat(core): artwork value type (S2)`.
- Usually one commit per slice. Stage file by file and review `git diff --cached --stat` before committing; never `git add -A`.
- No commit while the gate fails. Push, rebase and amend only on the author's explicit instruction.

## Documents: four files, hard limits, enforced by repo-check

| File | Limit | Contains | Never contains |
| --- | --- | --- | --- |
| docs/STATUS.md | 150 lines, 40,000 characters | current facts: working features, each said once and briefly; each verification's latest result with its date; known limits; not_run items; decisions pending from the author, in full | history, process, plans, earlier results |
| docs/ROADMAP.md | 250 lines, 40,000 characters | product definition, stages, each slice's goal and done criteria; finished slices as one line each, at most about 300 characters: id, date, version, the gist, its decision | implementation detail, results |
| docs/ARCHITECTURE.md | 300 lines, 69,000 characters | crate map, dependency direction, core types, threading and error model, enforced rules | migration plans, audit findings |
| docs/DECISIONS.md | 300 lines, 89,000 characters | decisions that constrain future code and are not self-evident; each at most 15 lines: date, decision, reason, consequences | research reports, long option comparisons |

- No line of the four documents is longer than 1,500 characters: one fact a line, never a page. `cargo repo-check` refuses a document past its lines or characters and a line past 1,500, naming the file, the line and the limit (`xtask/src/check/rules.rs`, S105).
- No other markdown in the repository (README.md at most 60 lines and this file at most 150 lines excepted). No CHANGELOG: history is `git log`. No ADR directory, no research directory, no local notes directory.
- At the end of a slice: rewrite STATUS as the new current facts (delete stale sentences, do not append); move the slice to "Done" in ROADMAP as one line; touch DECISIONS only when a new decision was made.
- Archiving: when a document exceeds its limit, delete useless content first; if it still exceeds, move finished slices or superseded decisions wholesale to `docs/archive/<name>-<YYYY-MM>.md`. The archive is written, never read, and has no limit.
- Research, audits and option comparisons are not written as files. Conclusions go to DECISIONS (at most 15 lines) or the commit message; the process stays in the session.

## Where output goes

Agent output belongs in `crates/` product code and tests. The following count as waste and are not done by default: modules or abstractions reserved for future features, generic plugin systems, cargo features in product crates, second implementation paths or silent fallbacks, scripts, reports, notes, one record per step.

- Design from the best end state (author, 2026-10-16): code and module structure follow the design as it should be, never bent by history or compatibility; restructure when a new design wants it; what a slice replaces goes in that slice or in the cleanup slice ROADMAP names; old data shapes are converted only by the store's upgrade steps, nothing else in the code handles them.
- Reasoning effort is set by the author at session start: high for design and trade-offs, low for splitting files, updating call sites and fixing compile errors.
- At most 3 subagents at a time (author, 2026-10-22), no nesting, no reviewer swarms; parallel slices work in their own worktrees on disjoint crates, and the main session merges them and keeps the four documents. Do in the main session what can be done there.
- Structure stays a module's own (author, 2026-11-07): before dispatching a slice, the main session reads the last slices' footprints; when one feature spreads over more than three crates or a central list every feature must edit keeps growing, a structural slice goes into ROADMAP before more features. Rules that can be checked go into repo-check, not into reviews.
- Code rules (enforced by repo-check and clippy): .rs files at most 400 lines; functions at most 60 lines; dependencies only point down (see ARCHITECTURE); core never mentions GPU, UI, file system or threads; `Result<_, String>` only at platform boundaries and in xtask; no `cfg(feature)` in product crates; every crate root has `#![forbid(unsafe_code)]` (JNI in `apps/android` excepted, and the harness, which denies it but for its counting allocator, S51).

## Design from the user inward

A feature is an interaction first and code second. For every slice that changes what the user sees or does:

1. Before writing code, write the walk-through as the user: goal, entry point, gestures, what appears, success, failure, cancel, undo, what is there after reopen. Put it in the slice's scenario file (`tests/scenarios/<slice>.json`, from S5 on) and in the commit message; never in a document.
2. Derive the intents and the projection from the walk-through, then the core commands, then rendering. Do not build the function first and attach a UI afterwards.
3. Finish by replaying the walk-through through the harness, looking at the screenshots it writes, and checking the UI checklist in ARCHITECTURE section 10. A slice whose walk-through is not a passing scenario is not done.
4. In the handover, tell the author in five lines what to try on the tablet.

## Handover note

At the end, state: what was done, what was verified (commands and results), what was not done, what the author must decide, and the footprint: the crates and files touched, the central places every such feature must edit, any structural friction met. Do not restate the documents.
