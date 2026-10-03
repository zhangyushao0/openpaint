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

S9 to S14 are done (see "Done"). Inking and flat colour need navigation, fill, stabilisation and a finished look first (author, 2026-10-02), so S14 to S17 come before the exit. Every slice from S14 on starts from the predecessor's implementation of the same feature (`C:\repo\draweave`: behaviour, constants, tested edge cases, fix commits; never its architecture) and the lessons in ARCHITECTURE section 10.

**S15 Fill** (new crate raster, core, app, ui, render)
- Connected-region fill on the active layer: drag the colour swatch onto the canvas (Procreate's ColorDrop) and a fill tap; tolerance set by holding and dragging; behaviour and edge cases as in draweave. Undoable; stored as artwork content.
- Done: cases for soft edges, gaps and pinholes; the walk-through scenario passes.

**S16 Stabilisation and pencil** (app, core, render)
- A StreamLine-like smoothing amount per brush in the stroke intent; the pencil retuned to draweave's tuned pencil (grain, pressure and tilt response). Old artworks keep their recipe snapshots.
- Done: smoothing tests (bounded lag, the stroke end reaches the pen); the pencil's golden re-recorded with the reason; old strokes unchanged (test).

**S17 Visual design** (ui, render adapter, android)
- Paper-white line-art look (D32): light paper chrome, ink-line self-drawn icons, a white canvas on the paper; intuitive and restrained (ARCHITECTURE section 10). Self-explanatory side bar (end icons, labels on pen hover, a size preview while dragging); scroll cues instead of scrollbars; a CJK-capable font from the host.
- Done: two or three style screenshots shown to the author before the full pass; every screen then reviewed against the section 10 checklist; scenarios pass.

## G3 Editing and shaping

Exit: the author finishes painterly shading (W2) and a complex composition revision (W3) on the tablet.

**S18 Selection** (core, app, ui, render): freehand and rectangle selection; paint and fill inside; deselect; the selection is tool state, not artwork. Done: GPU test that pixels outside the selection stay unchanged.
**S19 Transform** (core, app, ui, render): move, scale and rotate a layer or selection; preview through the renderer; commit as a `RegionEdit`. Done: preview and committed result match.
**S20 Smudge** (core, render, ui): a smudge brush drags the current layer's colour; incremental residual tiles; incremental = cold evaluation bit for bit. Done: GPU consistency test; at least 60 Hz smudging on the tablet.
**S21 Groups, clipping masks, blend modes** (core, app, ui, render, store): single-level groups, clipping chains, Multiply/Screen/Overlay; schema upgrade. Done: GPU tests for the compositing formulas.
**S22 Layer masks** (core, app, ui, render, store): add, disable, invert, clear and delete masks with undo; painting on masks. Done: GPU tests for masks combined with opacity, clipping and groups.

## G4 Colour, adjustments, exchange

**S23 Colour management**: P3 display, import interpretation and export conversion defined separately. **S24 Adjustments**: hue/saturation/brightness, curves, blur, sharpen, each with preview, cancel and history. **S25 Project exchange**: the native format as the full original; OpenRaster import and export. **S26 Library management**: search, version restore, external backup.
Exit: the author completes edit, restore and delivery (W4).

## G5 Stability and release

Device tiers, long sessions, background/foreground, low storage, failed upgrades; GPU device-loss recovery and a VRAM budget; a device smoke scenario of at most 20 s; signed releases and a version policy. Optimisation happens here, coarse-grained, driven by measurements on the tablet; earlier slices build the simplest design that meets their budgets (D33) and leave performance ideas under "Candidates". Exit: the declared device matrix passes with no known artwork corruption or silent loss.

## G6 Independent extensions after stabilisation

Wet media, dual brushes, a brush ecosystem, animation, multi-platform, cloud sync, plugins: each is its own project and never enters the main line automatically.

## Candidates

(Findings outside the current slice, one line each, dated.)

- 2026-10-02 Vulkan staging stall (~1.5 ms every ~4,000 `write_buffer` calls on the dev machine): pack the evaluator's per-frame writes or use a staging belt if the tablet shows it.
- 2026-10-02 Finished-stroke cost per frame: cache the tile binning of finished ops per artwork version before long sessions make the plan step slow.
- 2026-10-02 Presentation latency: on the tablet a queued frame reaches the glass p50 28 ms later (about 3.4 vsyncs at 120 Hz) against 9 ms from pen event to queue (D22); front-buffered stylus rendering or tighter frame timing is the lever, not a render thread.
- 2026-10-02 Layer panel follow-ups (S9): drag rows to reorder (press and hold), rename, layer thumbnails; `changed_regions` for a layer property change could cover only that layer's strokes instead of the whole canvas (an opacity drag re-evaluates the whole canvas every frame).
- 2026-10-02 Gallery follow-ups (S10): Procreate's swipe-left on a card for duplicate and delete, sorting by hand, selecting several; re-show the soft keyboard when the field is tapped after the keyboard was closed with its own button; custom canvas sizes.
- 2026-10-02 Side bar follow-ups (S11): a left-handed setting that mirrors it (one constant in `components/side_panel.rs`, the bubble side in `side_slider.rs`); a hold-to-show size preview circle.
- 2026-10-02 Brush follow-ups (S11): brush editing and import through a `BrushStore`; flow that builds up within a stroke (needs kept state, D17); cache per-sample speed per stroke so a scene rebuild does not recompute it.
- 2026-10-02 Colour and eyedropper follow-ups (S12): a loupe under the contact while picking; touch-and-hold eyedropper and the pen's eraser end or buttons (gesture and pen mapping are the author's call); on black or grey a tap on the hue ring changes no visible colour (Procreate's disc behaves alike); colour sliders, palettes, harmony.
- 2026-10-02 Narrow windows (S12): the top bar's right end holds four buttons, so on a window under about 700 points the save notice and Retry no longer fit; the side bar runs off the bottom under about 450 points.
- 2026-10-02 Reference and export follow-ups (S13): pinch zoom and resizing in the reference window (the gesture trackers exist since S14), storing the reference with the artwork (asset store, format change), export with a transparent background, layers or PSD.
- 2026-10-03 View follow-ups (S14): remember the view per artwork (draweave did, committed once at gesture end; a format change); rotation snapping (draweave had none); Procreate's four-finger tap to hide the interface (needs a visible control first, D32); a flip; mipmaps or a reduced copy for far zoomed-out views if shimmer shows on the tablet; ask for frames by deadline instead of continuously while fingers hold.
- 2026-10-03 `replay` (harness): its undo/redo taps aim at the top bar since S11 moved them to the side bar, and its golden predates the fit view; re-aim and re-record.
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
- S12 2026-10-02 `feat(render): eraser, Alpha Lock, colour wheel with recent colours, eyedropper by render picking, schema 4 (S12)`; confirmed on the tablet with injected input; the author's schema-3 library upgraded pixel-identical
- S13 2026-10-02 `feat(android): export, share and a floating reference image (S13)`; exported PNG equals the screen (test and tablet, AE 0); picking, saving and sharing confirmed on the tablet with injected input; G2 exit awaits the author
- S14 2026-10-03 `feat(ui): canvas view, navigation gestures, finger undo and redo, canvas size presets (S14)`; gestures, pen at zoom and rotation, taps and quick pinch confirmed on the tablet through the debug gesture instrumentation; no format change
