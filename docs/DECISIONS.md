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

- Decision: every external crate is declared once in `[workspace.dependencies]` with an exact `=` version; crates use `.workspace = true`. Current pins: wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2 (requires wgpu 30.0), pollster 1.0.1 (blocks on wgpu's adapter/device futures), png 0.18.1 (harness only), serde 1.0.229 and serde_json 1.0.151 (scenario files; devtools only, never core), log 0.4.34, and for the Android host only jni 0.22.4, android_logger 0.15.1, raw-window-handle 0.6.2 (the version wgpu 30 uses). Gradle side, pinned in `apps/android`: Gradle 8.13 (wrapper with checksum), Android Gradle plugin 8.13.2, Kotlin 2.2.21.
- Reason: wgpu and egui break APIs every release and egui-wgpu ties them together; a silent minor bump would change GPU behaviour and golden hashes without a reviewed change.
- Consequences: wgpu, egui and egui-wgpu are upgraded together in one commit that reruns the GPU tests; golden hashes that move are re-recorded in that commit with the reason. egui and egui-wgpu are pinned before any crate uses them, so the first user inherits a known-compatible pair.

## D15 2026-10-02 Android host: main-thread frames, array-packed input, xtask-driven build

- Decision: the Kotlin activity calls Rust only from the main thread: `Choreographer` paces frames on demand (Rust answers whether it wants the next one) and `MotionEvent`s cross JNI as primitive arrays (ids, tool types, x/y/pressure/tilt/orientation per step and pointer, times), decoded into typed batches in plain Rust. The UI painter lives for the process because egui sends its font atlas once. `cargo android-build` launches the Gradle wrapper jar from xtask; no gradlew scripts are tracked. The gate clippy-checks the JNI crate for `aarch64-linux-android`.
- Reason: one thread needs no locks between lifecycle, surface and input calls; arrays keep Kotlin free of product types and make decoding testable on the dev machine; gradlew and gradlew.bat are scripts (D05); without the Android clippy step the JNI code would be compiled by nobody but the APK build.
- Consequences: a slow frame delays input until a render thread exists (decided with S8's latency number); surfaces must be detached in `surfaceDestroyed` before it returns; changing the sample layout means changing `SAMPLE_FIELDS` in both `motion.rs` and `MainActivity.kt`.

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
