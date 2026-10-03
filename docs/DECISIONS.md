# Decisions

Only decisions that constrain future code and are not self-evident. Each entry is at most 15 lines: date, decision, reason, consequences. A superseded decision moves to `docs/archive/` as a whole; it is never rewritten in place. "The previous project" is Draweave (2026-07 to 2026-10), the predecessor this repository replaces.

## D01 2026-10-02 Stack: Rust 2024, wgpu, egui, one workspace

- Decision: core, rendering, application, storage and UI are all Rust; GPU through wgpu; UI through egui; Android is the only product host; the development machine runs a headless harness (D12).
- Reason: the previous project proved this stack on the target tablet (pressure, palm rejection, multi-layer compositing, smudge). Switching stacks would discard verified experience.
- Consequences: renderer state never lives in egui's type map; egui appears only under `render/src/adapter/`. The previous project paid heavily for ignoring this.

## D02 2026-10-02 Crate layering and purity are tool-enforced

- Decision: `core <- app <- {store, ui} <- runtime <- {harness, android}` and `core <- render <- ui`. core never mentions GPU, UI, file system, threads or clocks. The tables live in `xtask/src/check/rules.rs`.
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

## D06 2026-10-02 Three verification tiers; harness regression first, device smoke second

- Decision: `check -p` -> `test -p` -> `cargo gate` (once before the commit). Automated regression runs as headless harness scenarios on the dev machine (seconds); the device runs one smoke scenario of at most 20 seconds; full device scenarios and manual feel checks happen only at stage exits.
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

## D11 2026-10-02 Android host: a hand-written Activity, JNI and SurfaceView; no winit

- Decision: `apps/android` is a small Kotlin Activity that owns the lifecycle, a `SurfaceView` and the system pickers. Rust receives the Surface through JNI and renders with wgpu. `MotionEvent` fields (tool type, pressure, tilt, orientation, buttons, hover, historical samples) are forwarded as typed batches. winit and `android-activity` are not used anywhere.
- Reason: winit's Android touch abstraction drops stylus detail (tool type, tilt, buttons, hover, batched history). The previous project needed all of it and ran on exactly this host shape.
- Consequences: egui gets its `RawInput` from our own normaliser on every platform; Kotlin code stays limited to lifecycle, Surface, pickers and share; product rules never live in the host.

## D12 2026-10-02 The development machine runs a headless harness, never a desktop window

- Decision: `apps/harness` is a dev-only binary: offscreen wgpu rendering with readback, recorded-input replay through the full frame loop, JSON scenarios with pixel checkpoints, PNG and hash output. No window, no mouse handling, no desktop product.
- Reason: agents need a seconds-long loop on the dev machine (the previous project's headless lane ran 180 steps in 4 s against 199 s on the tablet), but a windowed desktop app would pull output into windowing and mouse code that proves nothing about a stylus product.
- Consequences: "seeing" a result means reading the PNG the harness wrote; pen feel and real input are judged only on the tablet.

## D13 2026-10-02 The UI is designed from the user inward, with Procreate as the default model

- Decision: every slice that changes what the user sees or does starts with a walk-through written as the user and ends with that walk-through as a passing harness scenario plus a screenshot review against the checklist in ARCHITECTURE section 10. Procreate's interaction model is the default; the theme and the components are defined once.
- Reason: in the previous project features were built function-first with the UI attached afterwards. The result resembled Procreate but felt unpolished in small aesthetic and usability details, because nobody had walked through it as a user. An agent cannot feel the tablet, but it can look at screenshots, count taps and verify hit sizes, so those are made explicit and checkable.
- Consequences: the walk-through lives in the scenario file and the commit message, never in a document; the author's on-device judgement at stage exits is the only authority on feel and beauty.

## D14 2026-10-02 External crates are pinned exactly in the workspace manifest

- Decision: every external crate is declared once in `[workspace.dependencies]` with an exact `=` version; crates use `.workspace = true`. Current pins: wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2 (requires wgpu 30.0), pollster 1.0.1 (blocks on wgpu's adapter/device futures), png 0.18.1 (harness only), serde 1.0.229 and serde_json 1.0.151 (scenario files; devtools only, never core), fjall 3.1.11 (store only; MSRV 1.90, locks its directory with std file locking), toml 1.1.6 (brush files; app only, read as a table without serde derive), image 0.25.10 (store only, default features off, `png` and `jpeg`: export encoding and reference decoding), log 0.4.34, and for the Android host only jni 0.22.4, android_logger 0.15.1, raw-window-handle 0.6.2 (the version wgpu 30 uses). Gradle side, pinned in `apps/android`: Gradle 8.13 (wrapper with checksum), Android Gradle plugin 8.13.2, Kotlin 2.2.21.
- Reason: wgpu and egui break APIs every release and egui-wgpu ties them together; a silent minor bump would change GPU behaviour and golden hashes without a reviewed change.
- Consequences: wgpu, egui and egui-wgpu are upgraded together in one commit that reruns the GPU tests; golden hashes that move are re-recorded in that commit with the reason. egui and egui-wgpu are pinned before any crate uses them, so the first user inherits a known-compatible pair.

## D15 2026-10-02 Android host: main-thread frames, array-packed input, xtask-driven build

- Decision: the Kotlin activity calls Rust only from the main thread: `Choreographer` paces frames on demand (Rust answers whether it wants the next one) and `MotionEvent`s cross JNI as primitive arrays (ids, tool types, x/y/pressure/tilt/orientation per step and pointer, times), decoded into typed batches in plain Rust. The UI painter lives for the process because egui sends its font atlas once. `cargo android-build` launches the Gradle wrapper jar from xtask; no gradlew scripts are tracked. The gate clippy-checks the JNI crate for `aarch64-linux-android`.
- Reason: one thread needs no locks between lifecycle, surface and input calls; arrays keep Kotlin free of product types and make decoding testable on the dev machine; gradlew and gradlew.bat are scripts (D05); without the Android clippy step the JNI code would be compiled by nobody but the APK build.
- Consequences: a slow frame delays input; S8's latency number kept frames on the main thread (D22); surfaces must be detached in `surfaceDestroyed` before it returns; changing the sample layout means changing `SAMPLE_FIELDS` in both `motion.rs` and `MainActivity.kt`.

## D16 2026-10-02 Evaluation is a per-pixel op interpreter over tile-binned lists; strokes are coverage unions

- Decision: every pixel is computed from scratch by a compute shader that walks, in scene order, the ops whose bounds reach its 16 px tile: `Segment` takes the maximum coverage of a tapered capsule (pressure sets the radius), `EndStroke` paints colour x coverage x opacity over the layer once, `EndLayer` composites with layer opacity. Premultiplied, sRGB-encoded blending; the shader rounds to 8 bits itself and stores a value that every unorm rounding mode writes back unchanged.
- Reason: a pixel's value then depends only on the ops that reach it, never on which regions are evaluated together, so incremental equals cold by construction rather than by care. Union coverage gives a continuous stroke with one opacity and no seams or density bumps at sample joints (D07). Explicit rounding gave the same golden hash on Vulkan and DX12.
- Consequences: an op must leave a pixel bit-exactly unchanged outside its bounds (zero coverage, empty layer), and every later op (image, fill, smudge) must keep that property; per-pixel cost grows with the ops per tile, so caches and bounded batching come before long strokes get slow (S4's 8 ms budget).

## D17 2026-10-02 The canvas target tracks the in-progress stroke; callers pass only other changes

- Decision: a `CanvasTarget` keeps the in-progress stroke's coverage so far in an `R32Float` texture. Each evaluation adds only the segments of its new samples (`EndActive`: maximum with the kept texel, stored back, then painted like `EndStroke`) and evaluates only the tiles they reach. The target remembers which stroke it holds (layer, style, samples) and the artwork version it was drawn over: a stroke that disappears while that version stays was dropped and is erased; one committed into a new version only gets the samples it gained since its last evaluation drawn. A lone dot that grows starts its coverage over, because later segments replace a dot rather than join it.
- Reason: rebuilding every segment of a 10,000-sample stroke each frame made frame cost grow with stroke length. A maximum is exact in floats, so the kept coverage equals the coverage over all segments: incremental equals cold by construction (D16), proven by GPU tests and by the 10,000-sample replay.
- Consequences: callers pass regions only for artwork changes other than committing the drawn stroke (S6 undo, layer edits). A brush whose stroke coverage is not a maximum (smudge, wet media) needs its own kept state and its own exactness proof. Per-frame GPU resources (bind groups, buffers) are kept on the target and rewritten in place.

## D18 2026-10-02 Only the stylus paints; touch is reserved for gestures

- Decision: a finger never draws. Pen contacts paint; touch contacts are reserved for gestures (two-finger tap undo, pinch zoom and the like). Decided by the author.
- Reason: the author's choice; it follows Procreate-style gesture use on a stylus tablet, where touch drives the canvas and the pen draws.
- Consequences: gesture slices map touch to gestures; palm rejection (S8) can treat touch as non-painting. A "finger paints" option, if ever wanted, is a separate later decision. S4 already behaves this way.

## D19 2026-10-02 Scenario checkpoints compare canvas hashes; references stay untracked

- Decision: a scenario checkpoint passes when the canvas readback (artwork pixels, no UI) hashes to the value in the scenario file. Screenshots of the whole screen are written for looking at and never compared. The diff reference is the canvas of the checkpoint's last pass on this machine (`artifacts/scenarios/<name>/<checkpoint>.canvas.png`, untracked); no reference images are committed.
- Reason: the evaluator's pixels are identical on Vulkan and DX12 (D16), egui's text edges are not (S4), so screen hashes would fail by backend. A hash is one reviewable line in the scenario; PNGs in the repository would duplicate it and grow with every slice.
- Consequences: a checkpoint that fails on a fresh clone has no diff, only the actual canvas; a slice that changes pixels on purpose updates the hash in the same commit after looking at the PNG. UI changes are verified by looking at screenshots and by GPU tests on specific pixels, not by scenario hashes.

## D20 2026-10-02 History: versions costed by unshared bytes; no history step during a stroke

- Decision: every artwork change that reaches the session is recorded in `app::History` as a version; a step costs `Artwork::bytes_since(previous)` (memory not shared with the version before); recording drops the redo side and trims the oldest steps while they exceed 64 MiB, always keeping the newest step. Undo and redo are unavailable (projection `StrokeInProgress`, intent refused) while a stroke is being drawn. After a history switch the runtime passes `render::changed_regions(shown, current)` to the evaluator.
- Reason: versions share almost everything (D08), so counting whole versions would trim far too early, and counting only samples would ignore the per-version chunk copies. Allowing undo mid-stroke would either drop the stroke or land it on a version the user no longer sees; refusing it cannot lose data.
- Consequences: new edit kinds (layers in S9, region edits) go through `History::record` and are costed by `bytes_since` without new code; `changed_regions` must learn a cheaper answer than "whole canvas" for any change kind that becomes frequent. A UI that offers history must read availability from the projection.

## D21 2026-10-02 Schema 1: one record per stroke, hand-written little-endian values, saves rewrite only what changed

- Decision: the library is one fjall database with keyspaces `library` (`last`, `next` project id) and `projects`: per project a head record (schema, canvas, counters, layer table with stroke counts) and one record per stroke keyed `[project][s][layer][index]` (big-endian keys). Values are fixed-width little-endian fields written by `store/src/codec.rs`, no serialization crate. A save is one atomic batch synced with `SyncData`: the head plus the stroke records after each layer's common prefix with the version the store last wrote (compared by sample pointer, id and style); after any failure the next save rewrites the project whole. The frozen sample `crates/store/tests/samples/schema-1.bin` is a dump of every record; the gate writes the sample artwork and compares bytes, and opens the sample.
- Autosave (app `storage.rs`): one save in flight on the runtime's storage worker; a failed version is not retried by itself (only after the next change, a Retry tap, or leaving the app); a library that cannot be opened starts a blank artwork in a new project and leaves the old records untouched, with a notice.
- Reason: principle 5 (pay per change): rewriting a whole artwork after each stroke would cost megabytes per stroke on the tablet's flash; per-stroke records make a stroke cost its own bytes. An owned byte layout cannot change with a dependency upgrade, and the frozen bytes prove it.
- Consequences: schema 2 (S9) adds a migration over these records and its own sample. History and editor state are not stored. Layer reorder and property changes rewrite only the head; deleting a layer removes its records.

## D22 2026-10-02 Rendering stays on the Android main thread; latency work targets presentation

- Decision: D15's open question is closed for G1: no render thread. The pen latency probe stays in the Android host and logs one summary per stroke.
- Reason: measured on the MovinkPad 14 at 120 Hz (S8, injected strokes): event to present call p50 9.2 ms, p95 10.4 ms, of which the frame's own work is p50 2.5 ms, p95 3.3 ms of an 8.3 ms vsync; the rest is waiting for the next Choreographer frame. Queue to on-screen present (SurfaceFlinger) is p50 28 ms, unchanged with `desired_maximum_frame_latency` 1 instead of 2. A render thread would remove at most the 2.5 ms of frame work from input delivery and buys nothing against the 28 ms.
- Consequences: latency work goes to presentation (front-buffered or low-latency presentation, frame timing) as its own candidate, not to threads. Revisit when frame work passes half a vsync (4 ms p95) or input handling is seen to stall.

## D23 2026-10-02 Layer panel: a popover acting on the active layer; choosing a layer and one slider touch are no steps of their own

- Decision: the Layers button at the right end of the top bar opens a popover (Procreate's entry): rows top of the stack first, the active one highlighted, an eye per row; below them the active layer's opacity slider, move up, move down and delete. Reorder is by the two buttons, not by dragging rows; delete asks nothing, undo restores the layer. A contact starting on the panel never reaches the canvas; a contact landing on the canvas outside it closes the panel, and a pen stroke there also draws. Every layer change is a history step, except: choosing the layer to draw on amends the current step (`History::amend`, redo kept), and the values of one slider touch amend the step its first change recorded (`settled` ends it).
- Reason: buttons are reliable with a finger on a list that also scrolls; drag-to-reorder needs press-and-hold and multi-pointer routing that does not exist yet. An undo that only moves the highlight would look like undo doing nothing; one undo per slider value would make a fade dozens of steps. Delete is undoable, so a modal would only slow it (ARCHITECTURE section 10).
- Consequences: D20's "every change is a version" holds with these two amendments; the projection carries the layer list (`SessionProjection` is `Clone`, not `Copy`). Dragging rows, renaming and thumbnails are later changes to this panel, not new panels.

## D24 2026-10-02 S9 keeps schema 1: layers needed no format change

- Decision: S9 ships without a schema 2. Schema 1 (D21) already stores everything S9 edits: the layer order, each layer's name, visibility, opacity and lock, and the active layer in the head; strokes keyed by layer id. The first migration step 1 -> 2 and its frozen sample land with the first slice that changes the format (S11 stores recipe snapshots). Pending the author's confirmation (STATUS).
- Reason: a schema 2 identical to schema 1 would add a migration that changes nothing; adding fields for later slices (alpha lock, blend, groups) would reserve format for features not yet accepted. Proven instead: the schema-1 sample still opens with its three layers, layer edits round-trip through the library, and the tablet's library written by the S8 build opened unchanged in the S9 build.
- Consequences: supersedes D21's line "schema 2 (S9) adds a migration over these records"; ROADMAP's S9 criterion "the schema 1 sample upgrades" is met by the unchanged sample test. D25 settles the pending question: schema 2 came with S10, the first real format change.

## D25 2026-10-02 Schema 2: a catalogue for the gallery, thumbnails as derived records

- Decision: keyspace `library` gains `schema` (the whole library's), `clock` (a change counter) and per project a catalogue entry `['a'][id]`: title and `touched` (the clock at create, duplicate or save; the gallery lists the highest first). `last` now names the artwork open when the app was left and is removed when the user leaves for the gallery, so the app reopens where it was left. Keyspace `projects` gains `[id]['t']`, the thumbnail: RGBA at most 448 x 336, each pixel the rounded mean of its canvas box, computed on the GPU and reproduced exactly by `render::thumbnail_reference`. Head and stroke layouts are unchanged; heads say 2.
- Migration 1 -> 2 (`store/src/migrate.rs`) is pure: from the heads it adds an "Untitled Artwork" entry per project touched in id order, the clock and the schema, and marks the heads 2; the library applies it in one batch when it opens a schema-1 library. `schema-1.bin` stays (it opens upgraded with every old record kept); `schema-2.bin` is the new frozen sample.
- Thumbnails are derived, not artwork: written whenever an artwork is left (for the gallery or by leaving the app), never used to rebuild it; a missing or unreadable one shows "No preview" and is rewritten on the next leave.
- Reason: the gallery needs titles, an order and pictures without reading any stroke; outside the head, a rename or a listing never touches artwork records. Rendering thumbnails when the gallery opens would evaluate every artwork each time.
- Consequences: the storage worker is a FIFO queue (`worker::Queue`, replacing `Latest`): gallery calls must never replace one another. Leaving the canvas waits for its save; if it fails the canvas stays with Retry. After a crash a thumbnail can show the version left last, not the last stroke.

## D26 2026-10-02 Text input: the host is the editor, egui's IME events carry the text

- Decision: text fields are egui `TextEdit`s. When one has the focus, egui's IME output makes the runtime report `keyboard_wanted`; the Kotlin `CanvasView` then becomes the text editor (`onCheckIsTextEditor`, an input connection with an empty buffer, no extract UI) and shows the soft keyboard. Composing text arrives as `ImeEvent::Preedit`, committed text as `Commit`, deletions as `DeleteSurrounding`, editing keys and hardware or injected keys as key events, the keyboard's Done as Enter. The back button goes to `Runtime::back` first.
- Reason: no Android `EditText` over the surface (a second UI toolkit, D11); egui's IME events already handle composition, so input methods for any script compose in place.
- Consequences: a new text field needs nothing in the host. Characters show only as far as egui's fonts have glyphs (no CJK yet, a ROADMAP candidate). Dialogs that type sit under the top bar, above the keyboard.

## D27 2026-10-02 Brush recipes: four fixed stages, data in core, TOML in app, an app-wide toolbox

- Decision: `core::BrushRecipe` is plain data: softness (0 hard, 1 fades from the centre line), size and opacity `Dynamics` (a pressure curve times a speed curve, up to 8 points, factors 0..1), and `Grain` (strength, coarse cell size). Per sample the renderer turns them into a radius and a flow (`render/src/stroke.rs`, speed = path of the last 40 ms / 40 ms); a segment's coverage is its soft-edged capsule times the flow interpolated along it, still a maximum (D16, D17); the paint op multiplies by a canvas-fixed grain computed in integers. Brush files are TOML read by `app/src/recipe.rs` into typed `RecipeError`s (no serde derive; unknown keys are errors); the built-ins live in `crates/app/brushes/`.
- The toolbox (brush, and each brush's own size and opacity) belongs to the app, not to an artwork (Procreate's model): the runtime owns `app::Toolbox` across artworks, hands its style to the open session and stores it apart (D28). Brush changes are no history steps. The side bar sits at the left edge (Procreate's default) with size, opacity, undo and redo; the brush panel is a popover like the layer panel.
- Reason: D07's fixed typed stages; recipes as core data keep core dependency-free and make the snapshot validated by `from_storage`. Ink Pen equals `BrushRecipe::plain()`, the brush before S11, so the author's approved pen feel and every older golden stayed bit for bit. Integer grain and per-segment values computed on the CPU kept Vulkan and DX12 identical.
- Consequences: a new stage is a new field, a core change and a schema change, never a plugin. Flow cannot build up within a stroke (union coverage); that needs kept state of its own. Brush editing and import go through a later `BrushStore`.

## D28 2026-10-02 Schema 3: every stroke carries its recipe; the toolbox is a separate record

- Decision: a stroke record holds its recipe after the colour (softness, four curves as count and points, grain strength and scale), so reopening never applies a changed brush (D07). `library/editor` holds the toolbox (current key, per brush key, size, opacity); an unreadable one is `Corrupt`, logged, and the brushes start at their defaults; unknown keys or values out of a brush's range fall back per brush.
- Migration 2 -> 3 (`migrate.rs`, pure) inserts the plain round brush's recipe, frozen in `migrate.rs`, into every stroke and marks heads 3; a schema-1 library runs 1 -> 2 -> 3, each step seeing the heads the one before wrote, in one batch. `schema-1.bin` and `schema-2.bin` stay as inputs (they open upgraded with every record kept); `schema-3.bin` is the frozen sample, toolbox included.
- Reason: per-stroke records (D21) keep saves paying per stroke; a recipe costs about 70 to 150 bytes against kilobytes of samples, and `from_storage` shares equal recipes again in memory.
- Consequences: an upgrade reads every stroke record into memory once. Verified on the tablet: the author's schema-2 drawing reopened pixel-identical.

## D29 2026-10-02 Schema 4: Alpha Lock in the head, a blend per stroke, a toolbox of two tools with colours

- Decision: each layer entry of a head gains Alpha Lock (u8) after its lock; each stroke record gains its blend (u8: 0 paint over, 1 paint atop, 2 erase) after its colour, before the recipe; `library/editor` becomes the tool (u8), the brush tool's choice (schema 3's whole record), the eraser's choice (an empty key is the default brush), the colour and up to 8 recent colours.
- Migration 3 -> 4 (`migrate.rs`, pure): heads re-encoded with Alpha Lock off per layer (a head that does not parse whole stays as it is), strokes given blend 0, the toolbox record kept with the brush tool current, the eraser's default choice, black and no recents appended. The library runs every step from the stored schema on, each step seeing the heads and strokes the one before wrote, in one batch. `schema-1/2/3.bin` stay as inputs and open upgraded with every record kept; `schema-4.bin` is the frozen sample (an alpha-locked layer, an atop and an eraser stroke, a toolbox with colours).
- Reason: the blend must be stored per stroke (D30); one byte keeps stroke records paying per stroke; the editor state stays apart from artworks (D09).
- Consequences: verified on the tablet: the author's schema-3 drawing reopened pixel-identical after the upgrade, its toolbox kept.

## D30 2026-10-02 Eraser, Alpha Lock, colour and eyedropper follow Procreate; a stroke keeps the blend it began with

- Decision: the eraser is a tool beside the brush using the same recipes, with its own brush and per-brush size and opacity; it takes coverage x opacity away from the active layer (destination out). Alpha Lock is a layer property, an undoable layer change in the layer panel's footer; a stroke records the lock when it begins and paints atop (colour changes only where the layer has alpha, the alpha stays). The eraser on an alpha-locked layer is refused (Procreate erases nothing there) and leaves no history step. Colours are rounded to 8 bits per channel; a colour joins the 8 recent ones when it settles (finger lifted from the wheel, a recent tapped, an eyedropper pick). The eyedropper is the side bar's button between the sliders: a contact on the canvas (pen or finger) picks the composited pixel through `CanvasTarget::pick` while it moves, the lift settles it and the tool before returns. Colour and tool changes are no history steps.
- Reason: re-evaluating strokes from scratch (D16) would let a later lock toggle change how old strokes look; capturing the blend keeps a version's pixels fixed. 8-bit colours make a wheel colour and the same colour picked from the canvas one recent colour, and a picked colour paints exactly the pixel it came from.
- Consequences: touch-and-hold eyedropper, the pen's eraser end and pen buttons are not mapped (the author's call). Picking reads back on the main thread (one texel, waits for the GPU).

## D31 2026-10-02 Export, share and the reference image: app flows, platform ports, a worker with its own target; the reference is not stored

- Decision: Procreate's Actions panel (a spanner right of Gallery) holds Save PNG/JPEG, Share PNG/JPEG, the Reference switch and Choose reference image. `app::Transfers` owns both flows for the app, not per artwork; the app never sees a path or URI: it asks the host through `HostRequest` and receives `ImageSink`/`ImageSource` ports in `HostAnswer`. Export is `render::Core::export`, a canvas target of its own evaluated whole through `RegionEvaluator` on the `openpaint-transfer` worker (an `Arc<Core>`, nothing the frame loop holds), then encoded and written there; JPEG quality 92 on the paper colour. The Android host serves the save dialog and the picker through the Storage Access Framework (detached descriptors to Rust) and shares through its own read-only `ShareProvider` over `cacheDir/share` (no AndroidX). PNG and JPEG go through the `image` crate in store (D14). The reference image is decoded on the worker, scaled to at most 2048 px, kept for the app's run and never stored: the format stays schema 4.
- Reason: a pixel never depends on which regions are evaluated with it (D16), so a separate target gives the screen's pixels exactly without touching the screen's target or stalling drawing. Ports keep the app and the harness free of Android; devtools plays the platform in scenarios. Storing the reference would add an image asset to the format, which needs the author (ARCHITECTURE section 10) and the asset store.
- Consequences: a new export format is an `ImageFormat` value plus an encoder; layered or PSD export needs its own job. One system dialog at a time (`Availability::Running`). Storing the reference with the artwork is a later format change with its own migration.

## D32 2026-10-02 The UI is paper-white line art, intuitive and restrained

- Decision: the chrome is light, near-neutral warm paper with ink-line outlines and self-drawn ink-line icons (no icon library); the white canvas sits on the paper; one deep ink colour marks the active state; no dark theme. Two rules: intuitive (a first-day tablet user needs no instructions; every control explains itself) and restrained (UI appears when needed and recedes; cues over controls, e.g. a cut-off last row and an edge fade instead of a scrollbar, an indicator only while scrolling). Hidden shortcuts such as gestures only duplicate a visible control. Supersedes "neutral dark greys for chrome" in ARCHITECTURE section 10.
- Reason: the author's direction; the dark, engineering look of Photoshop, Procreate and Clip Studio is not wanted. Near-neutral paper keeps colour judgement honest.
- Consequences: S17 applies it to every component through `ui/src/design/`; style screenshots go to the author before the full pass.

## D33 2026-10-02 Simplest design first; optimisation is coarse-grained and comes in G5; draweave is the behaviour reference

- Decision: each slice builds the simplest direct design that meets its done criteria and budgets; caches, incremental schemes and tuning beyond that are written under ROADMAP "Candidates" and done in G5, coarse-grained and measured on the tablet. Before building a feature, the slice reads draweave's implementation of the same feature (behaviour, constants, tests, fix commits) and carries its lessons over; its architecture is not copied.
- Reason: the author's direction; the predecessor spent heavily on structure and tuning, and agents here re-hit pitfalls it had already solved (undo/redo greying out on every stroke).
- Consequences: subagent briefs name the draweave files for the feature; lessons that apply to all UI live in ARCHITECTURE section 10.

## D34 2026-10-02 Gestures follow Procreate's defaults as draweave tuned them

- Decision: pinch zoom and rotate, two-finger pan, quick pinch to fit, two-finger tap undo, three-finger tap redo; dead zones, thresholds and snapping take draweave's tested values. Touch never paints (D18); the pen never navigates.
- Reason: the author's direction; feel lives in the dead zones and thresholds, which draweave already tuned on this kind of tablet.
- Consequences: S14 adds multi-pointer routing and scenario steps; touch-and-hold eyedropper and pen-button mapping remain separate decisions.

## D35 2026-10-03 The view is session state relative to fit; the screen samples the canvas bilinearly; finger gestures are app code

- Decision: `app::CanvasView` (pan in points, zoom 0.25 to 8 relative to fit = min(area / canvas) x 0.92, free rotation) lives in the session: no history step and, simplest first (D33), not stored: every artwork opens at fit, so S14 changes no format (schema stays 4). draweave had no rotation snapping and neither does S14 (this narrows D34). One `ViewMapping` serves input, rendering and picking. The blit maps every screen pixel through it and samples the canvas target bilinearly without mipmaps; the target and evaluation regions stay in canvas pixels, so export and thumbnails are untouched. Finger gestures are pure app code (`app::gestures`, draweave's thresholds in points) fed by the UI's canvas input, which knows the controls; the pen draws only from the canvas itself, never from the surround. New-canvas presets stay at most 4096 px a side, which every Vulkan device holds. Debug APKs carry a UiAutomation instrumentation for device gesture checks.
- Reason: zoom relative to fit refits after rotation for free; bilinear at 1:1 on whole pixels is exact, so nothing extra is needed for crisp 100 %. Keeping recognisers out of egui keeps them testable without a UI and in one place; `adb shell input` cannot make multi-finger gestures.
- Consequences: every scenario hash moved once (strokes land through the fit view). Remembering the view per artwork, snapping and far-zoom-out quality are ROADMAP candidates; storing the view is a format change with its own migration.

## D36 2026-10-03 A fill is artwork content: a bit mask and a colour at its place among the layer's strokes; schema 5

- Decision: `core::RegionEdit` (in `Layer::edits`) holds a `RegionMask` (tight bounding rectangle, rows bit-packed in an `Arc<[u32]>`), a colour, the Alpha Lock it was made under (paints atop) and `after`, the number of the layer's strokes under it. `Artwork::fill_region(mask, color)` fills the active layer above its strokes so far. The evaluator draws a fill as one `Fill` op (a paint op whose coverage is the mask bit, a hard edge) at its place; the scene uploads every mask once per scene into one storage buffer (binding 7). `changed_regions` costs a fill by its rectangle.
- Schema 5: each head layer gains a fill count (u32) after its stroke count; one record per fill `[project]['f'][layer][index]`: after, colour, alpha lock, rectangle, then the mask as alternating outside/inside runs over the rectangle in LEB128 varints (a closed shape costs a few bytes per row). Saves write fills by common prefix like strokes. Migration 4 -> 5 (pure): heads re-encoded with fill count 0, everything else kept. `schema-1..4.bin` open upgraded; `schema-5.bin` (two fills) is byte-exact.
- Reason: a pixel mask keeps the fill exactly what the flood found at native resolution (draweave traced contours to vector regions, which needed a contour crate and a point budget); the bit mask is also the GPU's form, so nothing converts per frame. Runs keep the frozen sample at 1448 bytes.
- Consequences: a full-canvas fill costs 618 KB in memory and history; every scene rebuild re-uploads all masks (fine for tens of fills; a Candidate if long sessions show it). Transform and selection edits will be further region kinds in `RegionEdit`.

## D37 2026-10-03 ColorDrop: draweave's flood and gestures; previews off the UI thread; the threshold is session state

- Decision: `openpaint-raster::connected_region` floods four neighbours from the seed against the seed's colour only, distance = max channel difference of premultiplied RGBA8 (alpha included), limit = threshold x 255 (default 0.6), then adds one 8-neighbour edge ring where the distance keeps rising (draweave 674c59b); no gap closing. The source is the active layer alone at full opacity (`render::Core::layer_pixels`, its own target, the single evaluator), sampled once per fill gesture on the `openpaint-fill` worker and kept until the fill ends. The session hands out only its newest request, one at a time; the last finished preview stays shown (`Session::shown`) and is never a history step; a commit records the newest request's fill as one step. A hidden or locked active layer refuses with a top-bar notice; Alpha Lock paints atop (Procreate). UI (`color_drop.rs`): from the colour button, active after 8 pt, anchored after 0.4 s still (< 3 pt), threshold += sign x (|dx| - 4) / 600 per point in 0.5 % steps, lift over a control or off the canvas cancels; after a drop a bar offers Continue filling (pen taps fill until Done or a tool change). The threshold lives in the session (not stored).
- Reason: draweave's tested behaviour; the fill reference layer is left out (simplest first, D33).
- Consequences: a flood of a full 2880 x 1716 canvas takes about 145 ms on the tablet, so a fast slide shows previews a few per second; a pencil's grain stops a low-threshold fill at its specks.

## D38 2026-10-03 Accumulating deposits on a canvas-fixed paper, after draweave's tuned pencil; schema 6

- Decision: a recipe deposits by `Cover` (the maximum coverage of its segments: crossings never darken; the Ink Pen and every stroke before S16) or `Accumulate` (tip flat `1 - d^8` or bell `(1 - d^2)^2`, flow, max density, optional paper). An accumulating segment adds optical density: its tip's cross-section integrated in closed form over the radii travelled, times opacity dynamics x flow (draweave's ADR-0050), so joints never count twice and more events never darken; coverage = max density x (1 - exp(-density)) x grain x opacity, so crossings darken up to the stroke's opacity. The active stroke keeps density (or coverage) and paper contact in two `R32Float` textures; an evaluation resumes from them at its first fresh op, so the float sum continues bit for bit (incremental = cold, tested).
- Paper (flat tips): ranks fixed to canvas coordinates; contact (over pressure, the maximum along the path) touches ridges with rank > 1 - contact; deposit dust + (1 - dust) reach^depth there, dust in valleys (fading below contact 0.3). One canvas pixel covers `scale` texels, so coverage is draweave's five-point Gauss-Hermite expectation over the mean and variance of the ranks under the pixel (a mip chain of moments in a storage buffer): one texel per pixel read as per-pixel noise and averaged heights fade the grain (both prototyped). No zoom-dependent filtering: the canvas is always one sample per canvas pixel, so the deferred zoomed-out filtering stays deferred.
- Texture: `render/assets/paper-sketch.bin`, 1024 x 512 ranks: draweave's `sketch.png` (ambientCG Paper001, CC0 1.0; SHA-256 3e96592f...) halved by 2 x 2 sums and ranked again to 8 bits, ties by index (SHA-256 49c1bfd5...); scale 4.5 there is draweave's 9. A released texture never changes; another look is another texture id.
- Pencil = draweave's tuned pencil (radius 0.3 + 0.7p, flow 0.9 + 2.2p, contact 0.4 + 0.55p, max density 1, depth 0.3, dust 0.3; no speed, no tilt); Soft Brush accumulates under a bell, flow 1.2, so the soft eraser builds up too.
- Schema 6: after each recipe's grain its deposit (u8 tag; tip, flow, max density, paper id; contact curve, scale, depth, dust). Migration 5 -> 6 (pure, `migrate_6.rs`) inserts "cover" into every stroke and marks heads; `schema-6.bin` byte-exact; schemas 1 to 5 open upgraded. The S11 pencil and soft brush keep their golden hashes; the author's artwork reopened AE 0.

## D39 2026-10-03 StreamLine smooths the stroke intent; the stroke stores the smoothed path

- Decision: draweave's leash (ADR-0014, 406cfe9) in the app's stroke intent: the drawn line moves only when the pen is farther than the leash, full leash clamp(3 x size, 24, 96) canvas pixels times amount^2; positions only (pressure, tilt, time stay the pen's); the amount is frozen at pen-down. At the lift the remaining offset spreads over the last 2 leashes with smoothstep weights (`StrokeSession::revise`), so the stroke ends where the pen left; the renderer redraws the old and the new end. The amount is per brush and tool in the toolbox (an f32 after opacity in the editor record, 0 on upgrade), set by "StreamLine 0-100 %" under the brush panel's rows.
- Reason: ROADMAP and ARCHITECTURE put stabilisation in the stroke intent; storing what was drawn keeps rendering, export, fill and picking unaware of it, and the format needs no per-stroke amount.
- Consequences: the raw pen path is not kept (re-smoothing old strokes is no feature). No corner smoothing or Catmull-Rom curves yet (a candidate if corners show).

## D40 2026-10-03 Floating ink-outlined islands instead of a top bar; the system's sans, read by the host (amends D32)

- Decision: the author chose option A of three style sets (pure line art): warm paper (250, 248, 244) islands and panels in a 1.25 pt ink outline (70, 67, 62), no shadows, the white canvas with a soft shadow on a greyer desk (233, 230, 224), ink (38, 37, 35) text and icons, the active state an ink ring or row outline in a deeper ink (22, 28, 44), never a filled block. Nothing spans the width: Gallery and Actions float in a left island, Brush, Eraser, Layers and the colour in a right one, notices, the fill bar and the view's zoom are pills in the top band between them; the canvas shows on the whole screen and fits under that 48-point band, so at fit the islands lie on the desk (fit and "Screen size" stay as before). Text is the system's sans (Roboto on Android, Noto Sans CJK for Chinese, both at weight 400) at 18-point body, which the Kotlin host finds in `/system/etc/fonts.xml` and Rust reads (at most 40 MiB each); the UI only orders the families.
- Reason: the author's direction: a calm sketchbook page, no dark engineering chrome, no long empty bar; Ubuntu Light was too thin to read at arm's length; draweave's fix 434ddbe for Chinese titles.
- Consequences: one theme in `ui/src/design/palette.rs`. The pen's hover outline is drawn in the screen blit, black or white per pixel against what lies under it (draweave beb3865), so `CanvasPlacement` carries it. The harness stands in Segoe UI and Noto Sans SC; hashes only moved where slider values moved by layout (one level, from rounding).
