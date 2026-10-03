# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 in progress: S9 (layers), S10 (artwork library), S11 (brush recipes) and S12 (eraser, Alpha Lock, colour) done; next slice: S13 in ROADMAP, then the author judges the G2 exit. G1 exit passed 2026-10-02 (the author judged pen feel and palm rejection on the tablet).

## Working features

- The canvas: a dark top bar with Gallery (a button) at its left end and, at its right end, right to left, the colour button (a round swatch of the current colour), Layers (a stack of sheets), Eraser (a slanted block) and Brush (a slanted brush); below it a white canvas. Over the canvas's left edge, centred on it, floats the side bar: the size slider, the eyedropper, the opacity slider, then undo and redo. A finger draws nothing, by decision (D18). One screen pixel is one canvas pixel. One panel (colour, layers, brushes) at a time.
- Tools (S12, D30): the pen paints with the Brush tool or takes paint away with the Eraser tool; the current tool's button is highlighted. Tapping the other tool's button switches tools; tapping the current one opens its brush panel (header "Brushes" or "Eraser"). The eraser uses the same three recipes with its own brush and its own size and opacity per brush; the side bar's sliders act on the current tool. Erasing is destination out on the active layer only.
- Colour (S12): the colour button opens a popover: header "Colour", a hue ring around a saturation (left to right) and value (bottom to top) square, then up to 8 recent colours, newest first ("Colours you choose appear here." while none). The colour changes while the finger moves and shows in the button at once; lifting it, tapping a recent colour or an eyedropper pick puts the colour first in the recents. Colours are 8-bit; black is the default.
- Eyedropper (S12): the side-bar button between the sliders turns it on (highlighted); the next contact on the canvas, pen or finger, picks the composited colour under it (read back from the canvas target) while it moves; lifting settles it and the tool used before returns. Tapping the button again turns it off without picking. Picking draws nothing.
- Alpha Lock (S12): the layer panel's footer has an Alpha Lock switch for the active layer (the row reads "Alpha Lock"); strokes begun while it is on paint only where the layer has paint and keep its alpha; turning it off later leaves them as they were. The eraser on an alpha-locked layer does nothing (Procreate's behaviour) and leaves no history step. Toggling is an undoable step.
- Brushes (S11, D27): Pencil, Ink Pen (the default, 12 px, 100 %), Soft Brush, each row with a stroke the evaluator painted; a tap chooses; the panel stays until a contact lands on the canvas outside it. Brush, tool and colour changes are no history steps; they survive restarts (stored apart from artworks, defaults when unreadable).
- Side bar (S11): vertical sliders, 0 at the bottom; the knob jumps to the finger and follows it; while held a bubble beside the bar shows "12 px" or "56 %". Size spans each brush's range on a square curve; opacity 1 % to 100 %. On a short screen the sliders shorten (never under two touch targets). Undo and redo (S6): one version per tap; grey while the pen draws. A contact starting on the bar never paints.
- Gallery (S10, D25): tap Gallery or press back on the canvas: the artwork and its thumbnail are stored, then the gallery replaces the canvas (if storing fails, the canvas stays with Retry). Bar: Gallery, "+". Cards, changed last first: picture, title (tap for Rename), copy, bin (asks before deleting).
- Reopen: the app reopens where it was left: on the canvas of the artwork that was open, or in the gallery.
- Layers (S9, D23): header "Layers" with a plus, rows top of the stack first (active highlighted, opacity and Alpha Lock noted, an eye), then for the active layer Opacity, Alpha Lock, move up, move down and delete (undo restores). Compositing bottom to top, Normal with layer opacity.
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts; a pen landing cancels touches already down.
- Autosave (S7): every finished stroke, layer change, undo and redo is saved in the background; leaving the app hands over what is left plus the thumbnail (waits up to 2 s). Failures show a notice with Retry or OK.
- core: `StrokeStyle { tool: StrokeTool { Paint, Erase }, recipe, size, opacity, color }`; `Stroke`/`ActiveStroke::blend() -> StrokeBlend { Over, Atop, Erase }` (Alpha Lock captured at begin); `LayerProps::alpha_locked`, `LayerCommand::SetAlphaLocked`, `CoreError::AlphaLocked`; `StrokeParts::blend`, `LayerParts::alpha_locked`.
- app: `Toolbox` with `Tool { Brush, Eraser }`, per-tool brush choice, `palette.rs` (`Palette`, `Hsv`, `RECENT_COLORS`), `picking`; `Intent::Tool(ToolIntent { Use, Eyedropper, Color })`, `Toolbox::dispatch_tool`; `EditorState { tool, paint, erase, color, recent }`, `BrushChoice`; `BrushProjection` adds tool, colour, recents, picking; `LayerIntent::SetAlphaLock`, `LayerRow::alpha_locked`.
- store: schema 4 (D29): Alpha Lock in head layer entries, a blend byte per stroke, the two-tool toolbox with colours; schemas 1, 2 and 3 upgrade in one batch (`migrate.rs`, pure steps 1 -> 2 -> 3 -> 4).
- render: paint ops carry a `Blend` (`PAINT_BLEND`; over, atop, erase generated into WGSL); `CanvasTarget::pick`; layout test in `protocol_tests.rs`.
- ui: `color_panel.rs`; components add `swatch`, `color_wheel`, `switch_row`; icons add Eraser, Eyedropper; `BrushPanel::buttons` (Eraser and Brush).
- runtime: `Canvas::dispatch(core, ..)` routes brush and tool intents to the toolbox and turns canvas contacts into picks while the eyedropper is on.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`. At 2880 x 1800 and 1.75: colour (2810, 42), Layers (2712, 42), Eraser (2614, 42), Brush (2516, 42), Retry (2397, 42); side bar x 70: size slider y 760 (0) to 494 (1), eyedropper 837, opacity slider 1180 (0) to 914 (1), undo 1271, redo 1369; brush rows Pencil (2450, 252), Ink Pen (2450, 364), Soft Brush (2450, 476); colour wheel centre (2586, 434), ring mid radius 210 px, square (2467..2705, 315..553), recent swatches y 719 from x 2353 every 66.5 px; layer panel with one layer: Alpha Lock (2600, 502); gallery positions as in `s10-gallery.json`.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, serde 1.0.229, serde_json 1.0.151, fjall 3.1.11, toml 1.1.6, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 28.3 s warm, 227 tests | 2026-10-02 | exit 0; `cargo repo-check` exit 0 with the new files staged (229 tracked files) |
| Erase and Alpha Lock GPU tests (`render/tests/stroke/erase.rs`) | pass | 2026-10-02 | erasing by opacity, on its own layer only, atop keeps alpha (edge pixel checked), incremental = cold for a grained soft eraser and an atop stroke, undo through `changed_regions`; golden `82e4c71c9e45a92a` identical on Vulkan and DX12 (RTX 5070) |
| Picking | pass | 2026-10-02 | `CanvasTarget::pick` equals the readback pixel at 6 points, `None` outside (render); through the frame loop an eyedropper tap gives exactly the readback pixel and paints it (runtime `tools.rs`) |
| Render goldens | pass, unchanged | 2026-10-02 | plain wave `ba8523102ad38e3a`, every-stage recipe `cfc027797b0b9f75`, brushes pencil `c7db069e23a22e48`, ink pen `a8c17ea8d659a519`, soft brush `9e52723c6f4900f9`; Vulkan and DX12 |
| `cargo scenario s12-colour-eraser` (debug) | pass, 10 of 10, ~1,530 frames, 2.8 s | 2026-10-02 | same hashes on Vulkan and DX12. Screenshots reviewed (`artifacts/scenarios/s12-colour-eraser/`): colour panel, wheel and recents, eraser panel, erased lines, Alpha Lock switch and row, blue atop red only, eyedropper highlighted, picked red, reopened colours |
| Older scenarios (debug) | pass, hashes unchanged | 2026-10-02 | taps moved: Layers 2810 -> 2712, Brush 2712 -> 2516, Retry 2593 -> 2397, undo 1229 -> 1271, redo 1327 -> 1369, slider drags 42 px, layer move/delete 586 -> 670 |
| Store tests: schema samples (4) | pass | 2026-10-02 | schema-1, -2 and -3 samples open upgraded to 4 with every record kept (heads marked 4 plus one Alpha Lock byte per layer, strokes given blend 0, the schema-3 toolbox kept with the eraser and black added); schema-4 sample byte-exact |
| Upgrade on the tablet: the author's schema-3 library opened by the S12 build | pass | 2026-10-02 | backup `artifacts/device/s12-library-before.tar`; log "library upgraded from schema 3 (1 projects, 22 strokes)"; canvas right of the side bar pixel-identical (`s12-before-upgrade.png`, `s12-upgraded.png`, ImageMagick AE 0) |
| S12 walk-through on the tablet (injected input) | pass | 2026-10-02 | MovinkPad 14, a new test artwork: wheel taps and drags (`s12-ring-tap.png`), blue and red lines, eraser with Soft Brush (`s12-erased.png`), undo (whole again) and redo (AE 0), Alpha Lock (`s12-alpha-locked.png`), red atop blue only, nothing on paper (`s12-red-atop.png`), eraser on the locked layer refused, canvas AE 0, eyedropper picked (0, 0, 255) and painted it exactly, `am force-stop` + start: canvas AE 0, colour, recents, tool and the eraser's brush restored. No panic or error in logcat. Found and fixed on the tablet: a tap pressing and lifting within one frame missed the hue ring |
| State left on the tablet | as found | 2026-10-02 | library restored to its state right after the upgrade (`s12-library-upgraded.tar`; the test artwork gone, toolbox: Brush, Ink Pen at the author's size, black, no recents); the author's drawing open, AE 0 against before the upgrade; S12 build installed |
| `replay`, release build | not_run this slice | — | golden `1f94cbfbd4b11e02` last confirmed in S9 |
| Brush and eraser feel with the real pen; colour wheel, eyedropper and panels by hand | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Brushes: three built-ins, no brush editing or import. Coverage is a union (D16), so a stroke never builds up over itself. Speed needs millisecond timestamps; injected `adb` strokes have constant pressure.
- Colour: one HSV wheel and recents only (no sliders, palettes, harmony); on black or grey a tap on the hue ring alone changes no visible colour. The eyedropper has no loupe; a pick reads one texel back on the main thread (waits for the GPU). The pen's eraser end and buttons do nothing.
- Alpha Lock is captured per stroke: turning it on does not change strokes drawn before; a layer property change re-evaluates the whole canvas.
- The side bar covers about 126 px of the canvas's left edge on the tablet; contacts starting there never paint. It is not mirrored for the left hand. On windows under about 450 points it runs off the bottom; under about 700 points wide the save notice no longer fits beside the four top-bar buttons.
- Gallery: duplicate and delete are buttons on each card, not Procreate's swipe; order is by last change only; no selection of several, no folders, no canvas size menu. A thumbnail is written when an artwork is left.
- Text: egui's fonts have no CJK glyphs. Closing the keyboard with its own button leaves the field focused.
- Layer panel: reorder by move up / move down only; no rename, no thumbnails, no blend modes.
- Palm rejection is basic: a hovering pen does not count; no contact size or shape. With injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate).
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22).
- Blending is in sRGB-encoded values. A save rewrites the head and the catalogue entry every time. Upgrading a library reads every stroke record into memory once.
- `leave` waits at most 2 s on the main thread; a stroke still being drawn when the app is left is not saved.
- GPU tests fail (not skip) on a machine without any adapter. Golden hashes are verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- No view transform: a stored canvas keeps its size, so after rotation part of it is off screen.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures.
- The screen painter is created with the first target's format. On API 28 and 29 the navigation bar stays visible. Gradle and Kotlin daemons hold `cargo android-build`'s output handles: redirect it to a file. No emulator; Android behaviour is checked over adb on the tablet.
- Scenarios: button positions are window pixels that move when a layout changes; a tap always spans two frames (a tap within one frame is only exercised on the tablet); screenshots are never compared; a slider can only be dragged, not held.

## Pending author decisions

- Gallery details that deviate from Procreate: duplicate and delete as buttons on every card instead of swipe-left; Rename as a dialog under the top bar instead of editing the title in place; the gallery ordered by last change instead of by hand. Recommended: keep until the author has used it on the tablet.
- Reorder by buttons instead of dragging rows (D23), and the opacity slider and Alpha Lock switch in the panel footer instead of behind a per-layer menu: both deviate in detail from Procreate's layer panel; drag-to-reorder is a ROADMAP candidate.
- Brush details (D27): the brush panel stays open after a choice, Ink Pen is the default, the size slider follows a square curve over each brush's range. Recommended: keep until the author has drawn with them.
- Eyedropper mapping (D30): only the side-bar button is wired. Options: (a) keep the button only; (b) add Procreate's touch-and-hold on the canvas with a finger (needs a hold gesture, and conflicts with nothing while fingers do not draw, D18); (c) map the pen's side button or eraser end. Recommended: (b) after S13, as one gesture slice with the two-finger undo.
- The pen's eraser end: today it does nothing. Recommended: make it erase with the eraser's brush (Procreate on iPad has no eraser end, so this is an Android-hardware choice for the author).
