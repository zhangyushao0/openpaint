# Roadmap

## Product definition

An Android-first, stylus-first 2D painting app that completes, on one tablet: sketch -> inking -> layered flat colour -> smudging and shaping -> selection revisions -> masks and colour overlays -> adjustments and finishing -> export -> resume after interruption. Interaction follows the user's mental model from Procreate by default, adapted to Android tablets; no copying of look, icons or assets.

Stage exits are judged on representative artworks made by the author on the tablet: W1 inking and flat colour, W2 painterly shading, W3 complex composition revision, W4 edit, restore and deliver. The device matrix is the Wacom MovinkPad 14 alone (author, 2026-10-09).

Non-goals of the main line: network, accounts, cloud sync, community, animation, 3D, multi-platform, plugin systems, a desktop product or desktop window of any kind (G8 projects, each the author's call).

## Rules

- A slice is a vertical cut one session can finish, verify and roll back on its own. Slices run in numbered order; no skipping.
- Every slice states its goal, the crates involved and done criteria. Done criteria are executable or observable, never "better".
- A slice that changes what the user sees or does begins with its walk-through written as the user and ends with that walk-through as a passing scenario plus a screenshot review (AGENTS.md, "Design from the user inward").
- The dev-machine loop is the harness (`apps/harness`): offscreen rendering, replay, scenarios, PNG and hash output. "Seeing" a result means reading its PNG. Real input and pen feel exist only on the tablet.
- Each feature follows Procreate's behaviour and the predecessor's tested behaviour (`C:\repo\draweave`: constants, edge cases, fix commits; never its architecture).
- Simplest design first: a slice builds the direct design that meets its done criteria; caches, incremental schemes and tuning go under "Candidates" and are done coarse-grained, measured on the tablet.
- Performance: a slice is fluid at its feature's default sizes on the tablet (a frame within one 60 Hz vsync); other numbers are measured and recorded in STATUS, never done criteria.
- When a slice is done: rewrite STATUS, move the slice to "Done" at the end of this file (one line), add a DECISIONS entry only if a decision was made.
- Stage-exit items that need the author on the tablet are recorded as `pass` / `fail` / `not_run`; a dev-machine result never substitutes.
- Findings outside the slice go under "Candidates"; they are not fixed in passing.

## Stages passed

- G0 Skeleton, 2026-10-02: the workspace, the gate, the Android host and the harness (S0, S1).
- G1 One stroke, end to end, 2026-10-02: pressure, undo, a stroke surviving a killed app, measured latency; the author judged pen feel and palm rejection `pass` (S2 to S8).
- G2 Artworks and layers, 2026-10-03: the author finished an inked, flat-coloured piece (W1 `pass`): layers, gallery, brushes, colour, export and reference, navigation, fill, StreamLine and the paper-white look (S9 to S17).
- G3 Editing and shaping, 2026-10-04: the author tried every feature (W2, W3): selection, motion, transform, smudge, groups, clipping, blend modes, masks, insert (S18 to S26).
- G4 Colour, adjustments, sharp rendering, library, 2026-10-05: the author completed W4 and saw the ten-bit precision check smooth: colour management, adjustments, export at a scale, fill edges, transform records, sharp zoom, blur and sharpen, smooth paths, library, ten-bit display (S27 to S37).
- G5 Daily painting, 2026-10-09: the author tried every feature: layer actions, Merge Down, copy and paste, pen buttons, palette, QuickShape and symmetry (S38 to S43).
- G6 Brushes, 2026-10-09: the author painted with the set, "all good, very good, the preview included": taper and tilt, shapes and scatter, the brush editor, sets (S44 to S47).
- G7 Stability and release, 2026-10-22: the author painted on the tablet, "works well": painting speed, GPU and storage robustness, memory, the device smoke and version policy, navigation that never waits, the display cache, the render core rewritten onto the GPU's rasterizer and the evaluator deleted (S48 to S67).

## 1.0 A clean release

The author, 2026-10-22: 1.0.0 is a new product, not an upgrade. No compatibility with anything before it: the formats designed afresh, the author's library on the tablet not kept; then everything unreasonable, duplicated or left over from the slices' history simplified, behaviour unchanged.

- S68 The 1.0 formats is done: format 1 for the library, OpenPaint files and kept versions (D95), every migration deleted, another format refused untouched; the hint session gone (D96).
- S69, S70, S71 Simplification is done: render, runtime and tools, app, ui, core and store each read whole and simplified in parallel worktrees, behaviour unchanged.
- S72 Cross-crate simplification is done: one image source, one whole draw, one runtime constructor, one store wrapper, one runtime test binary, the canvas's modules in `render/src/canvas/`.
- S73 1.0.0 is done: one layer refusal, one curve mapping, the budget test on its own clock, the gallery's delete note in ink; the four documents rewritten as the product now is; the version 1.0.0; on the tablet a fresh install, the smoke and the benches.

## 1.x Daily use

Findings from the author's own use of 1.0, fixed one at a time.

- S74 UI at the display's rate is done (1.0.1): `bench ui` measures each popover, the wheel, a slider and the editor's column; the cause was the pacing (a UI frame's CPU and GPU work together pass a vsync while the clocks are low, and frames waited for the one before: every third vsync at rest, every second with a hand) and an open panel's chrome pass (4 to 11 ms of GPU); a frame now goes every vsync unless a stroke is drawn or nothing moves (D97). Openings 36 to 50 -> 95 to 100 %, closings 51 to 88 -> 97 to 98 %, the wheel 45 -> 98 %, the editor's column 44 to 48 -> 92 to 96 %; the layer panel's first opening 42 -> 77 % and the editor's first frame 40 -> 78 % (first-open work, STATUS); resting, open-cold, zoom and strokes as before.

## G9 Practice

The author, 2026-10-23: practice is their main activity now. The app gets a practice mode, so they tap in and know what to do.
- **Course.** It follows V大's 《预科作业 3.0》 as the main line, in 8 stages matching its 8 weeks, so its videos can be watched alongside. It also takes in what works from Drawabox, Proko, Gurney, Marco Bucci and accuracy trainers.
- **Sessions.** The unit is a session of 20 to 30 minutes: warm-up from types already learnt, then the stage's main types, then a summary. After a session, another one is offered. Progress counts sessions done, never days: no quotas, streak pressure or nagging. A stage holds about 15 sessions and may be left early or revisited. Long colour copies are separate projects.
- **Exercises are scenes the app lays out itself,** never pre-made sheets. The reference is locked: it cannot be drawn on, nor picked from before the check. StreamLine is off in practice. One tap overlays the copy on its reference.
- **Scores come from the strokes' own path, pressure and time.** Each skill is measured several ways and tracked as a trajectory. Re-tests run at each stage's start and end. Optional targeted drills appear only in the summary, never in the main line.
- **Content.** Generated exercises carry their own answers. Pictures come only from packs: an official one built from CC0/PD and attributed CC BY sources, curated AI images, and renders of CC0 models, each item with its source and licence. The author's own pictures are imported locally. Nothing third-party is in the repository.
- **No accounts, community or sharing service.**
- **Method notes.** V大's methods, distilled from the course's subtitles, guide every exercise's instructions and checks. They are not in the repository.

- S75 Practice foundation is done (1.1.0): the Today card; sessions of 10 single lines and 8 single colours on a sheet the app lays out, a project the gallery never lists, its originals drawn by the screen (D98); the pen's own samples, clipped to the empty frame; Compare's overlay and numbers, stored once; Next keeping the zoom; the summary against earlier sessions; One more; records beside the artworks under format 1 (`openpaint-practice` the math). Generators the same per seed, metrics right on known cases, records round-tripping; 46 scenarios with every hash before unchanged; on the tablet a session walked through, How's card at 92 to 94 % of the vsyncs and Next's glide at 80 to 84 % (`bench practice`), the smoke 16.8 s.
- S76 Compare by overlay, and tools by the exercise. The author, 2026-10-23, after the first sessions:
  - **No numbers for lines.** A number that doesn't match what the eye sees misleads; line metrics may return later as a study of their own. The overlay is the feedback. Marking points and construction lines first is the right process, so nothing judges a first stroke.
  - **Colour keeps numbers** with a graphic that shows at a glance where it is off.
  - **The practice UI suits the exercise,** with the full UI always reachable.

  Scope:
  - **Overlay modes.** Compare overlays the original in two modes, switched with one tap:
    - in place, the frame as it is, so placement shows, as V大 checks;
    - aligned, the original moved and sized onto the copy's best fit, so the shape alone shows however far off the placement is.
  - **The overlay** is thick and clear over any copy, however rough, with nothing to fail.
  - **The correction pen.** After Compare the pen turns blue. The author corrects over the copy, compares again, and repeats as often as they like, as they practised on paper. Corrections are stored as such. Compare no longer locks an exercise.
  - **Line numbers go.** Line numbers go from the card, the summary and the records, together with the line metrics' code. The strokes stay stored, so a later study can measure them.
  - **Colour feedback:**
    - per exercise, both colours placed on the colour wheel's ring and square, with an arrow from the pick to the original, plus the hue, saturation and value differences and their direction in words;
    - in the summary, a radar of the colour judgement: hue, saturation and value accuracy, first pick against final, and low-saturation and dark colours.
  - **Tools by the exercise.** On a sheet the chrome holds only what the exercise uses, placed by it:
    - lines: a sketch pen (light), a drawing pen, and the correction pen after Compare, with undo and redo;
    - colours: the wheel and square inline beside the empty frame, picking and filling the frame in one place.

    The full UI is one tap away and back.

  Done:
  - The walk-through as a passing scenario: compare in place and aligned, correct in blue, compare again, pick and fill a colour from the inline wheel, see the arrow and the radar, open and leave the full UI.
  - The alignment tested on known shifts and scales.
  - Every other scenario hash unchanged.
  - On the tablet: a session walked through, its motions at the display's rate, the smoke, and the library restored last.
- S77 Two languages. The author, 2026-10-23: the whole UI in Chinese or English, switched in the app. Scope: every user-facing string in one table per language, the practice instructions included, a language setting stored with the toolbox, Chinese text set in a CJK font the host reads (D40), layouts that hold either language's lengths. Done: every screen in both languages without clipping (the scenarios' screenshots in both), the switch kept after reopening, scenario control names stable whichever language shows.
- Next, each specified after the one before lands:
  - Combined lines and the warm-up pool.
  - Single shapes in four tiers from the first official pack, with the pack tool and local imports.
  - Colour combinations and value scales, with the long-project space.
  - Perspective boxes, ellipses, point positioning, proportion and angle estimates.
  - Picture division, Notan and whole-picture copies.
  - Key shifts, gamut masks and abstract analysis.
  - Structure rotation against renders.
  - Trajectories, stage reports, re-tests and drills.

## G8 Independent extensions after stabilisation

Wet media, dual brushes, a brush ecosystem, animation, multi-platform, cloud sync, plugins: each is its own project and never enters the main line automatically.

## Candidates

(Findings outside the current slice, one line each, dated.)

- 2026-10-23 Practice sheets: a fill on a sheet floods its whole 2,920 x 3,160 layer before the frame cuts it (flood within the clip's box); Next's glide shows 80 to 84 % of the vsyncs while the 2x tier fills the arriving pair (fill the next pair's tiles ahead); an exported sheet lacks its originals; the StreamLine slider shows on a sheet where it does nothing.
- 2026-10-23 First openings: the layer panel's pictures draw each layer whole on a worker, a smudge's or filter's material one GPU piece of up to 45 ms, so its first opening over a dense artwork shows 77 % of vsyncs (make materials in pieces as the frame budget does); a panel's first frame lays out its text anew (15 to 20 ms of UI pass; the brush editor's whole column, 19 ms): lay out only what shows, or warm the glyphs ahead.
- 2026-10-23 `bench ui` feeds input inside the app, so the display wakes at each motion's start (no touch boost) and the vsyncs are its own clock's: a variant injecting real input and reading SurfaceFlinger's present times per motion would measure the glass; a real tap's opening shows one 20 ms frame near 0.75 s (cause not found).

- 2026-10-22 Taking ahead under a tight budget: the snapshots' room ignores the other units, so snapshots the budget let go are taken again at once while idle; a snapshot near a smudge whose residual the arena let go waits for it, and frames are wanted meanwhile (the budget test at 384 MiB; the tablet's 2.8 GB never met it). Count every unit in the room, and make a missing residual when a snapshot waits for it.
- 2026-10-20 The clocks kept up while drawing: while the GPU's clocks are low a small brush's frames come every other vsync; S67's hint session changed nothing, a busy device (a sampling shell loop) does: find what it raises (CPU idle states between the frame's waits, the memory bus) and a lever that reaches it; levers inside the app: a second frame in flight while the pen is down, a cheaper screen pass.
- 2026-10-20 Records and stamps: moved and merged content nests a level target per record and refuses beyond `MOVE_DEPTH` 16 at one place even where the moves do not overlap (levels by overlap, or more levels); `MAX_STAMPS` 48 limits recipes and the editor's ranges though the raster core lays every stamp as an instance (lifting it widens Scatter, Size jitter and Aspect, the author's call).
- 2026-10-18 Adjustment previews (STATUS pending 22): draw the reach the screen shows within the frame first, or a value only once the one before showed; retune a copy of the scene so a value never builds it anew (32 to 43 ms); an opening's slowest frame 105 to 141 ms over the kinds test artwork (its scene and the filters' materials made at once).
- 2026-10-17 Drawing: an in-progress smudge draws its new residual tiles' material from its layer every frame (take it from the drawing's layer snapshots); the lift of a smudge or a clipped stroke builds the scene (13 to 59 ms); a symmetric stroke takes its copies' tiles' snapshots again every frame (several in-progress strokes); a tier's ring tiles a stroke reaches fill after the lift; snapshots hold 2 MB a tile (Rgba16Float only if bit for bit within one core is relaxed).
- 2026-10-17 Reads of the whole (a pick, the gallery picture, a transform's lift) draw everything waiting at once on their thread; the fill's samples and smudge materials were not timed on the tablet (layer pictures: 90 to 250 ms a layer of the kinds test artwork).
- 2026-10-16 Smudges: a tile reading a residual the arena lacks waits while its material is made, a 256-px cell a frame; each smudge's quad covers its whole box (quads per residual tile it holds); wgpu-core 30.0.1 asserts in a waiting poll when another thread empties the queue, worked around by never waiting inside wgpu: report it upstream and take the waiting polls back once fixed.
- 2026-10-16 Background work: the frame loop and the raster worker both submit (2x fills' frame work p95 13 to 19 ms, waits in `submit`): fewer, larger submissions; a hand's first frame waits for the piece running and one tile more (18 to 24 ms at 8x): smaller regions while fills run if the author feels it; a later tier's atlas is made on the main thread as it starts (20 to 40 ms): smaller atlases grown in steps; the pace's first estimates come from S57's pile only.
- 2026-10-13 Memory: the driver's memory grows beside the app's (about 0.1 MB a stroke on the tablet; 4.7 to 5.2 GB in the dev hour): find what it keeps (buffers made again at each arena compaction, pen-down and lift, staging); the scene grows with every stroke: keep segment ops compact or bake old strokes; where the residuals needed together pass the binding (about 140 smudges) a frame fails: bind the arena in two buffers or make materials in bands; fjall's memtables and cache sized by device class if real sessions show them; a 10-minute session grew PSS 0.43 MB a stroke.
- 2026-10-13 Frame budget: a smudge's material tile over the busiest place (9 to 33 ms of GPU) is one piece and drops a frame or two while a fresh start makes residuals: split it; exports and filter kernels are not paced.
- 2026-10-13 Release build (author): daily painting stays on the debug build `dev.openpaint` (the device routines' `run-as` backups need it; a release-signed app would not see its library); a release-signed build, its own id and a signing key the author keeps, when the app is shared or published.
- 2026-10-12 Device loss: making the renderer again takes 3 s on the tablet (a wgpu pipeline cache would serve this and the app's start); a real driver reset was never seen; an export or picture running at the loss fails instead of running again.
- 2026-10-12 Interruptions: a platform cancel while the app stays in front drops the stroke being drawn (commit it as `leave` does, with the palm candidate below); `leave` commits the samples of the app's last frame only.
- 2026-10-02 Presentation latency: a queued frame reaches the glass p50 28 ms later (about 3.4 vsyncs) against 9 ms from pen event to queue; front-buffered stylus rendering or tighter frame timing is the lever.
- 2026-10-02 Pen stroke cancelled by a landing touch: with injected input the dispatcher cancels the pen's gesture when a touch lands mid-stroke and the stroke is dropped; if real hardware does the same, commit the stroke drawn so far.
- 2026-10-02 Layer properties: `changed_regions` for a layer property change could cover only that layer's strokes instead of the whole canvas (an opacity drag redraws the whole canvas every frame).
- 2026-10-02 Gallery: Procreate's swipe-left on a card, sorting by hand, selecting several; re-show the soft keyboard when a field is tapped after the keyboard was closed; custom canvas sizes; a finger may not scroll a long gallery (egui's default `ScrollSource`; the editor and the layer list set `ScrollSource::ALL`).
- 2026-10-02 Side bar and narrow windows: a left-handed mirror of the side bar and the islands; a hold-to-show size preview; under about 700 points wide the top islands and a notice meet, under about 450 tall the side bar runs off.
- 2026-10-02 Colour and eyedropper: touch-and-hold eyedropper; colour sliders, harmony; a contact crossing between the canvas and the reference picture samples only where it started.
- 2026-10-02 Reference and export: remember the reference window's place and zoom, store the reference with the artwork (a format change); export with a transparent background, layers or PSD; write the PNG band by band to the sink; a JPEG at 4x through a streaming encoder; faster PNG encoding (about 90 % of a 4x export).
- 2026-10-03 View: remember the view per artwork (a format change); rotation snapping; Procreate's four-finger tap to hide the interface; a flip; mipmaps for far zoomed-out views if shimmer shows.
- 2026-10-03 Fill: a fill reference layer, gap closing, keeping the threshold across artworks; the fill bar covers the canvas's top middle; a pencil's grain stops low-threshold fills; each scene rebuild uploads every fill mask again and a full-canvas flood takes about 145 ms.
- 2026-10-03 Pencil and StreamLine: breaking the paper's repeat (every ~228 x 114 canvas pixels); tilt for the paper's contact (a lead on its side touching only the ridges); keeping the raw pen path to re-smooth strokes; pressure smoothing; grain read at the output's footprint (blocky at 4x and in the 8x tier).
- 2026-10-03 Selection: Add and Remove modes, feather and antialiased edges (a coverage clip, a format change), Automatic and Ellipse, saving selections; one clip record per selection instead of its runs in every clipped stroke; the bar's buttons shift when Close appears.
- 2026-10-03 Motion and look: rows crossing on a reorder overlap their text; a rotated canvas shrinks into its card upright; egui's hover label fades in; Chinese typed on the tablet unchecked; the scenario walk-throughs before S17 say "top bar".
- 2026-10-03 Transform: Freeform, Distort and Warp; angle snapping; Fit to screen; the moved selection kept selected; the lift on a worker or derived from the records (126 to 140 ms readback for a busy layer); an antialiased selection cut; a drag preview that stays sharp; a turn handle reachable when the frame meets the top band.
- 2026-10-03 Smudge: large smudges are slow (cost grows with the radius squared): a coarser residual grid for large radii or a size cap; a smudge reading all layers (Procreate can).
- 2026-10-04 Masks: move a mask with its layer's whole-layer transform and transform it alone; masks on groups; apply a mask; a grey-only colour panel while a mask is chosen; swipe a row to delete.
- 2026-10-04 Insert: keep an inserted picture's own pixels (an asset, a format change); Procreate's Insert a file; decoding large photos in tiles.
- 2026-10-04 Colour management: PNG `gAMA`/`cHRM`/`cICP` without `iCCP`, grey and CMYK profiles; converting 16-bit PNGs before reducing them; no P3 working space and no deeper stored pixels (author).
- 2026-10-04 Adjustments and filters: Procreate's other adjustments (colour balance, gradient map, noise, liquify) and its Pencil mode; removing a curve point; Alpha Lock respected by filters; Motion and Perspective Blur; filtering content a transform brought in from beyond the canvas.
- 2026-10-05 Curves and paths: sharp corners kept where a slow pen turns hard; the harness `replay` taps undo and redo at pixels from before S17 and stops there.
- 2026-10-05 Layer panel: a row dragged past the list's end does not scroll it; a group cannot be dragged whole; a closed group takes a dragged layer only at its top or bottom; a group row shows no picture.
- 2026-10-05 Merge Down: merging a blend-mode layer that does not clip (pass-through nesting, refused now); Procreate's pinch of two rows and Combine Down; Merge Down while the mask row is chosen; unmerge.
- 2026-10-05 Copy and paste: pasting across canvas sizes; the system clipboard; Copy All of a blend-mode layer in a faded group exactly; Procreate's three-finger scrub; a copy on the mask copying the mask's grey; keeping the selection after a paste.
- 2026-10-05 Library: open a `.openpaint` from another app (an intent filter); a picture per kept version; compressing the thumbnail record (478 kB at screen size); room for longer titles on the cards.
- 2026-10-05 Pen buttons and palette: a hex keyboard; a cue on swatches in Edit mode; several palettes, naming and reordering swatches; an assignable eraser end; reading `KEYCODE_STYLUS_BUTTON_*` key events.
- 2026-10-06 QuickShape and guide: the second-finger constraint once palm rejection can tell a deliberate tap; rotational symmetry, storing the guide per artwork, grid, isometric and perspective guides; drawing mirrored copies incrementally (several in-progress strokes); a QuickShape delay setting.
- 2026-10-09 Tooling: the scenario runner's "no control is named" failure does not say which step failed; `device-restore` exits 1 whenever the app wrote fjall files after the backup; device comparisons need the same view, so routines reopen the artwork before each screenshot; `device-reset` names the first backup of the day, not the newest.
- 2026-10-09 Taper and shapes: a taper by path length per pixel (fast strokes taper in straight steps); a pressure and an opacity taper; spacing by the stroke's local diameter; stamps on the curve itself rather than its chords; stamps on paper; a dual brush.
- 2026-10-09 Brush editor and sets: StreamLine on the strip's pen strokes; the paper's values, the first size and opacity, removing a curve point; renaming and adding sets, dragging a brush onto a set word, importing and exporting brushes (Procreate's); a set longer than the screen runs off the panel; the picker's height follows its set.

## Done

Slices S0 to S72 and the stage exits: `docs/archive/ROADMAP-2026-10.md`.

- S73 2026-10-22 `chore(release): OpenPaint 1.0.0, what S72 left and the documents as the product now is (S73)`; one layer refusal, one curve mapping, the budget test on the held clock, the delete note in ink; the documents rewritten; on the tablet a fresh 1.0.0 with the smoke (15.9 s) and benches alike to S65 to S67's
- S74 2026-10-23 `perf(runtime): the interface at the display's rate, a frame every vsync while a hand or the interface moves (S74)`; `bench ui`, D97, the chrome pass over the shapes there are, a worker's paced regions 256 px; on the tablet 1.0.1, the smoke 16.9 s
- S75 2026-10-23 `feat(app): practice foundation, sessions of single lines and colours checked against their originals (S75)`; `openpaint-practice`, D98, `s75-practice`, `bench practice`; on the tablet 1.1.0, a session walked through, the smoke 16.8 s
