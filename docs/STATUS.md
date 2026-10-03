# Status

Updated 2026-10-03. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 slices done: S9 to S16 (S16: stabilisation and pencil). Before the G2 exit (W1) comes S17 (visual design; ROADMAP, D32 to D35). Next slice: S17. G1 exit passed 2026-10-02.

## Working features

- The canvas: a dark top bar with Gallery and Actions (a spanner) at its left end and, at its right end, right to left, the colour button, Layers, Eraser and Brush; below it the canvas area, where the artwork shows through the view on a dark surround. Over the area's left edge, centred on it, floats the side bar: the size slider, the eyedropper, the opacity slider, then undo and redo. A finger draws nothing (D18). One panel (actions, colour, layers, brushes) at a time.
- Brushes (S16, D38): the Pencil is draweave's tuned graphite: width, darkness and the share of paper touched follow pressure; the sketch paper's tooth is fixed to the canvas; a stroke darkens where it crosses itself, up to its opacity; more pen events over the same path never darken it. The Soft Brush builds up the same way under a bell tip (so does the eraser on it). The Ink Pen covers: crossing itself it stays one even coverage. Strokes drawn before S16 keep their recipes and look exactly as before.
- StreamLine (S16, D39): under the brush panel's rows "StreamLine" with its percentage and a slider (0 % at first), per brush and tool, kept across launches. The drawn line follows the pen on a leash (clamp(3 x size, 24, 96) canvas pixels at 100 %, times amount squared), visibly trailing while the pen is down; at the lift the end slides onto the lift point. Only the path is smoothed. It applies from the next stroke.
- Fill (S15, D36, D37): press the colour button and drag onto the canvas: after 8 pt a drop follows with "Lift to fill. Hold still to adjust."; lifting over the canvas fills the active layer's connected region around that point (four neighbours, the seed's colour, alpha included, threshold 60 % at first, plus one edge ring; no gap closing). Held still 0.4 s, sliding sideways changes the threshold ("Threshold 60.0 % · slide to adjust"). Lifting over a control or off the canvas cancels. After each drop a bar under the top bar offers "Continue filling" (pen taps fill until Done or a tool tap); while shown it covers the canvas's top middle. A fill is one undo step and artwork content; a hidden or locked layer is refused with a notice; Alpha Lock colours only the layer's paint. The threshold is kept for the session, not stored.
- View (S14, D35): every artwork opens at fit (92 % of the area); zoom relative to fit (0.25x to 8x). Two fingers pan, pinch-zoom and rotate past their dead zones; one finger on open canvas pans after 8 pt; a quick inward pinch fits; a quiet line reads size and turn. Finger taps on open canvas: two undo, three redo (held, they repeat).
- Undo, redo and every control a stroke blocks keep their look while the pen draws (`Availability::is_offered`); they grey only when there is nothing to act on. Canvas sizes (S14): Screen size, Square 2048, A4 portrait, A4 landscape.
- Actions (S13, D31): Save/Share PNG/JPEG, the Reference switch and picker; the reference is a floating window kept for the app's run. Tools and colour (S12, D30), brushes (S11, D27), layers (S9, D23), gallery (S10, D25), palm rejection (S8), autosave (S7), reopen where the app was left.
- core (S16): `deposit.rs` (`Deposit`, `Accumulation`, `Tip`, `Paper`, `PaperTexture`), `BrushRecipe::with_deposit`, `RecipeParts::deposit`, `StrokeSession::revise`. app (S16): `streamline.rs`, `Session::set_streamline`, `BrushIntent::StreamLine`, `[accumulate]` and `[paper]` in brush files. render (S16): 12-operand ops, `deposit.wgsl`, `paper.rs` with `assets/paper-sketch.bin`, two kept textures per target, `protocol_wgsl.rs`. store (S16): schema 6, `migrate_6.rs`. ui (S16): the StreamLine slider in `brush_panel.rs`.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference`, `s14-view-gestures`, `s15-fill`, `s16-stabilisation-pencil`. At 2880 x 1800 and 1.75: Brush button (2516, 42), brush rows Pencil (2450, 252), Ink Pen (2450, 364), Soft Brush (2450, 476), the StreamLine slider's right end (2850, 670); colour button (2810, 42); side bar opacity drag (70, 942) to (70, 1074) for about 40 %, undo (70, 1271), redo (70, 1369).
- Device input (debug builds): `am instrument -w --no-restart -e tool stylus -e pressure P -e path x,y,... dev.openpaint/.GestureInstrumentation` draws one pen path (8 ms per point); `-e from/-e to` for fingers as before.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check. Pins (D14) unchanged.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 36.2 s warm, 333 tests | 2026-10-03 | exit 0 |
| `cargo scenario s16-stabilisation-pencil` (debug) | pass, 6 of 6 checkpoints | 2026-10-03 | screenshots reviewed (`artifacts/scenarios/s16-stabilisation-pencil/`): brush-panel (StreamLine 0 %), pencil-loops-screen (light loop darker at its crossing, heavy near black), streamline-full (100 %), smooth-line-trails (the line 36 px behind the pen), smooth-line-screen (end on the lift point), reopened-ink-streamline (100 % kept), pencil-streamline-own (0 %) |
| Scenarios `s11-brushes`, `s12-colour-eraser` | pass, hashes re-recorded | 2026-10-03 | pixels changed on purpose: the new pencil and the accumulating soft brush and soft eraser; PNGs reviewed. Every other scenario passes unchanged |
| Deposit and StreamLine tests | pass | 2026-10-03 | crossings darken up to the opacity, ink stays even, 5 or 241 samples over one line lay the same density (within 1 level), no dots at joints, more pressure takes more paper; incremental = cold for graphite and airbrush; StreamLine: wobble at 100 % under 0.3 of the raw, lag = leash at any sample density, the finished stroke ends at the lift point, pressure untouched |
| Brush goldens | pass, Vulkan = DX12 | 2026-10-03 | pencil `0b0932295b553b12` and soft brush `91acbe03f571da3e` re-recorded (new recipes); the S11 pencil and soft brush recipes still draw their old hashes (`c7db069e23a22e48`, `9e52723c6f4900f9`); ink pen and every render golden unchanged |
| Schema 6 | pass | 2026-10-03 | `schema-6.bin` byte-exact (1500 bytes: a graphite stroke on paper, an airbrush dot, StreamLine in the toolbox); `schema-1..5.bin` open upgraded with every record kept, old strokes covering |
| `replay`, release build | pass | 2026-10-03 | golden `c3320cf50b5727af` unchanged; frames max 3.5 ms (budget 8) |
| S16 on the tablet (injected pen paths) | pass | 2026-10-03 | MovinkPad 14, a test artwork: pencil at pressure 0.2 and 0.9 crossing itself (`s16-pencil-light-heavy.png`; at the author's pencil opacity the heavy loop sits at the cap), Ink Pen at low opacity crossing itself stays even (`s16-ink-low-opacity-loop.png`), a zigzag at StreamLine 0 % and 100 % (`s16-streamline-lines.png`, `s16-streamline-trailing.png`, `s16-streamline-lifted.png`); force-stop and reopen: canvas identical (only undo greyed); no panic or error in logcat |
| The author's artwork in the S16 build | pass | 2026-10-03 | backup `artifacts/device/s16-library-before.tar`; "upgraded from schema 5 (1 projects, 38 strokes)"; the screen at fit is byte-identical to the S15 build's (`s16-after-install.png` = `s16-before-install.png`): the old pencil and soft strokes unchanged |
| State left on the tablet | as found | 2026-10-03 | test artwork gone: the library restored from the backup (byte-identical tar) and upgraded again by the final build (`s16-final.png` = `s16-before-install.png`); rotation untouched; S16 debug build installed |
| Pencil, StreamLine, gestures and fill feel with the real pen and fingers | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | not re-measured in S16 | 2026-10-03 | S15 build: event to present call p50 9.1 to 9.5 ms |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Pencil: the paper repeats every ~228 x 114 canvas pixels; no tilt response (as draweave); the grain is evaluated at canvas resolution, so zoomed in it enlarges and far zoomed out the screen's bilinear sampling may shimmer; at a low opacity a heavy pencil reaches the cap and crossings no longer darken.
- StreamLine: the stored stroke is the smoothed path (the raw pen path is not kept); no corner smoothing or curves between samples; pressure is never smoothed.
- Fill: no fill reference layer, no gap closing, no fill inside a selection; the threshold resets with each artwork session; a pencil's grain stops a low-threshold fill at its specks; a full-canvas flood takes about 145 ms on the tablet; every scene rebuild re-uploads all fill masks; the fill bar covers the canvas's top middle while it shows.
- View: not stored per artwork; no rotation snapping, no four-finger tap, no flip; bilinear without mipmaps, so far zoomed out thin lines shimmer.
- A held two- or three-finger shortcut or a held drop asks for frames continuously.
- Injected gestures: `input` has constant pressure (the instrumentation's `-e pressure` does not); a self-targeting instrumentation needs `--no-restart`; `input motionevent` at the top edge can pull the system bar.
- Export: the flattened canvas only, on white. Reference: one picture for the app's run.
- Brushes: three built-ins; no brush editing. Colour: HSV wheel and recents only. Alpha Lock captured per stroke and per fill.
- The side bar covers about 126 px of the area's left edge and is not mirrored; on windows under about 450 points it runs off the bottom.
- Gallery: duplicate and delete are card buttons, not swipe; order by last change only. Text: no CJK glyphs. Layer panel: reorder by buttons only.
- Palm rejection is basic; with injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped.
- Presentation dominates latency (D22). Blending is in sRGB-encoded values. A save rewrites the head every time.
- `leave` waits at most 2 s; a stroke still being drawn when the app is left is not saved.
- GPU tests fail (not skip) without an adapter. Golden hashes verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls. Each canvas target holds two extra float textures (about 40 MB at 2880 x 1716) and the core 2.8 MB of paper.
- egui gets pointer events from the first contact of each batch only; multi-finger gestures are read from the batches directly.
- Scenarios: button positions are window pixels; screenshots are never compared; the system's dialogs are played by devtools.

## Pending author decisions

1. G2 exit (W1), after S17: finish an inked and flat-coloured piece on the tablet and judge with the real pen and fingers the brushes (now the S16 pencil and the building-up soft brush), StreamLine, eraser, colour, eyedropper, panels, export, reference, the gestures and ColorDrop (hold time, slide speed, the 60 % default). Record `pass` / `fail` here.
2. Pencil and soft brush (S16, D38): draweave's tuned pencil as is; the Soft Brush (and the soft eraser) now builds up within a stroke, one full-pressure pass about 72 % of the opacity. Recommended: keep until drawn with; retune by one-line requests.
3. StreamLine (S16, D39): one slider under the brush rows, 0 % by default for every brush, 100 % = draweave's full leash; the stroke stores the smoothed path. Recommended: keep; a per-brush default (Procreate gives inking brushes some StreamLine) on request.
4. Fill (S15, D37): Continue filling is a bar offered after each drop, covering the canvas's top middle while shown, and ends on Done or a tool tap; the threshold is session state; no fill reference layer. Recommended: keep until used; the reference layer and a stored threshold are ROADMAP candidates.
5. View (S14, D35): not remembered per artwork, no rotation snapping, no four-finger interface toggle. Recommended: keep until used.
6. One-finger pan on open canvas (draweave's behaviour; Procreate paints or smudges with a finger). Recommended: keep, since a finger never paints (D18).
7. Layer panel (S9, D23): reorder by buttons, opacity and Alpha Lock in the footer. Recommended: keep until used.
8. Gallery (S10): duplicate and delete as card buttons; Rename as a dialog; order by last change only. Recommended: keep until used.
9. Brushes (S11, D27): the panel stays open after a choice; square size curve; Ink Pen default. Recommended: keep until drawn with.
10. Touch-and-hold eyedropper (S12, D30): still only the side-bar button. Recommended: add it with the next gesture work if wanted.
11. The pen's eraser end (S12): does nothing. Recommended: erase with the eraser's brush.
12. Reference image (S13, D31): kept for the app's run only. Actions entry (S13): four rows for Save and Share. Recommended: keep until used.
