# Decisions

Only decisions that constrain future code and are not self-evident. Each entry is at most 15 lines: date, decision, reason, consequences. A superseded decision moves to `docs/archive/` as a whole; it is never rewritten in place. "The previous project" is Draweave (2026-07 to 2026-10), the predecessor this repository replaces.

## D01 2026-10-02 Stack: Rust 2024, wgpu, egui, one workspace

- Decision: core, rendering, application, storage and UI are all Rust; GPU through wgpu; UI through egui; Android is the product platform, desktop is a development host only.
- Reason: the previous project proved this stack on the target tablet (pressure, palm rejection, multi-layer compositing, smudge). Switching stacks would discard verified experience.
- Consequences: renderer state never lives in egui's type map; egui appears only under `render/src/adapter/`. The previous project paid heavily for ignoring this.

## D02 2026-10-02 Crate layering and purity are tool-enforced

- Decision: `core <- app <- {store, ui} <- runtime <- {desktop, android}` and `core <- render <- ui`. core never mentions GPU, UI, file system, threads or clocks. The tables live in `xtask/src/check/rules.rs`.
- Reason: the previous project had the right dependency direction but grew huge files and duplicated structures inside each layer. The direction is guarded by tooling; the inside is guarded by file length.
- Consequences: a new crate is registered in `LAYERS` first; an upward dependency fails the gate.

## D03 2026-10-02 Files at most 400 lines, functions at most 60 lines, from the first commit

- Decision: `cargo repo-check` rejects .rs files over 400 lines; clippy `too_many_lines` at 60 is an error.
- Reason: the previous project had eight files over 1,900 lines before its big refactor; splitting them took a full day and rewrote 30,000 lines. Agents read big files in sed slices and re-read the same file over a hundred times. Small files make a whole-file Read cheap and edit anchors short.
- Consequences: tests are split by topic too; long functions become named step functions.

## D04 2026-10-02 Four documents with hard limits; overflow is archived

- Decision: STATUS 150, ROADMAP 250, ARCHITECTURE 300, DECISIONS 300 lines; no other markdown (README and AGENTS excepted); overflow moves to `docs/archive/`, which is written and never read.
- Reason: the previous project wrote 55 ADRs, 53 research notes and one record per milestone, plus 2,089 local notes outside the repo. Documents produced more characters than Rust, and most were never read again.
- Consequences: research and audit conclusions exist only as a decision of at most 15 lines or as a commit message.

## D05 2026-10-02 Tooling is Rust only; no scripts in the repository

- Decision: tooling lives in `xtask`; .py / .sh / .ps1 are never tracked; agents do not edit code through scripts.
- Reason: in the previous project agents wrote 391 Python replacement scripts (2.1 million characters) of which only a third was new code, the rest anchors and boilerplate; the local directory accumulated 9,818 .py files.
- Consequences: code changes go through Edit / Write only; mechanical bulk changes are made file by file after splitting.

## D06 2026-10-02 Three verification tiers; desktop regression first, device smoke second

- Decision: `check -p` -> `test -p` -> `cargo gate` (once before the commit). Automated regression runs as headless desktop scenarios (seconds); the device runs one smoke scenario of at most 20 seconds; full device scenarios and manual feel checks happen only at stage exits.
- Reason: the previous project ran its 50-second full gate 207 times as a routine check with a 48% build failure rate; a quarter of machine time was spent waiting for cargo. Its 180-step device scenario took 199 seconds and passed 2 of its first 11 runs.
- Consequences: GPU tests run once per gate; without device evidence STATUS says `not_run`.

## D07 2026-10-02 Brushes are continuous recipes, not stamps and not node graphs

- Decision: a brush is a declarative recipe (TOML: shape, pressure and speed curves, grain, dynamics) evaluated continuously per pixel along the stroke. No dab/stamp engine, no node graph, no shader plugins.
- Reason: the author tried a node graph in ZPaint's Brush Graph; it ended chaotic and non-uniform: the graph carried no brush identity, parameters meant different things per render path, several accumulation laws, mixed coordinate spaces, CPU/GPU double evaluation. Stamp engines make quality depend on spacing tricks.
- Consequences: the recipe format has fixed stages with typed fields; a stroke stores a recipe snapshot, so reopening never silently applies a renamed brush's new behaviour.

## D08 2026-10-02 The artwork is an immutable value; undo is a version switch

- Decision: `Artwork(Arc<State>)`; an edit returns a new version plus the kind of change; history stores versions trimmed by a byte budget; `from_storage` is the single full-validation entry; no public mutable setters.
- Reason: the previous project kept mutable layers next to three snapshot structures, which copied the active stroke every frame, built whole snapshots to read one field and deep-compared structural edits. The value form made undo faster and cut per-frame allocation from 239 KB to 12.5 KB.
- Consequences: the renderer detects change by identity; the in-progress stroke travels to the renderer as a separate item and never enters the artwork.

## D09 2026-10-02 Persistence: embedded KV (fjall), integer schema, pure-data migrations, frozen samples

- Decision: the store uses fjall for the artwork library; the `Project` DTO carries an integer schema version; every version has one pure-data migration step and one frozen sample, and the gate round-trips every sample; editor state is stored separately and falls back to defaults when corrupt.
- Reason: fjall was verified on the previous project's Android device (it needs Rust 1.98 file locking); editor state drove 3 of its 7 migrations, so separating it keeps the artwork format stable.
- Consequences: a slice that changes the format ships the migration step and sample; an old APK installing is not the same as an old build opening a new project, and releases say so.

## D10 2026-10-02 Errors are types; product crates have no cargo features

- Decision: `CoreError`, `AppError`, `PortError`, `RenderError`, `RuntimeError`; `Result<_, String>` only in JNI, system APIs and xtask. Product crates have no `cfg(feature)`; diagnostics, experiments and automation probes live in separate crates assembled only by composition roots.
- Reason: in the previous project `String` errors ran through renderer, runtime and xtask, 379 `cfg` sites stitched diagnostics into product code, 47% of the runtime crate was diagnostics, and `--all-features` produced four build combinations.
- Consequences: `cargo repo-check` rejects `[features]` and `cfg(feature`; the platform layer translates typed errors into user-facing messages.
