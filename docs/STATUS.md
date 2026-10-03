# Status

Updated 2026-10-03. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 passed 2026-10-03: the author drew on the tablet and judged W1 `pass` ("really good; almost no problems while drawing; the UI is beautiful, steady and has style"). The G2 follow-ups (loupe, outline while pressing, gesture-driven reference window; D41) are done. Next: S18 Selection (G3). G1 exit passed 2026-10-02.

## Working features

- Look (S17, D32, D40): paper-white line art. The white canvas shows on the whole screen and at fit lies on a light warm-grey desk with a soft shadow, under a 48-point top band. Nothing spans the width: a paper island outlined in ink at the top left holds Gallery and Actions (a spanner); one at the top right holds Brush, Eraser, Layers and the colour button (Procreate's order); the side bar island floats over the left edge, centred: the size slider (small dot below, large dot above), the eyedropper, the opacity slider (faint disc below, solid disc above), undo and redo. The active state is a deeper ink ring (tool, open panel, eyedropper, colour) or an ink outline (chosen row); pressing shows a faint wash. Icons are self-drawn ink lines, 1.5 points, round joins. A finger draws nothing (D18). One panel (actions, colour, layers, brushes) at a time.
- Text (S17): the system's sans (Roboto on the tablet) and Chinese face (Noto Sans CJK), both at weight 400, handed over by the Kotlin host from `/system/etc/fonts.xml`; 18-point body, 22-point headings. Without a system face the built-in one shows Latin text and Chinese shows as boxes (logged).
- Notices (S17): a failed save or open, a refused fill, export progress and results and the reference image's state are a pill in the top band left of the right island; the fill bar (ColorDrop) and the view's zoom are pills in the band's middle.
- Pen outline (S17, D41): hovering over open canvas, the current brush's outline at full size (at least 2 points across), one 1-point line, black over light and white over dark per pixel. Pressed with the brush: no outline (the ink is the feedback). Pressed with the eraser: the outline at the radius the stroke has at the pen's pressure, on the stroke's end (`render::tip_radius`). Hidden over controls, with a finger down and while picking. Hovering over an icon button, a swatch or a side slider shows its name.
- Eyedropper (S12, D41): the side bar's button; a contact (pen or finger) on the canvas or the reference picture shows the loupe, 76 points up and right of it (below it near the top): the colour under it over the current colour, which stays until the lift. Over a control or off the picture nothing new is previewed and the loupe follows. The lift commits the last colour previewed (joins the recent ones, the tool before returns); a lift with nothing previewed keeps the eyedropper on.
- Reference window (S13, D41): no title bar; a tap on it (finger or pen) shows a slim bar (move grip, ×) and a corner resize mark, hidden after 3 s without a touch on the window or by a touch outside. Dragging the bar moves the whole window; the mark resizes it (240 x 180 points to 85 % of the screen). In the picture one finger pans past 8 points and two fingers pinch-zoom (1x to 8x) about their middle with the canvas's dead zones; the canvas never moves with them, and the pen never paints there. No +/- buttons.
- Side sliders (S17): while held, a bubble shows the value and the brush's circle at its real size (size) or a swatch at that opacity (opacity). Lists have no scroll bar at rest: the last row is cut and fades; a thin line shows only while scrolling.
- Brushes (S16, D38): the Pencil is draweave's graphite on canvas-fixed paper, darkening where it crosses itself up to its opacity; the Soft Brush builds up under a bell tip (so does the eraser on it); the Ink Pen covers evenly. StreamLine (S16, D39) under the brush panel's rows, 0 % at first, per brush and tool.
- Fill (S15, D36, D37): drag the colour button onto the canvas; lifting fills the active layer's connected region (threshold 60 % at first, hold still 0.4 s and slide to adjust); after a drop the fill bar offers "Continue filling". One undo step; artwork content; hidden or locked layers refuse with a notice.
- View (S14, D35): every artwork opens at fit (92 % of the area under the top band); two fingers pan, pinch-zoom and rotate; quick pinch fits; finger taps: two undo, three redo.
- Actions (S13, D31): Save/Share PNG/JPEG, the Reference switch and picker. Tools and colour (S12, D30), brushes (S11, D27), layers (S9, D23), gallery (S10, D25), palm rejection (S8), autosave (S7), reopen where the app was left.
- Code (G2 follow-ups): app `ToolIntent::{Preview, Pick}`, `BrushProjection::preview`, `gestures/picture.rs` (`PictureGestures`); render `tip_radius`; ui `components/loupe.rs`, `components/floating.rs` (bar and resize mark, raw routing by the owner), `reference_window.rs` (place, size, zoom, bar), `reference_touch.rs` (contacts from the raw batches before the canvas), `pen_marks.rs` (outline and loupe), icons `Move`, `Resize` (no `Minus`); runtime `Canvas::erasing_tip`, `Shell::cursor(erasing)`; devtools `press` takes a `pressure`.
- Harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference` (steps follow the new window), `s14-view-gestures`, `s15-fill`, `s16-stabilisation-pencil`, `s17-visual-design`, `g2-followups`. At 2880 x 1800 and 1.75: Gallery (100, 59), Actions (226, 42), Brush (2542, 59), Eraser (2633, 59), Layers (2724, 59), colour (2815, 59); brush rows Pencil (2450, 268), Ink Pen (2450, 380), Soft Brush (2450, 492); add layer (2808, 170); side bar undo (70, 1315), redo (70, 1413), eyedropper (70, 837); size track 485.7 to 730.7 px, opacity track 947.7 to 1192.7 px (top = 1); Continue filling (1483, 59); a 480 x 320 reference first lies at (2222, 1352) to (2866, 1786) px, its bar 84 px high, × 84 px from its right end.
- Device input (debug builds): `am instrument -w --no-restart -e tool stylus -e pressure P -e path x,y,... dev.openpaint/.GestureInstrumentation` draws one pen path (a repeated point holds it, 8 ms or more per point); `-e hover x,y -e ms N` holds the pen above the glass; `-e from/-e to/-e ms` for fingers. Run one instrumentation at a time; `screencap` may run beside one.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check. Pins (D14) unchanged.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| G2 exit (W1), the author on the tablet | pass | 2026-10-03 | "really good; almost no problems while drawing; the UI is beautiful, steady and has style"; the follow-ups below came from it |
| `cargo gate` (dev machine, Windows) | pass, 38.7 s warm, 349 tests | 2026-10-03 | exit 0 |
| Every scenario (debug) | pass | 2026-10-03 | no hash re-recorded; `s13-export-reference` keeps its blue-line hash with the pick through pinch and pan; `g2-followups` 2 of 2 checkpoints and views |
| G2 follow-ups, harness screenshots | reviewed against section 10 | 2026-10-03 | `artifacts/scenarios/g2-followups/`: reference (no title bar), bar, moved, bar-hidden, pinched (canvas view still at fit), panned, loupe-reference, picked-reference, loupe-drifted (black kept over the island), picked-canvas, eraser-hover, eraser-pressed (smaller at pressure 0.4), brush-pressed (no outline), closed |
| G2 follow-ups on the tablet (injected input) | pass | 2026-10-03 | MovinkPad 14 on a test artwork: `g2f-reference`, `g2f-bar`, `g2f-moved`, `g2f-bar-hidden`, `g2f-pinched` (canvas unmoved), `g2f-panned`, `g2f-loupe-reference`, `g2f-picked-reference` (blue), `g2f-picked-canvas` (brown), `g2f-loupe-drifted` and `g2f-picked-drifted` (blue kept over the island), `g2f-eraser-hover`, `g2f-eraser-pressed`, `g2f-brush-pressed`, `g2f-closed`; no panic |
| The author's library in this build | pass | 2026-10-03 | backup `artifacts/device/g2f-library-before.tar` (3.4 MB); the library pulled after install and opening equals it byte for byte; no format change |
| State left on the tablet | as found | 2026-10-03 | test artwork deleted in the gallery; library and toolbox restored in place from the backup (byte-identical tar); `g2f-final.png` = `g2f-after-install.png`; test picture folder removed; this debug build installed; rotation untouched |
| Hover outline, render | pass | 2026-10-03 | GPU test: white over the dark band, black over paper, white over a dark surround, the inside untouched |
| Deposit, StreamLine, brush goldens, schema 6, `replay` | pass | 2026-10-03 | S16 results; rendering of artwork unchanged since |
| Loupe, eraser outline and reference gestures with the real pen and fingers | not_run | — | the author judges on the tablet |
| Pen latency, palm rejection on the tablet | not re-measured since S15 | 2026-10-03 | S15 build: event to present call p50 9.1 to 9.5 ms |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Layout: on windows under about 700 points wide the two top islands and a notice pill meet; under about 500 points tall the side bar runs off the bottom; nothing is mirrored for the left hand. The side bar covers about 130 px of the canvas's left edge at fit.
- Hover: egui's hover label fades in rather than appearing at once; the outline is drawn only in the canvas blit (under the islands it is hidden by them). Chinese typed on the tablet itself is unchecked.
- Eyedropper: a contact that starts on the canvas does not sample the reference when it crosses into the window, nor the reverse; near the screen's top the loupe drops below the contact and can cover the top-left island.
- Reference window: place, size and zoom are kept for the app's run only; its bar keeps frames coming while it shows (3 s).
- Fonts: one weight (400) everywhere; the harness's Segoe UI differs a little from the tablet's Roboto.
- Pencil: the paper repeats every ~228 x 114 canvas pixels; no tilt response; far zoomed out the bilinear screen may shimmer; at a low opacity a heavy pencil reaches the cap.
- StreamLine: the stored stroke is the smoothed path; no corner smoothing; pressure never smoothed.
- Fill: no fill reference layer, no gap closing, no fill inside a selection; the threshold resets with each artwork session; a pencil's grain stops a low-threshold fill at its specks; a full-canvas flood takes about 145 ms on the tablet; every scene rebuild re-uploads all fill masks.
- View: not stored per artwork; no rotation snapping, no four-finger tap, no flip; no mipmaps.
- A held two- or three-finger shortcut or a held drop asks for frames continuously.
- Injected gestures: `input` has constant pressure; a self-targeting instrumentation needs `--no-restart`; `input motionevent` at the top edge can pull the system bar; a hover without its exit leaves the input dispatcher refusing the next hover enter.
- Export: the flattened canvas only, on white. Reference: one picture for the app's run.
- Brushes: three built-ins; no brush editing. Colour: HSV wheel and recents only. Alpha Lock captured per stroke and per fill.
- Gallery: duplicate and delete are card buttons, not swipe; order by last change only. Layer panel: reorder by buttons only.
- Palm rejection is basic; with injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped.
- Presentation dominates latency (D22). Blending is in sRGB-encoded values. A save rewrites the head every time.
- `leave` waits at most 2 s; a stroke still being drawn when the app is left is not saved.
- GPU tests fail (not skip) without an adapter. Golden hashes verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls. Each canvas target holds two extra float textures (about 40 MB at 2880 x 1716) and the core 2.8 MB of paper; the Chinese face adds about 32 MB.
- egui gets pointer events from the first contact of each batch only; multi-finger gestures (canvas and reference window) are read from the batches directly.
- Scenarios: button positions are window pixels; screenshots are never compared; the system's dialogs are played by devtools; walk-throughs before S17 still say "top bar", and S12's still says the colour button changes while picking.

## Pending author decisions

1. Settled after use (kept as recommended, 2026-10-03): option A look and the system sans (D40); the hover outline at full size; draweave's pencil and the Soft Brush build-up (D38); StreamLine 0 % by default (D39); Continue filling and the session threshold (D37); the view not remembered per artwork, no rotation snapping, no four-finger toggle (D35); one-finger pan on open canvas; the layer panel's buttons and footer (D23); the gallery's card buttons and Rename dialog (S10); the brush panel staying open, square size curve, Ink Pen default (D27); the reference kept for the app's run (D31). Changed by the follow-ups instead: the eyedropper's feedback (loupe, commit on lift) and the outline while pressing (D41).
2. Touch-and-hold eyedropper and the pen's eraser end or buttons: still unmapped (D30). Recommended: decide with the next gesture work.
3. New: picking across the canvas and the reference window (a contact sampling whichever it is over). Recommended: keep apart until asked.
4. New: the reference window's bar hides after 3 s. Recommended: keep; one constant (`BAR_IDLE_MS`).
