# Status

Updated 2026-10-03. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 slices done: S9 to S15 (S15: fill). Before the G2 exit (W1) come S16 and S17 (stabilisation and pencil, visual design; ROADMAP, D32 to D35). Next slice: S16. G1 exit passed 2026-10-02.

## Working features

- The canvas: a dark top bar with Gallery and Actions (a spanner) at its left end and, at its right end, right to left, the colour button, Layers, Eraser and Brush; below it the canvas area, where the artwork shows through the view on a dark surround. Over the area's left edge, centred on it, floats the side bar: the size slider, the eyedropper, the opacity slider, then undo and redo. A finger draws nothing (D18). One panel (actions, colour, layers, brushes) at a time.
- Fill (S15, D36, D37): press the colour button (finger or pen) and drag onto the canvas: after 8 pt a drop of the colour follows and a bar under the top bar reads "Lift to fill. Hold still to adjust."; lifting over the canvas fills the connected region of the active layer around that point (four neighbours, compared with the seed's colour, alpha included, threshold 60 % at first, plus one edge ring under antialiased lines; no gap closing). Held still 0.4 s it anchors: the preview shows, the bar reads "Threshold 60.0 % · slide to adjust", sliding sideways changes it ((|dx| - 4) / 600 per point, 0.5 % steps), the last preview stays until the next is computed; lifting keeps the newest. Lifting over a control or off the canvas cancels. After a drop the bar offers "Continue filling": then each pen tap on the canvas fills (fingers still navigate) until Done or a tool tap. A fill is one undo step and artwork content (reopens exactly). A hidden or locked active layer says "Can't fill: the layer is hidden/locked." with OK; with Alpha Lock a fill colours only the layer's paint. The threshold is kept for the session, not stored.
- View (S14, D35): every artwork opens at fit (the whole canvas, centred, upright, 92 % of the canvas area); zoom is relative to fit (0.25x to 8x), so rotating the tablet refits at the same zoom. Two fingers pan, pinch-zoom and rotate about the point between them, each past its own dead zone (pan 8 pt, zoom |ln| 0.04, rotation 0.14 rad to lead; 18 pt, 0.10, 0.24 rad to join); one finger on open canvas pans after 8 pt; a quick inward pinch fits. While and 1 s after the view moves, a quiet line under the top bar reads the size and turn ("283 % · 37°").
- Finger taps on open canvas (S14): two undo, three redo; held still they repeat. A control taking a contact resets every finger tracker. The pen draws only from the canvas itself, through the view's inverse; the eyedropper picks through the same map.
- Undo, redo and every control a stroke blocks keep their look while the pen draws (`Availability::is_offered`); they grey only when there is nothing to act on.
- Canvas size (S14): the gallery's "+" opens New canvas: Screen size, Square 2048, A4 portrait, A4 landscape.
- Actions (S13, D31): Save/Share PNG/JPEG, the Reference switch and picker. Reference image (S13): a floating window, kept for the app's run. Tools and colour (S12, D30), brushes (S11, D27), layers (S9, D23), gallery (S10, D25), palm rejection (S8), autosave (S7), reopen where the app was left.
- core (S15): `RegionMask`, `RegionEdit`, `Layer::edits`, `Artwork::fill_region`, `FillParts`. raster (S15, new crate): `connected_region`. render (S15): opcode `Fill`, `SceneOps` with the fill masks, `Core::layer_pixels`, fills in `changed_regions`. app (S15): `fill.rs` (`FillIntent`, `FillJob`, `FillProjection`, `FillRefusal`, `DEFAULT_THRESHOLD`), `Session::{shown, fill_request, fill_finished, filling}`. runtime (S15): `fill.rs`, the `openpaint-fill` worker. ui (S15): `color_drop.rs`, `components::{fill_bar, carried_drop}`, the fill notice. store (S15): schema 5, `fills.rs`, LEB128 varints in `codec.rs`.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference`, `s14-view-gestures`, `s15-fill`. Step added in S15: `press` (one contact through `[x, y, ms]` keys, holds where keys share a place, `before_lift` screenshot). At 2880 x 1800 and 1.75: colour button (2810, 42), the colour square's top-right (2690, 330), the fill bar's buttons Continue filling (1483, 141) and Done (1568, 141), the top-bar notice's OK (2412, 42); side bar undo (70, 1271), redo (70, 1369).
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check. Pins (D14) unchanged.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 37.0 s warm, 317 tests | 2026-10-03 | exit 0 |
| `cargo scenario s15-fill` (debug) | pass, 18 of 18 checkpoints and views | 2026-10-03 | screenshots reviewed (`artifacts/scenarios/s15-fill/`): red-chosen, dropping (drop and hint bar), circle-filled-screen (filled to the line, Continue filling offered), continuing, done-continuing, leaked (gap lets it through), holding-preview (Threshold 60.0 %), threshold-raised (100 %, lines taken), refused-hidden (notice, no offer), shown-again, reopened-screen |
| Fill behaviour (raster, core, render, runtime tests) | pass | 2026-10-03 | holes and pinholes kept, transparency vs faint paint vs line art, diagonal gaps closed, open gaps and one-pixel breaks leak, soft edge filled under its rising side only; GPU: exactly the mask, order among strokes, Alpha Lock, incremental = cold after fill and undo, layer sampling; frame loop: a drop into an antialiased circle fills to the line with no light pixel inside |
| Schema 5 | pass | 2026-10-03 | `schema-5.bin` byte-exact (1448 bytes, two fills); `schema-1..4.bin` open upgraded with every record kept |
| `replay`, release build | pass | 2026-10-03 | undo/redo taps re-aimed at the side bar; golden re-recorded (`c3320cf50b5727af`): since S14 the pen lands through the fit view; drawing frames max 1.6 ms, taps max 2.3 ms (budget 8) |
| S15 on the tablet (injected `input stylus`/`motionevent`) | pass | 2026-10-03 | MovinkPad 14, a test artwork: pencil square, quick drop at 60 % leaked through the pencil's grain (`s15-drop-filled.png`), undo; hold and slide left to 22.5 % contained it (`s15-threshold-lowered.png`, `s15-lowered-filled.png`); the layer was sampled once per drop, each flood about 145 ms (logcat); Continue filling and a pen tap (`s15-continue-tap.png`), Done, undo; hidden layer refused (`s15-refused-hidden.png`); force-stop and reopen: the canvas pixel-identical (AE 0); no panic or error in logcat |
| The author's artwork in the S15 build | pass | 2026-10-03 | backup `artifacts/device/s15-library-before.tar`; the 4 -> 5 upgrade ran on the tablet ("upgraded from schema 4 (1 projects, 38 strokes)") and the screen at fit equals the S14 build's (AE 0, `s15-after-install.png` against `s15-before-install.png`) |
| State left on the tablet | as found | 2026-10-03 | test artwork deleted; the library restored from the backup (byte-identical tar, toolbox colour back to black) and upgraded again by the final build (AE 0 against the S14 screen, `s15-final.png`); rotation free; S15 build installed |
| Gesture and fill feel with real fingers; pen, brushes, colour, panels, export and reference by hand | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-03 | event to present call p50 9.1 to 9.5 ms in the S15 build |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Fill: no fill reference layer, no gap closing, no fill inside a selection; the threshold resets with each artwork session; a pencil's grain stops a low-threshold fill at its specks; a full-canvas flood takes about 145 ms on the tablet, so a fast slide shows a few previews per second; every scene rebuild re-uploads all fill masks; the fill bar covers the canvas's top middle while it shows.
- View: not stored per artwork; no rotation snapping, no four-finger tap, no flip; bilinear without mipmaps, so far zoomed out thin lines shimmer.
- A held two- or three-finger shortcut or a held drop asks for frames continuously.
- Injected gestures: constant pressure; a self-targeting instrumentation needs `--no-restart`; `input motionevent` at the top edge can pull the system bar.
- Export: the flattened canvas only, on white. Reference: one picture for the app's run.
- Brushes: three built-ins; coverage is a union (D16). Colour: HSV wheel and recents only. Alpha Lock captured per stroke and per fill.
- The side bar covers about 126 px of the area's left edge and is not mirrored; on windows under about 450 points it runs off the bottom.
- Gallery: duplicate and delete are card buttons, not swipe; order by last change only. Text: no CJK glyphs. Layer panel: reorder by buttons only.
- Palm rejection is basic; with injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped.
- Presentation dominates latency (D22). Blending is in sRGB-encoded values. A save rewrites the head every time.
- `leave` waits at most 2 s; a stroke still being drawn when the app is left is not saved.
- GPU tests fail (not skip) without an adapter. Golden hashes verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- egui gets pointer events from the first contact of each batch only; multi-finger gestures are read from the batches directly.
- Scenarios: button positions are window pixels; screenshots are never compared; the system's dialogs are played by devtools.

## Pending author decisions

1. G2 exit (W1), after S17: finish an inked and flat-coloured piece on the tablet and judge with the real pen and fingers the brushes, eraser, colour, eyedropper, panels, export, reference, the gestures and now ColorDrop (hold time, slide speed, the 60 % default). Record `pass` / `fail` here.
2. Fill (S15, D37): Continue filling is a bar offered after each drop and ends on Done or a tool tap; the threshold is session state; no fill reference layer. Recommended: keep until used; the reference layer and a stored threshold are ROADMAP candidates.
3. View (S14, D35): not remembered per artwork, no rotation snapping, no four-finger interface toggle. Recommended: keep until used.
4. One-finger pan on open canvas (draweave's behaviour; Procreate paints or smudges with a finger). Recommended: keep, since a finger never paints (D18).
5. Layer panel (S9, D23): reorder by buttons, opacity and Alpha Lock in the footer. Recommended: keep until used.
6. Gallery (S10): duplicate and delete as card buttons; Rename as a dialog; order by last change only. Recommended: keep until used.
7. Brushes (S11, D27): the panel stays open after a choice; square size curve; Ink Pen default. Recommended: keep until drawn with.
8. Touch-and-hold eyedropper (S12, D30): still only the side-bar button. Recommended: add it with the next gesture work if wanted.
9. The pen's eraser end (S12): does nothing. Recommended: erase with the eraser's brush.
10. Reference image (S13, D31): kept for the app's run only. Actions entry (S13): four rows for Save and Share. Recommended: keep until used.
