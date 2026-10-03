# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 in progress: S9 (layers), S10 (artwork library) and S11 (brush recipes) done; next slice: S12 in ROADMAP. G1 exit passed 2026-10-02 (the author judged pen feel and palm rejection on the tablet).

## Working features

- The canvas: a dark top bar with Gallery (a button) at its left end and, at its right end, right to left, Layers (a stack of sheets) and Brush (a slanted brush); below it a white canvas. Over the canvas's left edge, centred on it, floats the side bar: the size slider, the opacity slider, then undo and redo. A finger draws nothing, by decision (D18). One screen pixel is one canvas pixel.
- Brushes (S11, D27): the Brush button opens the brush panel under the top bar's right end (like Layers; opening one closes the other): header "Brushes", rows Pencil, Ink Pen, Soft Brush, each with its name and a stroke it painted through the evaluator, the current one highlighted. A tap chooses; a contact on the canvas outside closes the panel and a pen stroke there draws. Ink Pen (the default, 12 px, 100 %) is the round brush every stroke used before S11; Pencil is thin, grained and lighter with soft pressure or fast drawing; Soft Brush is wide with a fading edge, its strength following pressure. Each brush keeps its own size and opacity; the brush, size and opacity survive restarts (stored apart from artworks, defaults when unreadable). Brush changes are no history steps.
- Side bar (S11): vertical sliders, 0 at the bottom; the knob jumps to the finger and follows it; while held a bubble beside the bar shows "12 px" or "56 %". Size spans each brush's range on a square curve; opacity 1 % to 100 %. On a screen too short for it the sliders shorten (never under two touch targets) and the bar starts under the top bar. Undo and redo (S6): one version per tap, the canvas updates in the same frame; grey while the pen draws. A contact starting on the bar never paints.
- Gallery (S10, D25): tap Gallery or press back on the canvas: the artwork and its thumbnail are stored, then the gallery replaces the canvas (if storing fails, the canvas stays with Retry). Bar: Gallery, "+" (a new blank artwork the size of the canvas area). Cards, changed last first: picture (tap to open), title (tap for Rename under the top bar with the soft keyboard), copy, bin (asks before deleting). Titles 1 to 80 characters; new and upgraded artworks are "Untitled Artwork".
- Reopen: the app reopens where it was left: on the canvas of the artwork that was open, or in the gallery. An empty library starts on a blank canvas, stored with its first stroke.
- Layers (S9, D23): the Layers button opens a panel under it at the right edge: header "Layers" with a plus, rows top of the stack first (active highlighted, "34 %" when not opaque, an eye), then for the active layer Opacity with a slider, move up, move down and delete (undo restores). Compositing bottom to top, Normal with layer opacity.
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts; a pen landing cancels touches already down; a system-cancelled touch never taps.
- Autosave (S7): every finished stroke, layer change, undo and redo is saved in the background; leaving the app hands over what is left plus the thumbnail (waits up to 2 s). Failures show "Couldn't save. Your drawing is still here." and Retry, or "Couldn't open the last drawing. This is a new one." and OK.
- core: `BrushRecipe` (softness, size and opacity `Dynamics` = pressure `Curve` x speed `Curve`, `Grain`), `plain()`, `RecipeParts`; `StrokeStyle { recipe: Arc<BrushRecipe>, size, opacity, color }`; `from_storage` shares equal recipes.
- app: `Session::new/open/after_failed_open(.., brush: StrokeStyle)`, `set_brush`; `BrushLibrary` (built-in TOML files in `crates/app/brushes/`, `recipe.rs` parser with typed `RecipeError`), `Toolbox` (editor state: `dispatch(BrushIntent)`, `style`, `projection`, `state`/`restore`, `save_request`), `EditorState`; `Intent::Brush`; `SessionProjection::brush`/`with_brush`; `ProjectStore` adds `editor`, `save_editor`.
- store: schema 3 (D28): strokes carry their recipe after the colour; `library/editor` holds the toolbox; schemas 1 and 2 upgrade in one batch on open (`migrate.rs`, pure steps 1 -> 2 -> 3).
- render: per-segment radius and flow from the recipe and pen speed (`stroke.rs`), soft edge and canvas-fixed integer grain in the shader; protocol `Op` has 9 operands, named slots and `GRAIN_ONE` generated into WGSL; `stroke_region(style, samples, from)`.
- ui: `side_bar.rs`, `brush_panel.rs`; components add `side_panel`, `side_slider`, `brush_row`; icons add Brush; `CanvasInput::route` takes several covers; `Shell::set_brush_previews`.
- runtime: owns the `Toolbox` across artworks; `preview.rs` paints the picker's previews at start; `ui_pass.rs`; storage jobs `SaveEditor`, `Started(opened, editor)`.
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`. At 2880 x 1800 and 1.75: undo (70, 1229), redo (70, 1327), size slider x 70 from y 802 (0) to 536 (1), opacity slider y 1138 (0) to 872 (1), Brush button (2712, 42), brush rows Pencil (2450, 252), Ink Pen (2450, 364), Soft Brush (2450, 476), Retry (2593, 42); gallery positions as in `s10-gallery.json`.
- Build: `cargo android-build` / `cargo android-install` (cargo-ndk arm64-v8a API 28, Gradle 8.13, AGP 8.13.2, Kotlin 2.2.21, SDK 35). Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, serde 1.0.229, serde_json 1.0.151, fjall 3.1.11, toml 1.1.6, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 29.2 s, 209 tests | 2026-10-02 | exit 0; `cargo repo-check` exit 0 with the new files staged (218 tracked files) |
| Brush golden hashes (runtime `brushes` test, wave at 40 px) | pass | 2026-10-02 | pencil `c7db069e23a22e48`, ink pen `a8c17ea8d659a519`, soft brush `9e52723c6f4900f9`; incremental (3 samples per frame) equals cold for each; identical on Vulkan and DX12 (RTX 5070) |
| Render goldens | pass | 2026-10-02 | plain wave `ba8523102ad38e3a` unchanged (the recipe path reproduces the old brush bit for bit); every-stage recipe `cfc027797b0b9f75`; both on Vulkan and DX12 |
| `cargo scenario s11-brushes` (debug) | pass, 7 of 7, 1,471 frames, 2.5 s | 2026-10-02 | Vulkan and DX12 give the same hashes. Screenshots reviewed (`artifacts/scenarios/s11-brushes/`): panel with three previews, chosen row highlighted, side bar sliders follow the brush, pencil grain, soft edge, toolbox back after a restart |
| Older scenarios (debug) | pass, hashes unchanged | 2026-10-02 | undo/redo taps moved to the side bar, Retry to (2593, 42); `smoke` stroke moved right of the side bar: new hash `bc6e5fe383cf6a9c` |
| Store tests: schema samples (3), toolbox | pass | 2026-10-02 | schema-1 and schema-2 samples open upgraded to 3 with every record kept (heads marked 3, strokes given the plain recipe); schema-3 sample (two recipes, toolbox) byte-exact; an unreadable toolbox is `Corrupt` and the artwork still opens |
| App tests: brushes | pass | 2026-10-02 | built-ins parse; typed errors (syntax line and column, missing, wrong type, unknown key, out of range); per-brush settings; a changed built-in brush leaves strokes drawn before with their recipe |
| Upgrade on the tablet: the author's schema-2 library opened by the S11 build | pass | 2026-10-02 | backup `artifacts/device/s11-library-before.tar`; log "library upgraded from schema 2 (1 projects, 3 strokes)"; canvas right of the side bar pixel-identical before and after (`s11-before-upgrade.png`, `s11-upgraded.png`, ImageMagick AE 0) |
| S11 walk-through on the tablet (injected input) | pass | 2026-10-02 | MovinkPad 14: new test artwork, ink line, brush panel, pencil, size and opacity bubbles while held (`s11-size-bubble.png`, `s11-opacity-bubble.png`), soft brush, undo (canvas equals the one before, AE 0) and redo (AE 0), `am force-stop` + start: canvas AE 0, soft brush and per-brush sizes restored; test artwork deleted; the author's drawing reopened AE 0. No panic or error in logcat. Installed: S11 with the author's drawing open, Ink Pen current (the pencil keeps the test's size and opacity) (`s11-library-after.tar`) |
| `replay`, release build | not_run this slice | — | golden `1f94cbfbd4b11e02` last confirmed in S9; the plain brush's pixels did not change |
| Brush feel with the real pen; the side bar and panels by hand | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Brushes: three built-ins, no brush editing, import or per-brush settings beyond size and opacity; ink is black until S12. Coverage is a union (D16), so a stroke never builds up over itself (no airbrush flow accumulation). Speed comes from 40 ms of path and needs millisecond timestamps; injected `adb` strokes have constant pressure. The scene recomputes per-sample speed for strokes whose recipe uses it each time the version changes.
- The side bar covers about 126 px of the canvas's left edge on the tablet; contacts starting there never paint. It is not mirrored for the left hand yet. On short windows the bar may run off the bottom.
- Gallery: duplicate and delete are buttons on each card, not Procreate's swipe; order is by last change only; no selection of several, no folders, no canvas size menu. A thumbnail is written when an artwork is left, so after a crash it can show an older state.
- Text: egui's fonts have no CJK glyphs (a Chinese title shows boxes). Closing the keyboard with its own button leaves the field focused; tapping the field does not bring the keyboard back.
- Layer panel: reorder by move up / move down only; no rename, no thumbnails, no blend modes; a layer property change re-evaluates the whole canvas.
- Palm rejection is basic: a hovering pen does not count; no contact size or shape. With injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate).
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22).
- Blending is in sRGB-encoded values. A save rewrites the head and the catalogue entry every time; stroke records are compared by pointer from the start of each layer. Upgrading a library reads every stroke record into memory once.
- `leave` waits at most 2 s on the main thread; a stroke still being drawn when the app is left is not saved. Storage failure on the tablet was not provoked (only through the scenario's store switch).
- GPU tests fail (not skip) on a machine without any adapter. Golden hashes are verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- No view transform: a stored canvas keeps its size, so after rotation part of it is off screen.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures.
- The screen painter is created with the first target's format. On API 28 and 29 the navigation bar stays visible. Gradle and Kotlin daemons hold `cargo android-build`'s output handles: redirect it to a file. No emulator; Android behaviour is checked over adb on the tablet.
- Scenarios: button positions are window pixels that move when a layout changes; a palm is one touch at one point; screenshots are never compared; a checkpoint needs a canvas on screen; a slider can only be dragged, not held, so the bubble shows only on the tablet.

## Pending author decisions

- Gallery details that deviate from Procreate: duplicate and delete as buttons on every card instead of swipe-left; Rename as a dialog under the top bar instead of editing the title in place; the gallery ordered by last change instead of by hand. Recommended: keep until the author has used it on the tablet.
- Reorder by buttons instead of dragging rows (D23), and the opacity slider in the panel footer instead of behind a per-layer menu: both deviate in detail from Procreate's layer panel; drag-to-reorder is a ROADMAP candidate.
- Brush details (D27): the brush panel stays open after a choice (closes on the next canvas contact), Ink Pen is the default and the brush before S11 unchanged, the size slider follows a square curve over each brush's range. Recommended: keep until the author has drawn with them; feel and look of the three recipes are the author's call at the G2 exit.
