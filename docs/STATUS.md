# Status

Updated 2026-10-03. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 slices done: S9 to S17 (S17: visual design, D40). Next: the G2 exit (W1), the author's inked and flat-coloured piece on the tablet. G1 exit passed 2026-10-02.

## Working features

- Look (S17, D32, D40): paper-white line art. The white canvas shows on the whole screen and at fit lies on a light warm-grey desk with a soft shadow, under a 48-point top band. Nothing spans the width: a paper island outlined in ink at the top left holds Gallery and Actions (a spanner); one at the top right holds Brush, Eraser, Layers and the colour button (Procreate's order); the side bar island floats over the left edge, centred: the size slider (small dot below, large dot above), the eyedropper, the opacity slider (faint disc below, solid disc above), undo and redo. The active state is a deeper ink ring (tool, open panel, eyedropper, colour) or an ink outline (chosen row); pressing shows a faint wash. Icons are self-drawn ink lines, 1.5 points, round joins. A finger draws nothing (D18). One panel (actions, colour, layers, brushes) at a time.
- Text (S17): the system's sans (Roboto on the tablet) and Chinese face (Noto Sans CJK), both at weight 400, handed over by the Kotlin host from `/system/etc/fonts.xml`; 18-point body, 22-point headings. Chinese titles show real glyphs. Without a system face the built-in one shows Latin text and Chinese shows as boxes (logged).
- Notices (S17): a failed save or open, a refused fill, export progress and results and the reference image's state are a pill in the top band left of the right island; the fill bar (ColorDrop) and the view's zoom are pills in the band's middle. At fit none covers the drawing.
- Pen hover (S17): over open canvas the current brush's outline at full size (at least 2 points across) follows the hovering pen, one 1-point line, black over light and white over dark per pixel; it hides over controls, with a finger down and while picking. Hovering over an icon button, a swatch or a side slider shows its name.
- Side sliders (S17): while held, a bubble beside the bar shows the value and the brush's circle at its real size on the canvas (size) or a swatch of the colour at that opacity (opacity). Lists (layers, gallery) have no scroll bar at rest: the last row is cut partway and fades; a thin line shows only while scrolling.
- Brushes (S16, D38): the Pencil is draweave's tuned graphite on canvas-fixed paper, darkening where it crosses itself up to its opacity; the Soft Brush builds up under a bell tip (so does the eraser on it); the Ink Pen covers evenly. StreamLine (S16, D39) under the brush panel's rows, 0 % at first, per brush and tool.
- Fill (S15, D36, D37): drag the colour button onto the canvas; lifting fills the active layer's connected region (threshold 60 % at first, hold still 0.4 s and slide to adjust); after a drop the fill bar offers "Continue filling". One undo step; artwork content; hidden or locked layers refuse with a notice.
- View (S14, D35): every artwork opens at fit (92 % of the area under the top band); two fingers pan, pinch-zoom and rotate; quick pinch fits; finger taps: two undo, three redo.
- Actions (S13, D31): Save/Share PNG/JPEG, the Reference switch and picker. Tools and colour (S12, D30), brushes (S11, D27), layers (S9, D23), gallery (S10, D25), palm rejection (S8), autosave (S7), reopen where the app was left.
- ui (S17): `design/palette.rs` (the one palette), `paint.rs` (panel frame, marks, ink lines, sheet shadow), `fonts.rs` (`TextFonts`, `FontFile`), icons `SizeSmall/Large`, `OpacityLight/Dark`; components `island`, `pill` (no `top_bar`), `CanvasArea { screen, fit }`, `side_slider` with ends and `Preview`; `drop_bar.rs`; `CanvasInput::hover`, `Shell::cursor`. render (S17): `BrushCursor` in `CanvasPlacement`, 64-byte blit uniform. runtime: `set_text_fonts`. android: `fonts.rs` (`read_face`), `Native.fonts`, `SystemFonts.kt`. devtools: scenario step `hover`.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference`, `s14-view-gestures`, `s15-fill`, `s16-stabilisation-pencil`, `s17-visual-design`. The harness stands in Segoe UI and Noto Sans SC for the tablet's faces. At 2880 x 1800 and 1.75: Gallery (100, 59), Brush (2542, 59), Eraser (2633, 59), Layers (2724, 59), colour (2815, 59); brush rows Pencil (2450, 268), Ink Pen (2450, 380), Soft Brush (2450, 492); add layer (2808, 170); side bar undo (70, 1315), redo (70, 1413), eyedropper (70, 837); size track 485.7 to 730.7 px, opacity track 947.7 to 1192.7 px (top = 1); Continue filling (1483, 59).
- Device input (debug builds): `am instrument -w --no-restart -e tool stylus -e pressure P -e path x,y,... dev.openpaint/.GestureInstrumentation` draws one pen path; `-e hover x,y -e ms N` holds the pen above the glass; `-e from/-e to/-e ms` for fingers. Run one instrumentation at a time: a second one starting while the first finishes kills the app's process.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check. Pins (D14) unchanged.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 37.4 s warm, 337 tests | 2026-10-03 | exit 0 |
| Every scenario (debug) | pass | 2026-10-03 | `s17-visual-design` 3 of 3 checkpoints; taps moved to the islands; hashes re-recorded only in `s9-layers`, `s11-brushes`, `s16-stabilisation-pencil`, where a side or layer slider drag lands on a value about 1e-6 away under the new layout: every changed pixel differs by one level (checked) |
| S17 harness screenshots | reviewed against section 10 | 2026-10-03 | `artifacts/scenarios/s17-visual-design/`: fit, brush-panel, canvas, hover-outline, hover-label, size-preview, opacity-preview, dropping, fill-offered, layer-panel (cut row, no bar), colour-panel, rename-dialog and gallery with a Chinese title, reopened-gallery. Style options A, B, C: `artifacts/style/` |
| Hover outline, render | pass | 2026-10-03 | GPU test: white over the dark band, black over paper, white over a dark surround, the inside untouched |
| S17 on the tablet (injected input) | pass | 2026-10-03 | MovinkPad 14: `s17-fit`, `s17-brush-panel`, `s17-canvas`, `s17-hover-outline`, `s17-hover-label` ("Brushes"), `s17-size-preview`, `s17-opacity-preview`, `s17-colour-panel`, `s17-fill-offered` (the pill on the desk), `s17-layer-panel` (17 layers, cut row), `s17-gallery`; Roboto (2.4 MB) and Noto Sans CJK (32.4 MB) loaded; no panic |
| The author's artwork in the S17 build | pass | 2026-10-03 | backup `artifacts/device/s17-library-before.tar`; canvas region of `s17-after-install.png` = `s17-before-install.png` pixel for pixel (only the chrome changed); no format change |
| State left on the tablet | as found | 2026-10-03 | test artwork gone: library restored from the backup (byte-identical tar); `s17-final.png` = `s17-after-install.png`; S17 debug build installed |
| Deposit, StreamLine, brush goldens, schema 6, `replay` | pass | 2026-10-03 | S16 results; S17 changed no rendering of artwork (render goldens unchanged) |
| Look, fonts, pencil, StreamLine, gestures and fill with the real pen and fingers | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | not re-measured since S15 | 2026-10-03 | S15 build: event to present call p50 9.1 to 9.5 ms |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Layout: on windows under about 700 points wide the two top islands and a notice pill meet; under about 500 points tall the side bar runs off the bottom; nothing is mirrored for the left hand. The side bar covers about 130 px of the canvas's left edge at fit.
- Hover: egui's hover label fades in rather than appearing at once; the outline is drawn only in the canvas blit (under the islands it is hidden by them). Chinese typed on the tablet itself is unchecked (`adb input` cannot type it).
- Fonts: one weight (400) everywhere; the harness's Segoe UI differs a little from the tablet's Roboto.
- Pencil: the paper repeats every ~228 x 114 canvas pixels; no tilt response; far zoomed out the bilinear screen may shimmer; at a low opacity a heavy pencil reaches the cap. On the tablet the toolbox may still hold test values (the pencil's opacity looked low).
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
- egui gets pointer events from the first contact of each batch only; multi-finger gestures are read from the batches directly.
- Scenarios: button positions are window pixels; screenshots are never compared; the system's dialogs are played by devtools; walk-throughs before S17 still say "top bar".

## Pending author decisions

1. G2 exit (W1): finish an inked and flat-coloured piece on the tablet and judge with the real pen and fingers the look and text (S17), the brushes, StreamLine, eraser, colour, eyedropper, panels, export, reference, the gestures and ColorDrop. Record `pass` / `fail` here. The pencil's opacity on the tablet may be a leftover test value: check it first.
2. Look (S17, D40): option A with floating islands, the 18-point system sans at regular weight, the ink ring for the active state. Recommended: keep until drawn with; size, weight or colour changes are one-line requests.
3. Pen hover (S17): outline at the brush's full size, labels on icons and side sliders. Recommended: keep; a centre dot or a pressure-scaled outline only if asked (draweave removed its dot).
4. Pencil and soft brush (S16, D38): draweave's tuned pencil as is; the Soft Brush builds up within a stroke. Recommended: keep until drawn with.
5. StreamLine (S16, D39): 0 % by default for every brush. Recommended: keep; a per-brush default on request.
6. Fill (S15, D37): Continue filling offered after each drop until Done or a tool tap; the threshold is session state; no fill reference layer. Recommended: keep until used.
7. View (S14, D35): not remembered per artwork, no rotation snapping, no four-finger interface toggle. Recommended: keep until used.
8. One-finger pan on open canvas (draweave's behaviour). Recommended: keep, since a finger never paints (D18).
9. Layer panel (S9, D23): reorder by buttons, opacity and Alpha Lock in the footer. Recommended: keep until used.
10. Gallery (S10): duplicate and delete as card buttons; Rename as a dialog; order by last change only. Recommended: keep until used.
11. Brushes (S11, D27): the panel stays open after a choice; square size curve; Ink Pen default. Recommended: keep until drawn with.
12. Touch-and-hold eyedropper (S12, D30): still only the side-bar button. The pen's eraser end: does nothing. Recommended: decide with the next gesture work.
13. Reference image (S13, D31): kept for the app's run only. Recommended: keep until used.
