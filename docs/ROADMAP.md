# Roadmap

## Product definition

An Android-first, stylus-first 2D painting app that completes, on one tablet: sketch -> inking -> layered flat colour -> smudging and shaping -> selection revisions -> masks and colour overlays -> adjustments and finishing -> export -> resume after interruption. Interaction follows the user's mental model from Procreate by default, adapted to Android tablets; no copying of look, icons or assets.

Stage exits are judged on representative artworks made by the author on the tablet: W1 inking and flat colour, W2 painterly shading, W3 complex composition revision, W4 edit, restore and deliver.

Non-goals (G0 to G5): network, accounts, cloud sync, community, animation, 3D, multi-platform, plugin systems, a desktop product or desktop window of any kind.

## Rules

- A slice is a vertical cut one session can finish, verify and roll back on its own. Slices run in numbered order; no skipping.
- Every slice states its goal, the crates involved and done criteria. Done criteria are executable or observable, never "better".
- A slice that changes what the user sees or does begins with its walk-through written as the user and ends with that walk-through as a passing scenario plus a screenshot review (AGENTS.md, "Design from the user inward").
- The dev-machine loop is the harness (`apps/harness`): offscreen rendering, replay, scenarios, PNG and hash output. "Seeing" a result means reading its PNG. Real input and pen feel exist only on the tablet.
- When a slice is done: rewrite STATUS, move the slice to "Done" at the end of this file (one line), add a DECISIONS entry only if a decision was made.
- Stage-exit items that need the author on the tablet are recorded as `pass` / `fail` / `not_run`; a dev-machine result never substitutes.
- Findings outside the slice go under "Candidates"; they are not fixed in passing.

## G0 Skeleton

S0 and S1 are done (see "Done"); S1's on-tablet confirmation is pending with the author (STATUS).

## G1 One stroke, end to end

Exit: on the tablet, draw with pressure, undo, kill the app, reopen, the stroke is still there; pen-to-pixel latency has a measured number; the author judges basic pen feel `pass` / `fail`.

S2 is done (see "Done").

**S3 Render core** (render, harness)
- `FrameRequest`, `Scene` / `Plan`, stroke geometry -> coverage shader for a round continuous brush, `RegionEvaluator` as the single entry, readback; the harness command `render-stroke` renders a hard-coded stroke to PNG.
- Done: GPU test with a fixed pixel hash for a fixed stroke; incremental = cold evaluation bit for bit; the harness writes the stroke PNG; render contains no `egui` outside adapter.

**S4 Input to stroke, first screen** (runtime, app, ui, harness)
- Walk-through first: open the app, draw with the pen, see the stroke appear under the tip. Normaliser: platform batches -> `InputBatch` (pressure, tilt, timestamp), shared by Android and the harness; app `Session` plus `Intent::Stroke`; the UI canvas hands contacts to the session; the in-progress stroke goes to the renderer separately. `ui/src/design/` installs the theme once and `ui/src/components/` holds the first shared widgets (top bar, canvas frame). The harness command `replay` feeds a recorded stroke through the full frame loop, reports frame time and writes a screenshot.
- Done: replaying a 10,000-sample stroke keeps frame time under 8 ms on the dev machine and its PNG matches a golden hash; the normaliser has unit tests; the screenshot shows the top bar and canvas per ARCHITECTURE section 10.

**S5 Scenario runner** (new crate devtools, harness, xtask)
- JSON steps (tap, drag, stroke, wait, restart) plus checkpoints (readback hash, screenshot); `cargo scenario <name>`; `tests/scenarios/` holds the walk-throughs; the gate runs `smoke`.
- Done: `smoke` draws a stroke and passes a checkpoint in under 5 s; a failed checkpoint writes the diff PNG under `artifacts/` (untracked) and prints its path; later slices add their steps to the runner as they need them.

**S6 History** (app, ui)
- Walk-through: draw, tap undo, the stroke disappears; tap redo, it returns; the buttons grey out when nothing is left. `History` version stack with a byte budget; undo/redo intents; the projection reports availability.
- Done: the walk-through is a passing scenario; 20 undo/redo round trips leave the artwork equal; budget trimming has tests.

**S7 Storage and autosave** (store, runtime, harness)
- Walk-through: draw, leave, come back, everything is there; if saving fails, a message says so and offers retry. fjall library; `Project` DTO schema 1 with a frozen sample; `ProjectStore` implemented; a worker autosaves after each stroke; the last artwork opens on start.
- Done: the walk-through is a passing scenario using the `restart` step with identical pixels; sample round-trip test; gate under 60 s.

**S8 Android loop on the device** (android, runtime)
- Wire S2 to S7 on the Android host; stylus and touch batches enter the same normaliser; basic palm rejection; pen-to-pixel latency instrumented.
- Done: draw, undo and reopen on the tablet; the latency number is in STATUS; the author judges pen feel. G1 exit.

## G2 Artworks and layers

Exit: the author finishes an inked and flat-coloured piece (W1) on the tablet.

**S9 Layers** (core, app, ui, render, store)
- Multiple layers: create, delete, reorder, visibility, opacity, active layer; Normal compositing; layer panel; schema 2 with migration.
- Done: GPU test for compositing order; panel actions go through intents and are undoable; the schema 1 sample upgrades; the walk-through scenario passes.

**S10 Artwork library** (app, store, ui, render)
- Gallery: list, create, open, rename, duplicate, delete, thumbnails (small export); autosave per artwork.
- Done: a scenario covers create, rename, delete and reopen; thumbnails match the artwork.

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

- S0 2026-10-02 `feat(render): headless GPU device, offscreen clear and readback smoke (S0)`
- S1 2026-10-02 `feat(android): Kotlin host, surface frames with an egui top bar, typed motion batches (S1)`; tablet confirmation pending
- S2 2026-10-02 `feat(core): immutable artwork value, layer commands, stroke sessions, validated storage entry (S2)`
