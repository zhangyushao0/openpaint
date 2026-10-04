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
- Performance (D47): a slice is fluid at its feature's default sizes on the tablet; other numbers are measured and recorded, never done criteria; chasing them is G5.

## G0 Skeleton

S0 and S1 are done (see "Done").

## G1 One stroke, end to end

Exit: on the tablet, draw with pressure, undo, kill the app, reopen, the stroke is still there; pen-to-pixel latency has a measured number; the author judges basic pen feel `pass` / `fail`.

Passed 2026-10-02: S2 to S8 done (see "Done"); the author judged pen feel and palm rejection by hand `pass` on the tablet.

## G2 Artworks and layers

Exit: the author finishes an inked and flat-coloured piece (W1) on the tablet.

Passed 2026-10-03: S9 to S17 and the G2 follow-ups done (see "Done"); the author judged W1 `pass` on the tablet. Inking and flat colour needed navigation, fill, stabilisation and a finished look first (author, 2026-10-02), so S14 to S17 came before the exit. Every slice from S14 on starts from the predecessor's implementation of the same feature (`C:\repo\draweave`: behaviour, constants, tested edge cases, fix commits; never its architecture) and the lessons in ARCHITECTURE section 10.

## G3 Editing and shaping

Exit: the author finishes painterly shading (W2) and a complex composition revision (W3) on the tablet.

S18 to S24 are done (see "Done"); S21's 60 Hz holds up to about 340 px and fails at the largest sizes, an author decision (STATUS). Before the exit, from the author's first try on the tablet (2026-10-04):

**S25 One ring rule for text controls** (ui): the ink ring means "chosen" for words as it does for icons. A choice among words (Freehand / Rectangle, the blend modes, any `text_choice`) is bare text at rest and the chosen word carries an ink pill in the icon ring's weight; one-shot commands (Invert, Deselect, Clear, Group, the transform bar's buttons, dialog buttons) are bare text with the press wash and acknowledge, set after a thin divider when they share a bar with a choice; no frame-in-frame anywhere. Done: every screen's screenshots reviewed (selection bar, transform bar, layer options and mask footer, fill bar, dialogs, gallery), hit targets unchanged at 48 points, every scenario passes on names (hashes of canvas checkpoints unchanged).

**S26 Insert an image** (store, core, app, render, ui, android): Actions gains "Insert image…" (the system picker, PNG and JPEG, Procreate's Insert a photo): the picture becomes a new layer above the active one, fitted inside the canvas, and the transform opens on it at once; one undo removes it; it reopens, exports, masks, clips and transforms like painted content. The picture's pixels are stored once in the library (format change, migration and frozen sample). Done: walk-through scenario with a fixture image, GPU test of the placed picture (incremental = cold), migration test, tablet walk-through with a picture from `openpaint-test`, the author's library upgraded pixel-identical.

Then the G3 exit (W2, W3) by the author on the tablet.

## G4 Colour, adjustments, exchange

**S27 Colour management**: P3 display, import interpretation and export conversion defined separately. **S28 Adjustments**: hue/saturation/brightness, curves, blur, sharpen, each with preview, cancel and history. **S29 Project exchange**: the native format as the full original; OpenRaster import and export. **S30 Library management**: search, version restore, external backup.
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
- 2026-10-02 Brush follow-ups (S11): brush editing and import through a `BrushStore`; cache per-sample speed per stroke so a scene rebuild does not recompute it.
- 2026-10-02 Colour and eyedropper follow-ups (S12): touch-and-hold eyedropper and the pen's eraser end or buttons (gesture and pen mapping are the author's call); on black or grey a tap on the hue ring changes no visible colour (Procreate's disc behaves alike); colour sliders, palettes, harmony.
- 2026-10-02 Narrow windows (S12): the top bar's right end holds four buttons, so on a window under about 700 points the save notice and Retry no longer fit; the side bar runs off the bottom under about 450 points.
- 2026-10-03 Eyedropper across windows (G2 follow-ups): a picking contact that crosses between the canvas and the reference picture keeps sampling only where it started.
- 2026-10-02 Reference and export follow-ups (S13): remembering the window's place, size and zoom, storing the reference with the artwork (asset store, format change), export with a transparent background, layers or PSD.
- 2026-10-03 View follow-ups (S14): remember the view per artwork (draweave did, committed once at gesture end; a format change); rotation snapping (draweave had none); Procreate's four-finger tap to hide the interface (needs a visible control first, D32); a flip; mipmaps or a reduced copy for far zoomed-out views if shimmer shows on the tablet; ask for frames by deadline instead of continuously while fingers hold.
- 2026-10-03 Fill follow-ups (S15): draweave's fill reference layer (flood one layer, fill another); gap closing; keeping the threshold across artworks and launches (toolbox state); the fill bar covers the canvas's top middle while offered; a pencil's grain stops low-threshold fills at its specks.
- 2026-10-03 Fill cost (S15): each scene rebuild re-uploads every fill mask (618 KB for a full-canvas fill), and a full-canvas flood takes about 145 ms on the tablet; cache masks on the target and flood incrementally if long sessions or fast slides show it (G5).
- 2026-10-03 Pencil and StreamLine follow-ups (S16): grain filtering by view zoom (draweave's Gauss-Hermite at the screen's footprint, only if a re-evaluation at screen resolution ever comes; the screen samples the canvas bilinearly today); breaking the paper's repeat (every ~228 x 114 canvas pixels) with offsets; tilt widening the pencil (draweave's pencil ignored it); corner smoothing and Catmull-Rom curves if sparse input shows polyline corners; keeping the raw pen path to re-smooth strokes; pressure smoothing.
- 2026-10-03 Visual design follow-ups (S17): the scenario walk-throughs before S17 still say "top bar" in their prose (their taps follow the islands); on windows under about 700 points the two top islands and a notice pill meet; egui's hover label fades in rather than appearing at once; Chinese typed into a title on the tablet itself (only the system font loading and the harness's Chinese title are checked); a left-handed mirror of the side bar and the islands.
- 2026-10-03 Selection follow-ups (S18): Add and Remove modes, feather and antialiased edges (a coverage clip, a format change), Automatic and Ellipse, saving selections; one clip record per selection instead of its runs in every clipped stroke; counting clip masks in history bytes; the bar's buttons shift when Close appears (centred pill).
- 2026-10-03 Motion follow-ups (S19): the chrome pass and egui's motion bookkeeping add about 1 ms of frame work while idle on the tablet (G5); rows crossing on a reorder overlap their text; a rotated canvas shrinks into its card upright; finger injection on the tablet was refused system-wide this session (pen worked), so the quick-pinch glide is checked in the harness only.
- 2026-10-03 Transform follow-ups (S20): Freeform, Distort and Warp; angle snapping while turning (draweave had only the 45° button); Fit to screen; mipmapped or bicubic resampling for strong shrinking; the moved selection kept selected instead of deselecting; lifting on a worker instead of the main thread (a full-canvas lift waits for one GPU readback); a transform still open when the app is left is not saved (commit it on leave); a whole-layer patch per transform in history (64 MiB budget).
- 2026-10-03 Smudge follow-ups (S21): large smudges are slow (600 px: frame work p50 21 to 28 ms; the contacts' kernels grow with the radius squared, and the footprint is evaluated again over everything beneath, as for a large paint brush); a coarser residual grid for large radii, fused stamp and commit, or a size cap are the levers (author's call, STATUS). Reopening evaluates every smudge cold; residual memory (20 bytes per pixel of its tiles) is never trimmed below the arena's live blocks; a smudge reads only its own layer (Procreate can sample all layers).
- 2026-10-04 Mask follow-ups (S24): move a mask with its layer's whole-layer transform (Procreate links them) and transform a mask on its own; masks on groups; apply a mask into its layer (with merging); a mask thumbnail on its row; a grey-only colour panel while a mask is chosen (Procreate) instead of the colour's luma; swipe a mask row to delete.
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
- S15 2026-10-03 `feat(raster): ColorDrop fill with threshold hold, continue filling, fills as artwork content, schema 5 (S15)`; drop, hold-and-slide, fill tap, undo, hidden-layer refusal and reopen confirmed on the tablet with injected input; the author's schema-4 library upgraded pixel-identical
- S16 2026-10-03 `feat(render): accumulating pencil on canvas-fixed paper, StreamLine in the stroke intent, schema 6 (S16)`; pencil, ink and StreamLine confirmed on the tablet with injected pen paths; the author's schema-5 library upgraded pixel-identical
- S17 2026-10-03 `feat(ui): paper-white line art in floating islands, system fonts, pen hover outline, self-explanatory side bar (S17)`; option A chosen from three style sets; walk-through confirmed on the tablet with injected pen, hover and finger input; the author's canvas unchanged pixel for pixel
- G2 exit 2026-10-03: the author's W1 piece on the tablet, `pass`
- G2 follow-ups 2026-10-03 `feat(ui): eyedropper loupe, eraser outline while pressed, gesture-driven reference window (G2 follow-ups)`; walk-through confirmed on the tablet with injected pen and finger input; the author's library byte-identical
- S18 2026-10-03 `feat(app): freehand and rectangle selection, painting and filling inside, schema 7 (S18)`; walk-through confirmed on the tablet with injected pen and finger input; the author's schema-6 library upgraded pixel-identical
- S19 2026-10-03 `feat(ui): motion foundation: one spring, four patterns, chrome grown from its islands (S19)`; SDF chrome landed; walk-through recorded on the tablet with injected pen input; the author judges the feel
- S20 2026-10-03 `feat(app): transform a layer or a selection as a placed patch, preview equal to commit, schema 8 (S20)`; walk-through confirmed on the tablet with injected pen and fingers
- S21 2026-10-03 `feat(render): smudge as a residual evaluated in layer order, in progress equal to cold, schema 9 (S21)`; walk-through confirmed on the tablet; 60 Hz `pass` up to about 340 px, `fail` at 600 px
- S22 2026-10-04 `chore(devtools): named scenario targets and device commands (S22)`; every scenario on names with its old hashes; one device routine through the commands alone, library byte-identical after restore
- S23 2026-10-04 `feat(render): groups, clipping masks and blend modes, schema 10 (S23)`; GPU tests per formula against hand-worked values, chains, pass-through group opacity, incremental = cold (D49, D50)
- S24 2026-10-04 `feat(render): layer masks painted in grey, applied before the layer's end, schema 11 (S24)`; GPU tests for masks with opacity, clipping chains, faded groups, off and inverted, incremental = cold (D51); confirmed on the tablet; the G3 exit awaits the author
