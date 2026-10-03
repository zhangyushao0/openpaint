# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 in progress: S9 (layers) done; next slice: S10 in ROADMAP. G1 exit passed 2026-10-02 (the author judged pen feel and palm rejection on the tablet).

## Working features

- The first screen: a dark top bar with "OpenPaint", undo and redo arrows, and at its right end the Layers button (a stack of sheets); below it a white canvas. Draw with the pen and a 12-pixel black round stroke follows the tip with pressure, ending in the brush's round tip where the ink stopped. A finger draws nothing, by decision (D18). One screen pixel is one canvas pixel.
- Layers (S9, D23): tap the Layers button and a panel drops down under it at the right edge, whole in the same frame, the button highlighted while it is open. Header "Layers" with a plus (adds an empty layer above the active one, which becomes active). Rows top of the stack first, the active one highlighted, a dimmer "34 %" when a layer is not opaque, an eye at the right end (open: shown; struck through: hidden). Tap a row to draw on that layer. Below the rows, for the active layer: "Opacity" with its percentage, a slider (jumps to the press, follows the finger, the canvas fades live), move up, move down and delete (no confirmation). Unavailable buttons are grey: move up on the top layer, down on the bottom one, delete with one layer, plus at 256 layers. A contact that starts on the panel never paints; any contact landing on the canvas outside closes the panel, and a pen stroke there also draws on the active layer. Tapping undo or redo keeps the panel open and it follows. Compositing: bottom to top, Normal (source over) with layer opacity; hidden layers add nothing.
- Undo and redo (S6): each tap switches one version back or forward, the canvas updates in the same frame. Every layer change is one step (add, delete, show/hide, move, and one whole slider touch); choosing the active layer is not a step of its own (undo goes back past it, redo stays). A grey arrow ignores taps; while the pen draws both are grey.
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts; a pen landing cancels touches already down; a system-cancelled touch never taps. A hovering pen does not count.
- Resume (S7): every finished stroke, layer change, undo and redo is saved in the background. Leaving the app saves what is left (waits up to 2 s). Reopening shows the last artwork pixel for pixel with its layers, order, visibility, opacity and active layer; history starts empty; the panel is closed. A failed save shows an alert, "Couldn't save. Your drawing is still here." and Retry, left of the Layers button; an unreadable library starts a blank artwork with "Couldn't open the last drawing. This is a new one." and OK.
- core: immutable `Artwork` (canvas, layers bottom to top, active layer, id counters); `apply(ArtworkCommand::Layer(..))` for create, delete, reorder, select, rename, visibility, opacity, lock; `StrokeSession`; `to_parts` / `from_storage`; typed `CoreError`; chunked `StrokeList`; `bytes_since`. No dependencies.
- app: `InputBatch`, `Contact`, `InputSample`; `PalmRejection`; `Session { new, open, after_failed_open, dispatch, artwork, active, projection, history, save_request, leave_request, save_finished, saving }`. Intents: `Stroke(Contact)`, `History(Undo | Redo)`, `Storage(RetrySave | Dismiss)`, `Layer(Add | Select | Delete | Raise | Lower | SetVisible | SetOpacity { settled })`; refusals are `AppError::Unavailable { action, availability }`. `History { record, amend, undo, redo }`, versions costed by `bytes_since`, 64 MiB budget (D20). `SessionProjection` (`Clone`): availability per `EditorAction` (undo, redo, retry, dismiss, add/delete/raise/lower/edit layer; `Availability::Full` at the layer limit), `storage()`, `layers() -> LayerList { rows, active }`. Ports: `Project`, `ProjectStore { open_last, create, save }`, `PortError`.
- store: `Library::at(dir)` implements `ProjectStore` on fjall, schema 1 (D21): head record with the layer table, one record per stroke; a save is one synced batch of the changed records. S9 changed nothing in the format (D24).
- render: `Core::headless()`, `create_surface`, `RegionEvaluator::new(&core, &mut target).evaluate(&FrameRequest, &[PixelRect])` as the single entry (D16, D17), `changed_regions`, `ScreenPainter`; FNV-1a 64 `content_hash`.
- ui: theme in `design` (adds `SELECTED`, `OUTLINE`, `ON_CHROME_SECONDARY`, `PANEL_WIDTH`); icons Undo, Redo, Alert, Layers, Add, Raise, Lower, Delete, Visible, Hidden; components `top_bar`, `icon_button`, `icon_toggle`, `text_button`, `notice`, `list_row`, `slider`, `popover`, `canvas_frame` with `CanvasInput` (routes around a cover); `Shell { show, canvas_area, route }` with the layer panel.
- runtime: `Runtime::start(density, Box<dyn ProjectStore>)` with the storage worker (`worker::Latest`, the only place a thread starts); `attach`, `resize`, `detach`, `leave`, `motion`, `frame`, `projection`, `storing`, and for the harness `frame_offscreen`, `read_offscreen`, `read_canvas`, `screenshot`, `wait_idle`, `artwork`, `adapter`. Every change other than a stroke commit passes `changed_regions` to the evaluator.
- android: Kotlin `MainActivity`, `Native.create(filesDir)`, `Native.leave` in `onPause`, `SurfaceView`, `Choreographer` frames on the main thread (D15, D22), `MotionEvent`s as arrays; `PenLatency` probe logging per stroke under tag `openpaint`.
- harness and scenarios: `render-smoke`, `render-stroke`, `shell-smoke`, `replay` (golden `1f94cbfbd4b11e02`); `cargo scenario <name>` with steps `tap`, `drag`, `stroke` (`lift`, `palm`), `wait`, `restart`, `storage_fails`, `storage_works`, `checkpoint`, `screenshot` (D19). Scenarios: `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume` (Retry now at (2691, 42)), `s8-device-loop`, `s9-layers` (2880 x 1800 at 1.75: Layers (2810, 42), plus (2810, 154), first row's eye (2810, 238), second row (2500, 322), slider y 502 from x 2341 to 2831, move up (2370, 586), delete (2810, 586), with two layers).
- Build: `cargo android-build` / `cargo android-install` (cargo-ndk arm64-v8a API 28, Gradle 8.13, AGP 8.13.2, Kotlin 2.2.21, SDK 35).
- Gate steps: fmt, clippy, clippy-android (`aarch64-linux-android`), test, scenario smoke (debug build), repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, serde 1.0.229, serde_json 1.0.151, fjall 3.1.11, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 38 s, 169 tests | 2026-10-02 | exit 0; `cargo repo-check` exit 0 with the new files staged (174 tracked files) |
| `cargo scenario s9-layers` (debug) | pass, 13 of 13 checkpoints, 1,685 frames, 2.8 s | 2026-10-02 | Vulkan, RTX 5070. Screenshots reviewed: panel under the Layers button, active row highlighted, grey buttons where nothing can act, eye struck through when hidden, "34 %" readable, panel whole on its first frame, reopened panel shows the same layers. The move-up checkpoints equal the faded one: black over faded black is the same pixels in either order |
| `cargo scenario smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop` (debug) | pass, hashes unchanged | 2026-10-02 | s7's Retry tap moved left of the Layers button |
| `replay`, release build | pass, golden `1f94cbfbd4b11e02` unchanged | 2026-10-02 | drawing p99 0.40 ms; undo/redo taps p99 2.0 ms, max 2.4 ms |
| Compositing GPU tests (3) | pass | 2026-10-02 | blue layer over red covers it, lowered it does not, 50 % gives (128, 0, 128), hidden or deleted layers drop out; five layer edits forward and back through `changed_regions` equal cold; a stroke drawn under another layer stays under it and commits equal to cold |
| Layer intent (5), runtime panel GPU (2), store layer round trip (1) tests | pass | 2026-10-02 | each panel action one undo step; choosing a layer amends; one slider touch one step; refused while a stroke is drawn; panel taps through the frame loop with the canvas equal to cold after each of six undos; a pen from the panel paints nothing; layer edits and an undone delete reopen exactly from the library |
| Upgrade on the tablet: the S8 build's library opened by the S9 build | pass | 2026-10-02 | `cargo android-install` over the installed app: the author's drawing reopened unchanged (`artifacts/device/s9-before-upgrade.png`, `s9-upgraded.png`); a copy of that library is in `artifacts/device/s9-library-schema1.tar` |
| S9 walk-through on the tablet (injected input) | pass | 2026-10-02 | MovinkPad 14, 2880 x 1800. Panel, add, outside tap closes, pen stroke on the new layer, hide, show, opacity swipe to 34 %, choose Layer 1, move up, undo, redo, delete, undo, pen on the panel paints nothing, pen outside closes and draws, `am force-stop` + `am start -W`: same layers, order, opacity and active layer (`s9-panel.png` to `s9-reopened.png`). The test layer was then deleted: the author's drawing is back as one layer (`s9-restored.png`). No error or panic in logcat |
| Real pen and hand on the panel | not_run | — | the author judges at the G2 exit |
| `cargo android-install` | pass | 2026-10-02 | the S9 build is installed |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Layer panel: reorder by move up / move down only (no dragging rows); no rename, no thumbnails, no blend modes; the list scrolls past about 15 layers on the tablet (not exercised). Drawing on a hidden layer commits strokes nobody sees until it is shown again. While the only ink is black, reordering does not change the pixels.
- A layer property change (visibility, opacity, order) re-evaluates the whole canvas; an opacity slide does so every frame. Fine on the tablet so far (not timed).
- Palm rejection is basic: a hovering pen does not count; no contact size or shape. With injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate).
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22).
- History and editor state are not stored; a restart starts with an empty history. The brush is fixed (12 px, black, opacity 1) until S11; pressure maps linearly to the diameter; blending is in sRGB-encoded values.
- One project only; no gallery until S10. A project that failed to open stays in the library unreachable until then.
- A save rewrites the head every time; stroke records are compared by pointer from the start of each layer.
- `leave` waits at most 2 s on the main thread; a stroke still being drawn when the app is left is not saved.
- Storage failure on the tablet was not provoked (only through the scenario's store switch).
- GPU tests fail (not skip) on a machine without any adapter. Golden hashes are verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- No view transform: a stored canvas keeps its size, so after rotation part of it is off screen.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures.
- The screen painter is created with the first target's format. On API 28 and 29 the navigation bar stays visible. Gradle and Kotlin daemons hold `cargo android-build`'s output handles: redirect it to a file. No emulator; Android behaviour is checked over adb on the tablet.
- Scenarios: button positions are window pixels that move when the bar or panel layout changes; a palm is one touch at one point; screenshots are never compared.

## Pending author decisions

- Schema 2 (D24): S9 needed no format change, so none was made and S9's "the schema 1 sample upgrades" is met by the unchanged schema-1 sample. Options: (a) keep schema 1 until the first real format change, S11's recipe snapshots (recommended); (b) bump to a schema 2 with the same layout now, only to exercise a 1 -> 2 migration step and a second frozen sample.
- Reorder by buttons instead of dragging rows (D23), and the opacity slider in the panel footer instead of behind a per-layer menu: both deviate in detail from Procreate's layer panel; drag-to-reorder is a ROADMAP candidate.
