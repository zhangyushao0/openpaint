# Roadmap

## Product definition

An Android-first, stylus-first 2D painting app that completes, on one tablet: sketch -> inking -> layered flat colour -> smudging and shaping -> selection revisions -> masks and colour overlays -> adjustments and finishing -> export -> resume after interruption. Interaction follows the user's mental model from Procreate by default, adapted to Android tablets; no copying of look, icons or assets.

Stage exits are judged on representative artworks made by the author on the tablet: W1 inking and flat colour, W2 painterly shading, W3 complex composition revision, W4 edit, restore and deliver.

Non-goals (G0 to G5): network, accounts, cloud sync, community, animation, 3D, multi-platform, plugin systems, a desktop product.

## Rules

- A slice is a vertical cut one session can finish, verify and roll back on its own. Slices run in numbered order; no skipping.
- Every slice states its goal, the crates involved and done criteria. Done criteria are executable or observable, never "better".
- When a slice is done: rewrite STATUS, move the slice to "Done" at the end of this file (one line), add a DECISIONS entry only if a decision was made.
- Stage-exit items that need the author on the tablet are recorded as `pass` / `fail` / `not_run`; a dev-machine result never substitutes.
- Findings outside the slice go under "Candidates"; they are not fixed in passing.

## G0 Skeleton

**S0 Desktop host and dependencies** (desktop, render, ui, runtime)
- Add and pin wgpu, egui, egui-wgpu, winit (versions recorded in DECISIONS).
- The desktop host opens a window: an egui top bar plus a canvas area cleared through the render adapter with wgpu; resizing does not crash.
- Done: `cargo run -p openpaint-desktop` shows the window; `cargo gate` under 60 s warm; no new markdown.

**S1 Android host** (android, runtime)
- Decide the host form first (`android-activity` + winit, or a hand-written Activity + JNI) and record it in DECISIONS; add the `aarch64-linux-android` target, `cargo-ndk`, a minimal Gradle project and a `cargo android-build` alias.
- Done: the APK installs on the author's tablet and shows the same interface as the desktop; stylus events with pressure reach Rust and are logged; background/foreground and rotation do not crash. The author confirms the install.

## G1 One stroke, end to end

Exit: on the tablet, draw with pressure, undo, kill the app, reopen, the stroke is still there; pen-to-pixel latency has a measured number; the author judges basic pen feel `pass` / `fail`.

**S2 Core artwork value** (core)
- `Artwork(Arc<State>)`, `Layer`, `Stroke`, `StrokeSample`, ids; `LayerCommand` (create, delete, reorder, properties); `apply` -> `Applied { artwork, change }`; `StrokeSession` begin/extend/finish/cancel; `from_storage` full validation; `CoreError`.
- Done: every command has success and error cases; no public mutable setters; `cargo test -p openpaint-core` passes; core has no external dependencies.

**S3 Render core** (render, desktop)
- `Core` (device, queue, pipelines), `FrameRequest`, `Scene` / `Plan`, stroke geometry -> coverage shader for a round continuous brush, `RegionEvaluator` as the single entry, readback.
- Done: GPU test with a fixed pixel hash for a fixed stroke; incremental = cold evaluation bit for bit; the desktop shows a hard-coded stroke; render contains no `egui` outside adapter.

**S4 Input to stroke** (runtime, app, ui, desktop)
- Normaliser: winit pen and mouse events -> `InputBatch` (pressure, tilt, timestamp); app `Session` plus `Intent::Stroke`; the UI canvas hands contacts to the session; the in-progress stroke goes to the renderer separately.
- Done: continuous drawing on the desktop with mouse or pen; frame time under 8 ms with a 10,000-sample stroke (dev machine); the normaliser has unit tests.

**S5 History** (app, ui)
- `History` version stack with a byte budget; undo/redo intents; top-bar buttons; the projection reports availability.
- Done: 20 undo/redo round trips leave the artwork equal; budget trimming has tests; UI buttons disable per availability.

**S6 Storage and autosave** (store, runtime, desktop)
- fjall library; `Project` DTO schema 1 with a frozen sample; `ProjectStore` implemented; a worker autosaves after each stroke; the last artwork opens on start; save failures show a message with retry.
- Done: draw on the desktop, kill the process, reopen with identical content; sample round-trip test; gate under 60 s.

**S7 Android loop on the device** (android, runtime)
- Wire S2 to S6 on the Android host; Android touch and stylus events enter the same normaliser; basic palm rejection; pen-to-pixel latency instrumented.
- Done: draw, undo and reopen on the tablet; the latency number is in STATUS; the author judges pen feel. G1 exit.

## G2 Artworks and layers

Exit: the author finishes an inked and flat-coloured piece (W1) on the tablet.

**S8 Desktop automated regression** (new crate devtools, desktop, xtask)
- Headless scenario runner: JSON steps (tap, drag, stroke, undo, save, reopen) plus pixel checkpoints (readback hashes); `cargo scenario <name>`; the gate runs the `smoke` scenario.
- Done: `smoke` covers draw, undo, save and reopen in under 5 s; a failed checkpoint prints the diff image path (under `artifacts/`, untracked).

**S9 Layers** (core, app, ui, render, store)
- Multiple layers: create, delete, reorder, visibility, opacity, active layer; Normal compositing; layer panel; schema 2 with migration.
- Done: GPU test for compositing order; panel actions go through intents and are undoable; the schema 1 sample upgrades.

**S10 Artwork library** (app, store, ui, render)
- Gallery: list, create, open, rename, duplicate, delete, thumbnails (small export); autosave per artwork.
- Done: a desktop scenario covers create, rename, delete and reopen; thumbnails match the artwork.

**S11 Brush recipes v1** (core, render, ui, store)
- `BrushRecipe` TOML: shape, size and opacity curves over pressure and speed, grain; built-in pencil, ink pen and soft brush; brush picker with size and opacity sliders; strokes store a recipe snapshot.
- Done: GPU hash tests for the three brushes; recipe parse errors are typed; changing a built-in recipe does not change old artworks (test).

**S12 Eraser, Alpha Lock, colour** (core, app, ui, render)
- The eraser uses the same recipes; Alpha Lock; HSV wheel with recent colours; eyedropper through render picking.
- Done: GPU tests for erasing and Alpha Lock; picked colours match exported pixels.

**S13 Export and reference image** (app, store, render, ui, android)
- PNG/JPEG export through `RegionEvaluator` readback on a worker; floating reference window (move, zoom, pick); Android system picker and share in the host.
- Done: the exported image matches the screen pixel for pixel (test); picking, exporting and sharing work on the tablet. The author judges the G2 exit.

## G3 Editing and shaping

Exit: the author finishes painterly shading (W2) and a complex composition revision (W3) on the tablet.

**S14 Selection** (core, app, ui, render): freehand and rectangle selection; paint inside; deselect; the selection is tool state, not artwork. Done: GPU test that pixels outside the selection stay unchanged.
**S15 Transform** (core, app, ui, render): move, scale and rotate a layer or selection; preview through the renderer; commit as a `RegionEdit`. Done: preview and committed result match.
**S16 Fill** (new crate raster, core, app, ui): connected-region fill, tolerance, intersection with the selection. Done: cases for soft edges, gaps and pinholes.
**S17 Smudge** (core, render, ui): a smudge brush drags the current layer's colour; incremental residual tiles; incremental = cold evaluation bit for bit. Done: GPU consistency test; at least 60 Hz smudging on the tablet.
**S18 Groups, clipping masks, blend modes** (core, app, ui, render, store): single-level groups, clipping chains, Multiply/Screen/Overlay; schema upgrade. Done: GPU tests for the compositing formulas.
**S19 Layer masks** (core, app, ui, render, store): add, disable, invert, clear and delete masks with undo; painting on masks. Done: GPU tests for masks combined with opacity, clipping and groups.

## G4 Colour, adjustments, exchange

**S20 Colour management**: P3 display, import interpretation and export conversion defined separately. **S21 Adjustments**: hue/saturation/brightness, curves, blur, sharpen, each with preview, cancel and history. **S22 Project exchange**: the native format as the full original; OpenRaster import and export. **S23 Library management**: search, version restore, external backup.
Exit: the author completes edit, restore and delivery (W4).

## G5 Stability and release

Device tiers, long sessions, background/foreground, low storage, failed upgrades; GPU device-loss recovery and a VRAM budget; a device smoke scenario of at most 20 s; signed releases and a version policy. Exit: the declared device matrix passes with no known artwork corruption or silent loss.

## G6 Independent extensions after stabilisation

Wet media, dual brushes, a brush ecosystem, animation, multi-platform, cloud sync, plugins: each is its own project and never enters the main line automatically.

## Candidates

(Findings outside the current slice, one line each, dated.)

## Done

(One line per slice: id, date, commit. At 30 lines, move to `docs/archive/ROADMAP-<YYYY-MM>.md`.)
