# Status

Updated 2026-10-03. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 slices done: S9 to S13 and S14 (canvas view and gestures). Before the G2 exit (W1) come S15 to S17 (fill, stabilisation and pencil, visual design; ROADMAP, D32 to D35). Next slice: S15. G1 exit passed 2026-10-02.

## Working features

- The canvas: a dark top bar with Gallery and Actions (a spanner) at its left end and, at its right end, right to left, the colour button, Layers, Eraser and Brush; below it the canvas area, where the artwork shows through the view on a dark surround. Over the area's left edge, centred on it, floats the side bar: the size slider, the eyedropper, the opacity slider, then undo and redo. A finger draws nothing (D18). One panel (actions, colour, layers, brushes) at a time.
- View (S14, D35): every artwork opens at fit (the whole canvas, centred, upright, 92 % of the canvas area); zoom is relative to fit (0.25x to 8x), so rotating the tablet refits at the same zoom. Two fingers pan, pinch-zoom and rotate about the point between them, each past its own dead zone (pan 8 pt, zoom |ln| 0.04, rotation 0.14 rad to lead; 18 pt, 0.10, 0.24 rad to join); one finger on open canvas pans after 8 pt; a quick inward pinch (two fingers, within 0.4 s, from at least 48 pt to at most 0.45 of that) fits. No rotation snapping. A third finger, a cancel or a finger that started on a control blocks navigation until all fingers lift. The view is not stored and is no history step. While and 1 s after the view moves, a quiet line under the top bar reads the actual size and turn ("283 % · 37°"), then fades; the pen landing hides it at once.
- Finger taps on open canvas (S14): two undo, three redo, decided by the most fingers down when the last lifts; a finger moving over 8 pt voids it; held still they repeat (after 0.45 s, then every 0.08 s, nothing more on the lift). A control taking a contact resets every finger tracker.
- Pen through the view (S14): a pen contact that touches down on the canvas itself (not the surround, not a control) draws; its samples map through the view's inverse, so ink lands under the tip at any zoom and rotation. The eyedropper (pen or finger) picks through the same map.
- Undo, redo and every control a stroke blocks keep their look while the pen draws (`Availability::is_offered`); a tap then is refused at the intent. They grey only when there is nothing to undo or redo (or no layer to act on).
- Canvas size (S14): the gallery's "+" opens New canvas: Screen size (the canvas area, 2880 × 1716 on the tablet), Square 2048 × 2048, A4 portrait 2480 × 3508, A4 landscape 3508 × 2480; a tap outside or back closes it.
- Actions (S13, D31): Save PNG… / Save JPEG… / Share PNG… / Share JPEG… under Share; the Reference switch and Choose reference image… under Canvas. Export renders the whole canvas at full size on a worker; PNG equals the canvas; cancel says nothing; a failed write says so with OK.
- Reference image (S13): a floating window (move, zoom 1x to 8x, pan, close); picked from exactly by the eyedropper; kept for the app's run, never stored.
- Tools and colour (S12, D30): Brush and Eraser with their own brushes, sizes and opacities; a colour wheel with 8 recent colours; the side bar's eyedropper; Alpha Lock in the layer panel. Brushes (S11, D27): Pencil, Ink Pen (default), Soft Brush. Layers (S9, D23). Gallery (S10, D25). Palm rejection (S8). Autosave (S7). Reopen where the app was left.
- app (S14): `view.rs` (`CanvasView`, `Viewport`, `ViewGesture`, `ViewMapping` with `to_canvas`, `to_screen`, `to_canvas_rows`; `ViewIntent { Gesture, Fit }`; `FIT_MARGIN`, `MIN_ZOOM`, `MAX_ZOOM`), `gestures/` (`Gestures` over `pair.rs`, `filter.rs`, `pan.rs`, `taps.rs`; `Touch`, `GestureFrame`, `Gestured`, `TouchAction`), `CanvasPreset`, `GalleryIntent::Create(CanvasPreset)`; `Session::view`, `SessionProjection::{view, canvas, drawing}`.
- ui (S14): `canvas_input.rs` (`CanvasInput` routes the pen through the view and fingers to `Gestures`), `components::view_status`, the gallery's New canvas popover; `Shell::set_now`, `Shell::animating`.
- render (S14): the blit samples the canvas target bilinearly through the view's affine map, clipped to the canvas area (`CanvasPlacement { origin, size, to_canvas }`); the target stays at canvas resolution.
- runtime (S14): `clock.rs` (`InputClock`: sample time carried on by the wall clock); frames keep coming while the status fades or fingers hold.
- android (S14): debug builds carry `GestureInstrumentation` (`src/debug/`), which injects touch and pen gestures through UiAutomation: `am instrument -w --no-restart -e from x,y[,x,y…] [-e to …] [-e ms N] [-e tool stylus] dev.openpaint/.GestureInstrumentation`.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference`, `s14-view-gestures`. Steps added in S14: `touch` (1 to 5 fingers from `from` to `to` over `ms`, Android's down/up order, the last move with the first lift) and `view` (zoom within 1 %, rotation within 0.5°, pan within 1 pt). At 2880 x 1800 and 1.75: canvas area origin (0, 84), centre (1440, 942); New canvas rows Screen size (2500, 240), Square (2500, 322); other positions as in STATUS of S13 and the scenario files.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check. Pins (D14) unchanged.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 56.3 s, 280 tests | 2026-10-03 | exit 0; repo-check 265 tracked files |
| `cargo scenario s14-view-gestures` (debug) | pass, 12 of 12 checkpoints and views | 2026-10-03 | screenshots reviewed (`artifacts/scenarios/s14-view-gestures/`): fit, zoomed (status line), detail under the pen at 283 % · 37°, after panning, fitted by quick pinch, New canvas sizes, sizes closed, square opened, reopened at fit |
| Older scenarios (debug) | pass, hashes re-recorded | 2026-10-03 | every canvas hash moved because strokes now land through the fit view (0.92), not 1:1; the equalities between checkpoints (undo = earlier version and so on) are unchanged; PNGs looked at; s10 taps Screen size after "+" |
| Screen-to-canvas mapping | pass | 2026-10-03 | app `view.rs` round trips at three views; render `stroke/screen.rs`: at 1:1 on whole pixels the screen equals the canvas bytes, zoomed and turned each point shows where the map puts it; runtime `drawing.rs`: ink under the pen at fit and after a two-finger spread and twist, finger pan, two/three-finger taps, quick pinch to fit, the side bar unchanged between pen-down and pen-up |
| S14 on the tablet (injected through `GestureInstrumentation`) | pass | 2026-10-03 | MovinkPad 14, a square test artwork: New canvas popover (`s14-new-canvas-sizes.png`), pen at fit (`s14-pen-at-fit.png`), two-finger spread and twist to 331 % · 13° (`s14-pinched.png`), pen at that view under the tip (`s14-pen-at-zoom.png`, ink at the three sampled points), two-finger tap undid, three-finger tap redid (`s14-two-finger-undo.png`, `s14-three-finger-redo.png`), quick pinch fitted (`s14-quick-pinch-fit.png`), portrait refit (`s14-portrait.png`); no panic or error in logcat |
| The author's artwork in the S14 build | pass | 2026-10-03 | backup `artifacts/device/s14-library-before.tar`; no format change (schema 4); it opens at fit (`s14-after-install.png`); Save PNG of it equals the S13 build's 1:1 screen right of the side bar (ImageMagick AE 0, `s14-exported.png` against `s14-before-install.png`) |
| State left on the tablet | as found | 2026-10-03 | the test artwork deleted in the gallery; the exported test file deleted from Download; the author's artwork open on the canvas; rotation free; S14 build installed |
| `replay`, release build | fail (stale since S11) | 2026-10-03 | its undo/redo taps still aim at the top bar; ROADMAP candidate |
| Gesture feel with real fingers; pen, brushes, colour, panels, export and reference by hand | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- View: not stored per artwork (every open is at fit); no rotation snapping, no four-finger tap to hide the interface, no flip. The screen samples the canvas bilinearly without mipmaps, so far zoomed out thin lines shimmer. The view status line can cover the top of the canvas for about a second after a gesture.
- A held two- or three-finger shortcut repeats by the input clock; while fingers rest, the app asks for frames continuously.
- Injected gestures: the debug instrumentation injects at constant pressure; a self-targeting instrumentation needs `--no-restart` or Android restarts the app first.
- Export: the flattened canvas only, on white. Reference: one picture for the app's run, no pinch zoom inside its window.
- Brushes: three built-ins; coverage is a union (D16). Colour: HSV wheel and recents only. Alpha Lock captured per stroke.
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

1. G2 exit (W1), after S17: finish an inked and flat-coloured piece on the tablet and judge with the real pen and fingers the brushes, eraser, colour, eyedropper, panels, export, reference and now the gestures (dead zones, quick pinch, taps). Record `pass` / `fail` here.
2. View (S14, D35): not remembered per artwork, no rotation snapping, no four-finger interface toggle (D32: no control it would duplicate). Recommended: keep until used; each is a ROADMAP candidate.
3. One-finger pan on open canvas (draweave's behaviour; Procreate paints or smudges with a finger). Recommended: keep, since a finger never paints (D18).
4. Layer panel (S9, D23): reorder by buttons, opacity and Alpha Lock in the footer. Recommended: keep until used.
5. Gallery (S10): duplicate and delete as card buttons; Rename as a dialog; order by last change only. Recommended: keep until used.
6. Brushes (S11, D27): the panel stays open after a choice; square size curve; Ink Pen default. Recommended: keep until drawn with.
7. Touch-and-hold eyedropper (S12, D30): still only the side-bar button. Recommended: add it with the next gesture work if wanted; it fits the tap tracker (hold without movement).
8. The pen's eraser end (S12): does nothing. Recommended: erase with the eraser's brush.
9. Reference image (S13, D31): kept for the app's run only. Recommended: keep until used.
10. Actions entry (S13, D31): four rows for Save and Share. Recommended: keep until used.
