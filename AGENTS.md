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

## Reading and editing

- Read source with Read; search with Grep / Glob. Do not read source through cat, sed or grep commands.
- Edit source with Edit; use Write for new files or full rewrites. Never edit through Python, sed or heredoc scripts.
- No .py / .sh / .ps1 files in the repository; tooling is Rust in `xtask`. `cargo repo-check` rejects them.
- Bash is for cargo, git and running programs only.

## Three verification tiers, in order

| Tier | Command | When |
| --- | --- | --- |
| 1 | `cargo check -p <crate>` | after every coherent edit; seconds |
| 2 | `cargo test -p <crate> [filter]` | the tests this slice touches |
| 3 | `cargo gate` | once before the commit: fmt, clippy, all tests, repo-check |

- Scenarios: `cargo scenario <name>` replays one; `cargo scenarios` replays all in one process (about 2 min), one line each.
- When tier 3 fails, go back to tier 1 to fix; do not rerun tier 3 repeatedly.
- Read the exit code. Never judge a pass from gate output piped through `tail` or `grep`.
- Fix clippy findings in code; no `#[allow]`. A justified exception is `#[expect(lint, reason = "...")]`.
- Device verification is a separate tier, done only on the slices ROADMAP marks. If it did not run, STATUS says `not_run`, never "pass".
- Scenario steps name the controls they touch (`{ "tap": { "at": "Undo" } }`, see `crates/devtools/src/target.rs`); only places on the canvas are pixels. The runner fails a step that touches a control by pixels and prints the name to use.

## Device chores: these commands only, never hand-typed adb

- `cargo device-backup [name]`: the author's library to `artifacts/device/backups/<name>.tar`, listed. Always first.
- `cargo device-check [label] [--no-install]`: wake, build and install, launch, screenshot to `artifacts/device/<label>.png`, the app's logcat errors; fails on a panic.
- `cargo device-input pen x,y,... [pressure]` | `finger x,y[,x,y] [x,y[,x,y] [ms]]` | `hover x,y [ms]`: one injected gesture (debug build), screen woken first.
- `cargo device-restore <name>`: the backup put back in place and compared byte for byte. Always last, after removing test artworks in the gallery.
- `cargo device-clean`: the agent's test files (`/sdcard/{Pictures,Download}/openpaint-test`, `/sdcard/openpaint-*.mp4`), nothing else.

## Commits

- Conventional Commits, scope is the crate name, slice id at the end: `feat(core): artwork value type (S2)`.
- Usually one commit per slice. Stage file by file and review `git diff --cached --stat` before committing; never `git add -A`.
- No commit while the gate fails. Push, rebase and amend only on the author's explicit instruction.

## Documents: four files, hard limits, enforced by repo-check

| File | Limit | Contains | Never contains |
| --- | --- | --- | --- |
| docs/STATUS.md | 150 lines | current facts: working features, verification results with dates, known limits, not_run items, decisions pending from the author | history, process, plans |
| docs/ROADMAP.md | 250 lines | product definition, stages, each slice's goal and done criteria; finished slices as one line each | implementation detail, results |
| docs/ARCHITECTURE.md | 300 lines | crate map, dependency direction, core types, threading and error model, enforced rules | migration plans, audit findings |
| docs/DECISIONS.md | 300 lines | decisions that constrain future code and are not self-evident; each at most 15 lines: date, decision, reason, consequences | research reports, long option comparisons |

- No other markdown in the repository (README.md at most 60 lines and this file at most 150 lines excepted). No CHANGELOG: history is `git log`. No ADR directory, no research directory, no local notes directory.
- At the end of a slice: rewrite STATUS as the new current facts (delete stale sentences, do not append); move the slice to "Done" in ROADMAP as one line; touch DECISIONS only when a new decision was made.
- Archiving: when a document exceeds its limit, delete useless content first; if it still exceeds, move finished slices or superseded decisions wholesale to `docs/archive/<name>-<YYYY-MM>.md`. The archive is written, never read, and has no limit.
- Research, audits and option comparisons are not written as files. Conclusions go to DECISIONS (at most 15 lines) or the commit message; the process stays in the session.

## Where output goes

Agent output belongs in `crates/` product code and tests. The following count as waste and are not done by default: modules or abstractions reserved for future features, generic plugin systems, cargo features in product crates, second implementation paths or silent fallbacks, scripts, reports, notes, one record per step.

- Reasoning effort is set by the author at session start: high for design and trade-offs, low for splitting files, updating call sites and fixing compile errors.
- At most 2 subagents, no nesting, no reviewer swarms. Do in the main session what can be done there.
- Code rules (enforced by repo-check and clippy): .rs files at most 400 lines; functions at most 60 lines; dependencies only point down (see ARCHITECTURE); core never mentions GPU, UI, file system or threads; `Result<_, String>` only at platform boundaries and in xtask; no `cfg(feature)` in product crates; every crate root has `#![forbid(unsafe_code)]` (JNI in `apps/android` excepted).

## Design from the user inward

A feature is an interaction first and code second. For every slice that changes what the user sees or does:

1. Before writing code, write the walk-through as the user: goal, entry point, gestures, what appears, success, failure, cancel, undo, what is there after reopen. Put it in the slice's scenario file (`tests/scenarios/<slice>.json`, from S5 on) and in the commit message; never in a document.
2. Derive the intents and the projection from the walk-through, then the core commands, then rendering. Do not build the function first and attach a UI afterwards.
3. Finish by replaying the walk-through through the harness, looking at the screenshots it writes, and checking the UI checklist in ARCHITECTURE section 10. A slice whose walk-through is not a passing scenario is not done.
4. In the handover, tell the author in five lines what to try on the tablet.

## Handover note

At the end, state: what was done, what was verified (commands and results), what was not done, what the author must decide. Do not restate the documents.
