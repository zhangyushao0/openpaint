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

S0 and S1 are done (see "Done").

## G1 One stroke, end to end

Exit: on the tablet, draw with pressure, undo, kill the app, reopen, the stroke is still there; pen-to-pixel latency has a measured number; the author judges basic pen feel `pass` / `fail`.

Passed 2026-10-02: S2 to S8 done (see "Done"); the author judged pen feel and palm rejection by hand `pass` on the tablet.

## G2 Artworks and layers

Exit: the author finishes an inked and flat-coloured piece (W1) on the tablet.

S9 to S11 are done (see "Done").

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

- 2026-10-02 View transform (zoom, pan, rotate, refit after device rotation): no slice owns it; until then the canvas is fixed at its first size, one screen pixel per canvas pixel.
- 2026-10-02 Vulkan staging stall (~1.5 ms every ~4,000 `write_buffer` calls on the dev machine): pack the evaluator's per-frame writes or use a staging belt if the tablet shows it.
- 2026-10-02 Finished-stroke cost per frame: cache the tile binning of finished ops per artwork version before long sessions make the plan step slow.
- 2026-10-02 Touch gestures for history (two-finger tap undo, three-finger tap redo, Procreate's model): gesture mapping is the author's call (ARCHITECTURE section 10); needs multi-touch routing and a multi-pointer scenario step. Undo/redo are side-bar buttons until then (and after, as in Procreate).
- 2026-10-02 Presentation latency: on the tablet a queued frame reaches the glass p50 28 ms later (about 3.4 vsyncs at 120 Hz) against 9 ms from pen event to queue (D22); front-buffered stylus rendering or tighter frame timing is the lever, not a render thread.
- 2026-10-02 Layer panel follow-ups (S9): drag rows to reorder (press and hold), rename, layer thumbnails; `changed_regions` for a layer property change could cover only that layer's strokes instead of the whole canvas (an opacity drag re-evaluates the whole canvas every frame).
- 2026-10-02 CJK and other non-Latin glyphs (S10): egui's built-in fonts have none, so a Chinese title or layer name shows boxes; the host would hand a system font (for example Noto Sans CJK) to the UI at start.
- 2026-10-02 Gallery follow-ups (S10): Procreate's swipe-left on a card for duplicate and delete, sorting by hand, selecting several; re-show the soft keyboard when the field is tapped after the keyboard was closed with its own button; a canvas size menu for "+".
- 2026-10-02 Side bar follow-ups (S11): a left-handed setting that mirrors it (one constant in `components/side_panel.rs`, the bubble side in `side_slider.rs`); Procreate's modify button between the sliders (eyedropper, S12); a hold-to-show size preview circle.
- 2026-10-02 Brush follow-ups (S11): brush editing and import through a `BrushStore`; flow that builds up within a stroke (needs kept state, D17); cache per-sample speed per stroke so a scene rebuild does not recompute it.
- 2026-10-02 Pen stroke cancelled by a landing touch: with injected input the dispatcher cancels the pen's gesture when a touch lands mid-stroke and the stroke is dropped; if real hardware does the same, commit the stroke drawn so far on such a cancel.

## Done

(One line per slice: id, date, commit. At 30 lines, move to `docs/archive/ROADMAP-<YYYY-MM>.md`.)

- S0 2026-10-02 `feat(render): headless GPU device, offscreen clear and readback smoke (S0)`
- S1 2026-10-02 `feat(android): Kotlin host, surface frames with an egui top bar, typed motion batches (S1)`; confirmed on the tablet
- S2 2026-10-02 `feat(core): immutable artwork value, layer commands, stroke sessions, validated storage entry (S2)`
- S3 2026-10-02 `feat(render): region evaluator with tile-binned ops and a continuous round brush (S3)`
- S4 2026-10-02 `feat(runtime): pen input to strokes on the first screen (S4)`; confirmed on the tablet with injected input
- S5 2026-10-02 `feat(devtools): scenario runner with canvas checkpoints, cargo scenario, smoke in the gate (S5)`
- S6 2026-10-02 `feat(app): history with a byte budget, undo and redo in the top bar (S6)`; confirmed on the tablet with injected input
- S7 2026-10-02 `feat(store): fjall library with per-stroke records, autosave worker, save-failure retry (S7)`; confirmed on the tablet with injected input
- S8 2026-10-02 `feat(android): palm rejection, round stroke ends, pen latency probe on the tablet (S8)`; draw, undo, reopen and latency confirmed on the tablet with injected input; G1 exit awaits the author (pen feel, palm by hand)
- S9 2026-10-02 `feat(app): layers with a popover panel, undoable layer actions, ordered compositing (S9)`; confirmed on the tablet with injected input; no schema 2 (D24, the author to confirm)
- S10 2026-10-02 `feat(store): artwork library with a gallery, schema 2, thumbnails, rename by keyboard (S10)`; confirmed on the tablet with injected input and Gboard; the author's schema-1 library upgraded unchanged
- S11 2026-10-02 `feat(render): brush recipes, brush panel, side bar with size, opacity and history, schema 3 (S11)`; confirmed on the tablet with injected input; the author's schema-2 library upgraded pixel-identical
