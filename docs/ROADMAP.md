# Roadmap

## Product definition

An Android-first, stylus-first 2D painting app that completes, on one tablet: sketch -> inking -> layered flat colour -> smudging and shaping -> selection revisions -> masks and colour overlays -> adjustments and finishing -> export -> resume after interruption. Interaction follows the user's mental model from Procreate by default, adapted to Android tablets; no copying of look, icons or assets.

Stage exits are judged on representative artworks made by the author on the tablet: W1 inking and flat colour, W2 painterly shading, W3 complex composition revision, W4 edit, restore and deliver.

Non-goals (G0 to G7): network, accounts, cloud sync, community, animation, 3D, multi-platform, plugin systems, a desktop product or desktop window of any kind.

## Rules

- A slice is a vertical cut one session can finish, verify and roll back on its own. Slices run in numbered order; no skipping.
- Every slice states its goal, the crates involved and done criteria. Done criteria are executable or observable, never "better".
- A slice that changes what the user sees or does begins with its walk-through written as the user and ends with that walk-through as a passing scenario plus a screenshot review (AGENTS.md, "Design from the user inward").
- The dev-machine loop is the harness (`apps/harness`): offscreen rendering, replay, scenarios, PNG and hash output. "Seeing" a result means reading its PNG. Real input and pen feel exist only on the tablet.
- When a slice is done: rewrite STATUS, move the slice to "Done" at the end of this file (one line), add a DECISIONS entry only if a decision was made.
- Stage-exit items that need the author on the tablet are recorded as `pass` / `fail` / `not_run`; a dev-machine result never substitutes.
- Findings outside the slice go under "Candidates"; they are not fixed in passing.
- Performance (D47): a slice is fluid at its feature's default sizes on the tablet; other numbers are measured and recorded, never done criteria; chasing them is G7 (named G5 in decisions before 2026-10-04).

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

Passed 2026-10-04: S18 to S26 and the G3 follow-ups done (see "Done"); the author tried every feature on the tablet: all work and drawing feels good (complex paintings not yet exercised in depth). S21's 60 Hz holds up to about 340 px and fails at the largest sizes (D47, kept until G7). S25 (one ring rule for text controls) and S26 (insert an image) came from the author's first try, the follow-ups (the chosen mark slides, press wash in the control's shape, Add Mask) from the exit.

## G4 Colour, adjustments, sharp rendering, library

S27 Colour management, S28 Adjustments, S29 to S36 are done (see "Done").

Render architecture (author, 2026-10-04): the truth stays operation records, pixels are only derived, discardable caches; strokes stay sharp at any scale. Steps in order, each shippable alone; not built: per-layer pyramids, disk caches, vector fill contours, stroke editing, a filter graph, 8x export.
- S29 Scale-aware evaluation and export at 1x / 2x / 4x, S30 Fill edges, S31 Transform as a record and S32 Sharp zoom are done (see "Done"); per-band binning measured cheap, so no spatial index (D55); fill edges are derived from the mask per scene (D56); a transform is a record evaluated by mapping back, the lifted patch only the drag's preview (D57); the screen shows one view tier of canvas-aligned tiles above √2 screen pixels per canvas pixel (D58).
- S33 Blur and sharpen is done: adjustment records over a derived canvas-resolution boundary (the smudge's material and arena), never baked patches (D59). The render-architecture steps are complete.

From the investigation of 2026-10-05 (fast strokes showed corners when zoomed in; adjustment previews at 36-40 values a second): S34 Smooth stroke paths and S35 Preview speed are done (see "Done").

S36 Library management (OpenPaint files, kept versions restored as copies, search; PSD and OpenRaster dropped, author 2026-10-04) and S37 Ten-bit display (the canvas and view tiers at 10 bits on a 10-bit surface the display processor composites itself; artworks stay sRGB, author 2026-10-04; exports 8-bit) are done (see "Done"). G4's slices are complete.
Exit: the author completes edit, restore and delivery (W4). Passed 2026-10-05: the author found nothing wrong on the tablet; the precision check showed the ten-bit half smooth.

## G5 Daily painting

S38 Layer actions (thumbnails, duplicate, rename, drag to reorder), S39 Merge Down (a nested record, option (a), D66), S40 Copy and paste (records, a three-finger swipe menu, D67), S41 Pen buttons, a palette and hex entry (assignable side buttons, the eraser end erasing, stored with the toolbox, D68) and S42 QuickShape and symmetry guides (held strokes snap to stored samples along the shape, mirrored copies as records, the guide session-only, D69) are done (see "Done"); G5's slices are complete. From S41 on (2026-10-05): small features share a slice, the tablet routine runs only for format, device-input or display changes and stage exits, UI-only work gets light tests.

Acceptance 2026-10-09: the author tried the G5 features on the tablet, `pass` (QuickShape judged good; Merge Down keeps refusing mode clashes, D66 option (a), the call delegated to the agent). Found: the author's pen has three side buttons, which it reports from the tip outwards as `BUTTON_STYLUS_PRIMARY`, `BUTTON_STYLUS_SECONDARY` and `BUTTON_TERTIARY`, no key events. S43 G5 follow-ups (the third side button, curves interpolated between table entries, repo-check seeing new files; D70) is done (see "Done").
Exit: the author paints a real piece on the tablet. Passed 2026-10-09: the author tried every G5 feature on the tablet (not a complex piece; QuickShape judged good).

## G6 Brushes

Direction (author, 2026-10-04 and 2026-10-09): think from the digital painter and the finished picture, never from imitating traditional media (no "oil" or "watercolour" brushes); the pencil keeps its cultural feel (grain, pressure) without simulating graphite inside. Procreate's Brush Studio is the mental model, with only the dozen settings the engine really has. Strokes stay the truth: shapes, scatter and dabs are evaluated per pixel along the path (Ciallo, SIGGRAPH 2024), never a stamp engine, so every frame = cold and sharp at any scale hold; a visible dab rhythm is optional and only for texture brushes. Research 2026-10-09 (tutorials, Wacom's and Clip Studio's guides, forums): professionals paint with 3 to 6 brushes; a hard round, a soft round, one textured brush and a tapered liner are the consensus core; square tips give the chiselled concept-art look; one brush per painting keeps it whole. Built-ins by job, about 10: Pencil (with tilt), Monoline, Inker, Hard Round, Soft Brush, Textured Round, Flat, Opaque Paint, Speckle; Smudge is the blender. Wet mixing (pick-up, drag, dilution) reads neighbours and comes after G6 (G8).

- S44 Taper and tilt is done (see "Done"): a taper stage whose end applies at the lift, tilt as a third factor of the dynamics, schema 19 (D71); Pencil with tilt, Inker, Monoline, Hard Round.
- S45 Shapes and scatter. Goal: shape outlines beyond the disc (irregular rims, a square tip turning with the stroke direction or the pen's azimuth), scatter and rotation jitter from a seed stored with the stroke, the dab rhythm as an option, all per pixel along the path; delivers Textured Round, Flat, Opaque Paint and Speckle. Sliced after S44 lands.
- S46 Brush editor. Goal: a panel grown from the brush row with the engine's settings as sliders and curves and a live preview strip the pen can draw on; duplicate and reset built-ins; brushes stored with the toolbox. Sliced after S45.
- S47 Brush sets. Goal: sets (Sketching, Inking, Painting, Texture), reorder, duplicate, delete, recent brushes; no import or export yet. Sliced after S46.
Exit: the author paints with the set on the tablet and judges each brush `pass` / `fail`.

## G7 Stability and release

Device tiers, long sessions, background/foreground, low storage, failed upgrades; GPU device-loss recovery and a VRAM budget; a device smoke scenario of at most 20 s; signed releases and a version policy. Optimisation happens here, coarse-grained, driven by measurements on the tablet; earlier slices build the simplest design that meets their budgets (D33) and leave performance ideas under "Candidates". Exit: the declared device matrix passes with no known artwork corruption or silent loss.

## G8 Independent extensions after stabilisation

Wet media, dual brushes, a brush ecosystem, animation, multi-platform, cloud sync, plugins: each is its own project and never enters the main line automatically.

## Candidates

(Findings outside the current slice, one line each, dated.)

- 2026-10-02 Vulkan staging stall (~1.5 ms every ~4,000 `write_buffer` calls on the dev machine): pack the evaluator's per-frame writes or use a staging belt if the tablet shows it.
- 2026-10-02 Finished-stroke cost per frame: cache the tile binning of finished ops per artwork version before long sessions make the plan step slow.
- 2026-10-02 Presentation latency: on the tablet a queued frame reaches the glass p50 28 ms later (about 3.4 vsyncs at 120 Hz) against 9 ms from pen event to queue (D22); front-buffered stylus rendering or tighter frame timing is the lever, not a render thread.
- 2026-10-02 Layer panel follow-ups (S9): `changed_regions` for a layer property change could cover only that layer's strokes instead of the whole canvas (an opacity drag re-evaluates the whole canvas every frame).
- 2026-10-02 Gallery follow-ups (S10): Procreate's swipe-left on a card for duplicate and delete, sorting by hand, selecting several; re-show the soft keyboard when the field is tapped after the keyboard was closed with its own button; custom canvas sizes.
- 2026-10-02 Side bar follow-ups (S11): a left-handed setting that mirrors it (one constant in `components/side_panel.rs`, the bubble side in `side_slider.rs`); a hold-to-show size preview circle.
- 2026-10-02 Brush follow-ups (S11): brush editing and import through a `BrushStore`; cache per-sample speed per stroke so a scene rebuild does not recompute it.
- 2026-10-02 Colour and eyedropper follow-ups (S12): touch-and-hold eyedropper (the author's call); on black or grey a tap on the hue ring changes no visible colour (Procreate's disc behaves alike); colour sliders, harmony.
- 2026-10-02 Narrow windows (S12): the top bar's right end holds four buttons, so on a window under about 700 points the save notice and Retry no longer fit; the side bar runs off the bottom under about 450 points.
- 2026-10-03 Eyedropper across windows (G2 follow-ups): a picking contact that crosses between the canvas and the reference picture keeps sampling only where it started.
- 2026-10-02 Reference and export follow-ups (S13): remembering the window's place, size and zoom, storing the reference with the artwork (asset store, format change), export with a transparent background, layers or PSD.
- 2026-10-03 View follow-ups (S14): remember the view per artwork (draweave did, committed once at gesture end; a format change); rotation snapping (draweave had none); Procreate's four-finger tap to hide the interface (needs a visible control first, D32); a flip; mipmaps or a reduced copy for far zoomed-out views if shimmer shows on the tablet; ask for frames by deadline instead of continuously while fingers hold.
- 2026-10-03 Fill follow-ups (S15): draweave's fill reference layer (flood one layer, fill another); gap closing; keeping the threshold across artworks and launches (toolbox state); the fill bar covers the canvas's top middle while offered; a pencil's grain stops low-threshold fills at its specks.
- 2026-10-03 Fill cost (S15): each scene rebuild re-uploads every fill mask (618 KB for a full-canvas fill), and a full-canvas flood takes about 145 ms on the tablet; cache masks on the target and flood incrementally if long sessions or fast slides show it (G7).
- 2026-10-03 Pencil and StreamLine follow-ups (S16): grain filtering by view zoom (draweave's Gauss-Hermite at the screen's footprint, only if a re-evaluation at screen resolution ever comes; the screen samples the canvas bilinearly today); breaking the paper's repeat (every ~228 x 114 canvas pixels) with offsets; tilt widening the pencil (draweave's pencil ignored it); keeping the raw pen path to re-smooth strokes; pressure smoothing.
- 2026-10-03 Visual design follow-ups (S17): the scenario walk-throughs before S17 still say "top bar" in their prose (their taps follow the islands); on windows under about 700 points the two top islands and a notice pill meet; egui's hover label fades in rather than appearing at once; Chinese typed into a title on the tablet itself (only the system font loading and the harness's Chinese title are checked); a left-handed mirror of the side bar and the islands.
- 2026-10-03 Selection follow-ups (S18): Add and Remove modes, feather and antialiased edges (a coverage clip, a format change), Automatic and Ellipse, saving selections; one clip record per selection instead of its runs in every clipped stroke; counting clip masks in history bytes; the bar's buttons shift when Close appears (centred pill).
- 2026-10-03 Motion follow-ups (S19): the chrome pass and egui's motion bookkeeping add about 1 ms of frame work while idle on the tablet (G7); rows crossing on a reorder overlap their text; a rotated canvas shrinks into its card upright; finger injection on the tablet was refused system-wide this session (pen worked), so the quick-pinch glide is checked in the harness only.
- 2026-10-03 Transform follow-ups (S20): Freeform, Distort and Warp; angle snapping while turning (draweave had only the 45° button); Fit to screen; mipmapped or bicubic resampling for strong shrinking; the moved selection kept selected instead of deselecting; lifting on a worker instead of the main thread (a full-canvas lift waits for one GPU readback); a transform still open when the process is killed (an install) is lost (leaving commits it, S26).
- 2026-10-03 Smudge follow-ups (S21): large smudges are slow (600 px: frame work p50 21 to 28 ms; the contacts' kernels grow with the radius squared, and the footprint is evaluated again over everything beneath, as for a large paint brush); a coarser residual grid for large radii, fused stamp and commit, or a size cap are the levers (author's call, STATUS). Reopening evaluates every smudge cold; residual memory (20 bytes per pixel of its tiles) is never trimmed below the arena's live blocks; a smudge reads only its own layer (Procreate can sample all layers).
- 2026-10-04 Mask follow-ups (S24): move a mask with its layer's whole-layer transform (Procreate links them) and transform a mask on its own; masks on groups; apply a mask into its layer (with merging); a grey-only colour panel while a mask is chosen (Procreate) instead of the colour's luma; swipe a mask row to delete.
- 2026-10-04 Insert follow-ups (S26): keep an inserted picture's own pixels (an asset, a format change) so later transforms resample from them, not from the layer; a turn handle that stays reachable when the frame meets the top band; Procreate's Insert a file and pasting; decoding large photos in tiles instead of whole.
- 2026-10-04 Colour follow-ups (S27): PNG `gAMA`/`cHRM`/`cICP` without `iCCP`, grey ICC profiles and CMYK JPEGs' profiles (taken as sRGB today); converting 16-bit PNGs before reducing them to 8 bits; no P3 working space (author, 2026-10-04) and no deeper stored pixels (author, 2026-10-09: curve tables interpolate instead).
- 2026-10-04 Adjustment follow-ups (S28): Procreate's other adjustments (colour balance, gradient map, noise, liquify and the rest) and its Pencil mode (adjusting by painting); removing a curve point (Reset clears the curve shown).
- 2026-10-04 Transform record follow-ups (S31): an antialiased selection cut (the moved part is cut at the mask's pixel edge, enlarged with it); a drag preview that stays sharp (the patch is bilinear until the hand lifts); selections whose masks interleave with what they move double the listed content per record (a coarse coverage map instead of rectangles in `scene_cleared.rs` if it shows); the lift's readback (126 to 140 ms on the tablet for a busy layer) only frames the content now and could be derived from the records.
- 2026-10-04 Export follow-ups (S29): write the PNG to the chosen file band by band (`ImageSink` as a writer) instead of holding the encoded file; a JPEG at 4x through a streaming encoder; PNG encoding is about 90 % of a 4x export (a faster deflate level or encoding on a second thread).
- 2026-10-05 Sharp zoom follow-ups (S32): drawing at 8x with a large Ink Pen (about 40 px) reaches p95 17.5 to 21 ms frame work (default size within the floor): bin the tier's stroke tiles from the canvas's plan or cap the in-place pixels by the pen's region; a tier near √2 holds 64 MB (tiles of 128 pixels or a ring only where the view moves would hold less); while 4 tiles a frame fill in after a pinch (0.4 to 0.8 s) frames come at about 60 Hz instead of 120; the selection marks and the pen outline are drawn over a tier as over the canvas (unchanged); far zoomed-out views still have no mipmaps.
- 2026-10-05 Blur and sharpen follow-ups (S33): Alpha Lock respected by filters; Motion and Perspective Blur, Procreate's Pencil mode; filtering content a transform brought in from beyond the canvas.
- 2026-10-05 Curve follow-ups (S34): flatten spans per tile instead of per pixel (workgroup-shared pieces) if strokes turning at every sample show on the tablet (the GPU pays about 2.8x at 4x for a random walk); sharp corners kept where a slow pen turns hard (an angle rule); the harness `replay` taps undo and redo at pixels from before S17 and stops there (name the controls).
- 2026-10-05 Preview follow-ups (S35): blur and sharpen previews reach about 30 values a second on the tablet (STATUS): the evaluator costs about 2.5 µs a tile even for three ops (load only the operands an op uses instead of the whole record, fewer registers), and the filter kernels run over the whole block rather than the tiles near content; layers under and over the previewed one are evaluated again where its content lies (a snapshot of the composite below and of the layers above); view tiers above 1x evaluate a preview in full; the snapshot stays allocated until the next structural change.
- 2026-10-05 Layer panel follow-ups (S38): a row dragged past the list's visible end does not scroll it; a group cannot be dragged whole; a closed group takes a dragged layer only at its top or bottom; a group row shows no picture; the lifted row looks like the chosen one (an ink outline on paper).
- 2026-10-05 Merge Down follow-ups (S39): a blend-mode layer that does not clip (or two layers of different modes) merges only with pass-through nesting, which leaves later adjustments no single buffer (refused now, "Modes clash"); Procreate's pinch of two rows and Combine Down; Merge Down while the mask row is chosen (the footer shows the mask's options); a snapshot preview for adjustments over a nested record (they evaluate in full); unmerge.
- 2026-10-05 Copy and paste follow-ups (S40): pasting across canvas sizes (crop or extend the copy's selection clear); the system clipboard (an image in and out); Copy All of a blend-mode layer directly in a faded group exactly (pass-through, not isolated); Procreate's three-finger scrub to clear a layer; a copy on the mask target copying the mask's grey (today the layer's paint); keeping the selection after a paste.
- 2026-10-05 Library follow-ups (S36): open a `.openpaint` file from another app (an intent filter on the extension; Android matches it unreliably); a picture per kept version; compressing the thumbnail record (478 kB at screen size, in every file and version); room for longer titles on the cards.
- 2026-10-05 Pen button and palette follow-ups (S41): a hex keyboard (the field opens the text keyboard; digits are on its second page); a cue on swatches in Edit mode (only a hint line says taps remove); several palettes, naming and reordering swatches; an assignable eraser end and a touch-and-hold eyedropper; reading `KEYCODE_STYLUS_BUTTON_*` key events if a pen sends its buttons only as keys.
- 2026-10-06 QuickShape and guide follow-ups (S42): the second-finger constraint (15-degree lines, circles, squares) once palm rejection can tell a deliberate tap from a palm; rotational symmetry, storing the guide per artwork (a format change), 2D grid, isometric and perspective guides; drawing a stroke's mirrored copies incrementally in the renderer (several in-progress strokes) instead of re-evaluating them whole every frame if long strokes or smudges with the guide on show it on the tablet; Procreate's QuickShape delay setting.
- 2026-10-09 Tooling follow-ups (S43): the scenario runner's "no control is named" failure does not say which step failed (the agent bisected); `device-restore` exits 1 whenever the app wrote files after the backup (fjall's orphaned files are left and listed; lsm-tree drops them on the next open); a device before/after comparison needs the same view, so routines reopen the artwork from the gallery before each screenshot.
- 2026-10-09 Taper and tilt follow-ups (S44): mirrored copies (S42) show their end taper while the original is drawn (they are committed strokes; several in-progress strokes in the renderer would fix it); a smudge's end is never tapered (its residual is laid as drawn); fast strokes taper in straight steps between knots (a taper by path length per pixel would need lengths in the shader); a pressure taper and an opacity taper (Procreate's); tilt for the paper's contact (a pencil on its side touching only the ridges); azimuth as a stage (S45).
- 2026-10-02 Pen stroke cancelled by a landing touch: with injected input the dispatcher cancels the pen's gesture when a touch lands mid-stroke and the stroke is dropped; if real hardware does the same, commit the stroke drawn so far on such a cancel.

## Done

(One line per slice: id, date, commit. At 30 lines, move to `docs/archive/ROADMAP-<YYYY-MM>.md`.)

- S0 to S17, the G2 exit and the G2 follow-ups (2026-10-02 to 03): `docs/archive/ROADMAP-2026-10.md`
- S18 to S37, the G3 exit, the G3 follow-ups and the G4 exit (2026-10-03 to 05): `docs/archive/ROADMAP-2026-10.md`
- S38 2026-10-05 `feat(app): layer pictures, duplicate, rename and drag to reorder (S38)`; pictures = the layer alone reduced by box means on the GPU (tests), out of date only after a lift or an edit; duplicate, rename and move one undo step each, incremental = cold; no format change; merge down left to the author (D65)
- S39 2026-10-05 `feat(core): merge down as a nested layer record, pixel-identical, schema 15 (S39)`; clipped merges bit for bit at every mode, opacity and mask on the GPU, over-merges of one composing mode within one level (254 of 6.1 M channels), refused with why elsewhere; edits after the merge act on the whole; the author's library upgraded pixel-identical on the tablet and restored byte-identical (D66)
- S40 2026-10-05 `feat(app): copy and paste through a three-finger swipe menu, pasted content as records, schema 16 (S40)`; a copy = its source through its mask bit for bit on the GPU, Copy All over the paper within one level, incremental = cold, cut exact; the author's library upgraded on a copy and on the tablet (AE 0), restored byte-identical (D67)
- S41 2026-10-05 `feat(app): assignable pen buttons, a palette and hex entry, stored with the toolbox (S41)`; held tools come back after the lift, presses act once, the eraser end erases (app tests), schema 17 toolbox only; on the tablet injected button states, the eraser end and the real soft keyboard typing a hex worked, library restored byte-identical; real buttons to be confirmed by the author's pen (D68)
- S42 2026-10-06 `feat(app): QuickShape snaps held strokes, symmetry mirrors strokes as records (S42)`; recognition strict on hand-like and fast fixtures, stored samples on the exact shape with the drawn pressure, draw, snap and edit one step, copies one step, every frame = cold on the GPU; no format change; on the tablet a held line and ellipse snapped, a handle edited, radial copies and the centre moved by a finger, library restored byte-identical (D69)
- G5 exit 2026-10-09: the author tried every G5 feature on the tablet, `pass` (QuickShape judged good)
- S43 2026-10-09 `feat(app): a third pen side button, curves interpolated between table entries, repo-check scans new files (S43)`; `BUTTON_TERTIARY` is Side button 3 (Undo at first), schema 18 toolbox only; a steep curve over a soft ramp steps by 1 level / 1 code instead of 4 / 16 (GPU test); repo-check lists untracked files, the QuickShape tests split; on the tablet the author's library upgraded 17 -> 18 and an injected tertiary hover undid, the backup's members restored byte-identical with two orphaned fjall files left (D70)
- S44 2026-10-09 `feat(render): taper at stroke ends and tilt in the dynamics, three inking brushes, schema 19 (S44)`; an Inker line within a pixel of its taper at 5/50/95 %, every frame before the lift = the stroke without its end taper cold, the lift's frame = the commit cold (canvas and view tiers), tilt by its curves' factors (GPU tests); the author's pen leans 45 degrees at the median, 61 at p90; every older scenario hash unchanged; on the tablet the library upgraded 17 -> 18 -> 19 with the last artwork AE 0, an injected tilted Pencil line wider and greyer, restored byte-identical with two orphaned fjall files left (D71)
