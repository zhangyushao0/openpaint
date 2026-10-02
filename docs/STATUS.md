# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G1 exit passed 2026-10-02: S0 to S8 done; the author judged pen feel and palm rejection by hand on the tablet. Next slice: S9 in ROADMAP.

## Working features

- The first screen: open the app, a dark top bar with "OpenPaint", then undo and redo arrows; below it a white canvas. Draw with the pen and a 12-pixel black round stroke follows the tip with pressure; lift and it stays, ending in the brush's round tip where the ink stopped (the zero-pressure lift sample Android reports is not drawn). A finger draws nothing, by decision (D18: touch is reserved for gestures). One screen pixel is one canvas pixel.
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts: it neither paints, nor presses a button, nor disturbs the stroke. When the pen lands, touches already down are cancelled, so a hand resting on a button never taps it. A touch the system cancels (`ACTION_CANCEL`, `FLAG_CANCELED` on an up) never taps. A hovering pen does not count: with the pen off the glass a finger taps as usual.
- Undo and redo (S6): tap the arrows with a finger or the pen tip; each tap switches the artwork one version back or forward, the canvas updates in the same frame. An arrow with nothing left is dim grey and ignores taps; while the pen is drawing both are grey. A new stroke after undo drops what could be redone. A pen stroke that starts on the canvas stays a stroke when it lifts over the bar; a contact that starts on the bar never paints.
- Resume (S7): every finished stroke, undo and redo is saved in the background; nothing on screen changes. Leaving the app (`onPause`) saves what is left and waits up to 2 s. Opening the app shows the artwork saved last, pixel for pixel, at its own canvas size; history starts empty (it is not stored), so undo is grey after a restart. If a save fails, the right end of the top bar shows an alert icon, "Couldn't save. Your drawing is still here." and a Retry button; drawing goes on; the next stroke, a Retry tap or leaving the app tries again; a success removes the notice. If the library cannot be read, the app starts a blank artwork in a new project, leaves the old records untouched and shows "Couldn't open the last drawing. This is a new one." with OK.
- core: `Artwork` is an immutable value (`Arc` state: canvas, layers bottom to top, active layer, id counters); `apply(ArtworkCommand::Layer(..))` for create, delete, reorder, select, rename, visibility, opacity and lock; `StrokeSession` (begin, extend, finish, cancel); `to_parts` / `from_storage` with full validation; typed `CoreError`. Layer strokes are a chunked `StrokeList` (256 per chunk) shared between versions; `Artwork::bytes_since(base)`. No dependencies.
- app: normalised input (`InputBatch`, `Contact`, `InputSample`); `PalmRejection::filter(batch) -> Vec<InputBatch>`; `Session { new, open, after_failed_open, dispatch, artwork, active, projection, history, save_request, leave_request, save_finished, saving }`. Intents: `Stroke(Contact)` (pen Down/Move/Up/Cancel; zero-pressure samples of an Up are dropped), `History(Undo | Redo)`, `Storage(RetrySave | Dismiss)`; refusals are `AppError::Unavailable { action, availability }`. History: versions costed by `bytes_since`, 64 MiB budget (D20). Autosave: one save in flight; a failed version waits for a change, Retry or leaving (D21). Ports: `Project`, `ProjectStore { open_last, create, save }`, `PortError`.
- store: `Library::at(dir)` implements `ProjectStore` on fjall (schema 1, D21): a head record per project and one record per stroke; a save is one synced batch of the changed records; reading refuses truncated, misplaced, newer-schema and invalid data with typed errors.
- render: `Core::headless()`, `create_surface`, `RegionEvaluator::new(&core, &mut target).evaluate(&FrameRequest, &[PixelRect])` as the single entry (D16, D17), `changed_regions`, `ScreenPainter` drawing `ScreenFrame { surround, canvas, ui }` into a window or a `ScreenTarget`; FNV-1a 64 `content_hash` on readbacks.
- ui: theme in `design`, icons Undo, Redo, Alert; components `top_bar`, `icon_button`, `text_button`, `notice`, `canvas_frame` with `CanvasInput`; `Shell { show, canvas_area, route }`.
- runtime: `Runtime::start(density, Box<dyn ProjectStore>)` starts the storage worker (`worker::Latest`, the only place a thread starts); `attach`, `resize`, `detach`, `leave`, `motion` (normalise, then palm rejection), `frame`, `projection`, `storing`, and for the harness `frame_offscreen`, `read_offscreen`, `read_canvas`, `screenshot`, `wait_idle`, `artwork`, `adapter`. egui gets a cancelled contact as a release off screen (no click). `MotionPhase::PointerCancel { index }` for one pointer the platform took away.
- android: Kotlin `MainActivity` passes `getFilesDir()` to `Native.create`, calls `Native.leave` in `onPause`; full-screen `SurfaceView`, `Choreographer` pacing on the main thread (D15, D22), `MotionEvent`s as primitive arrays plus flags and `eventTimeNanos`. A `PenLatency` probe logs one line per stroke under tag `openpaint`: event to frame start, event to present call, frame work, each p50/p95/max. The logger keeps fjall and lsm-tree at warn. JNI errors and panics are logged.
- harness and scenarios: `render-smoke`, `render-stroke`, `shell-smoke`, `replay` (10,000-sample scribble, golden `1f94cbfbd4b11e02`); `cargo scenario <name>` with steps `tap`, `drag`, `stroke` (optional `lift`, optional `palm { at, lands: before | during }`), `wait`, `restart`, `storage_fails`, `storage_works`, `checkpoint`, `screenshot` (D19). Scenarios: `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop` (2880 x 1800 at 1.75, undo at (230, 42): a stroke lifting at a drifted zero-pressure point and the same stroke without it both hash `fd2fc755444a58d9`; a palm on the canvas before the pen `83ea2f0fd20aca1a`; a palm on undo during a stroke `b72a8c615f05e373`; a palm resting on undo before the pen lands `996871cb313b5815`; finger undo, then restart, `b72a8c615f05e373`).
- Build: `cargo android-build` (cargo-ndk arm64-v8a API 28 release, Gradle 8.13, AGP 8.13.2, Kotlin 2.2.21, SDK 35); `cargo android-install` also installs and starts the activity.
- Gate steps: fmt, clippy, clippy-android (`aarch64-linux-android`), test, scenario smoke (debug build), repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, serde 1.0.229, serde_json 1.0.151, fjall 3.1.11 (MSRV 1.90), jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2. No windowing crate in the graph.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 37.4 s, 157 tests | 2026-10-02 | first run after the last edit |
| `cargo scenario s8-device-loop` (debug) | pass, 7 of 7 checkpoints, 1,736 frames, 2.4 s | 2026-10-02 | Vulkan. `four-strokes.png`: four strokes, palms left no mark, undo light (nothing was undone) |
| `cargo scenario smoke`, `s4-first-stroke`, `s6-history`, `s7-resume` (debug) | pass, hashes unchanged | 2026-10-02 | |
| `replay`, release build | pass, golden `1f94cbfbd4b11e02` unchanged | 2026-10-02 | undo/redo taps 307 frames, p99 2.3 ms, max 2.7 ms |
| app palm rejection (5), stroke intent (9), runtime palm GPU tests (3), normaliser, pointer, android decode (5) and latency (3) tests | pass | 2026-10-02 | GPU: a hand on undo during a stroke, a hand resting on undo before the pen, a `PointerCancel` on undo: never undo; a finger tap afterwards does |
| Store, history, autosave, render, devtools runner tests | pass | 2026-10-02 | unchanged by S8 |
| S8 on the tablet: draw, undo, reopen (injected input) | pass | 2026-10-02 | MovinkPad 14, Android 15, landscape 2880 x 1800, 120 Hz. Four `input stylus swipe` strokes (`artifacts/device/s8-four.png`), finger tap on undo removes the fourth (`s8-undo.png`), `am force-stop` + `am start -W`: three strokes, arrows grey (`s8-reopened.png`). No error or panic in logcat; no fjall or lsm-tree lines |
| S8 on the tablet: stroke end | pass | 2026-10-02 | `input stylus swipe` reports the lift at the end point with pressure 0; the stroke now ends in a round cap where the ink stopped (`artifacts/device/s8-stroke-end-zoom.png`, crop of `s8-first-stroke.png`) |
| S8 on the tablet: touch resting on undo, then a pen stroke (injected) | pass | 2026-10-02 | the system itself cancels the touch when the pen lands (`ACTION_CANCEL`); the stroke is drawn, nothing undone (`s8-palm-on-undo.png`, `s8-palm-injected.png`) |
| S8 on the tablet: touch landing during a pen stroke (injected) | as designed, see limits | 2026-10-02 | with injected events the dispatcher cancels the pen's own gesture when the touch lands; the stroke is dropped, nothing undone |
| Pen latency on the tablet, software part (injected strokes, 7 strokes of 300 to 2,000 ms) | measured | 2026-10-02 | kernel event time to the frame's present call: p50 8.8 to 9.4 ms, p95 10.2 to 10.5 ms, max 10.7 to 19.7 ms. Of it, waiting for the next Choreographer frame p50 6.7 to 6.9 ms; frame work (UI, evaluation, encode, submit, present call) p50 2.0 to 2.5 ms, p95 3.0 to 3.8 ms, max 6.9 ms of an 8.3 ms vsync. One earlier run measured frame work p50 7.2 ms, p95 8.6 ms (not reproduced) |
| Queue to on-screen present on the tablet (`dumpsys SurfaceFlinger --latency`, same strokes) | measured | 2026-10-02 | buffer queued to present fence p50 28.2 to 28.6 ms, p95 29.3 to 30.1 ms (about 3.4 vsyncs); GPU finished p50 4.6 ms after queueing. Unchanged with `desired_maximum_frame_latency` 1 (tried, reverted) |
| Estimated event-to-glass on the tablet | about 37 ms p50 | 2026-10-02 | 9 ms software + 28 ms presentation. Not covered: digitizer scan and firmware before the kernel stamps the event (injected events have none), display scan-out and pixel response after the present fence |
| Real pen pressure and feel on the tablet | pass (author) | 2026-10-02 | G1 exit: light and heavy, fast and slow, flicks; stroke ends round |
| Palm rejection with a real hand on the tablet | pass (author) | 2026-10-02 | palm on the canvas and on undo while drawing, hand over the top bar: no marks, no lost strokes, nothing undone; strokes back after swiping the app away |
| Pen latency with the real pen (author's strokes, 15 strokes of 5 to 494 events) | measured | 2026-10-02 | event to present call p50 7.5 to 8.5 ms, p95 8.5 to 11.7 ms, max 19.3 ms; frame work p50 1.4 to 2.2 ms, max 9.4 ms; no error or panic in logcat |
| `cargo android-install` | pass | 2026-10-02 | dev machine, NDK 29.0.14206865, JDK 17; the S8 build is installed with a cleared library |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Palm rejection is basic: a hovering pen does not count, so a hand that taps a button and lifts before the pen touches counts as a tap; no contact size or shape is used. Whether the system's own palm and stylus-over-touch rejection stops touches during a real stroke was not observed; with injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate).
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22, ROADMAP candidate). Input is batched to Choreographer frames; unbuffered dispatch is not requested.
- The latency probe counts every frame after a pen event as consuming it; it does not know whether the frame reached the screen.
- History and editor state are not stored; a restart starts with an empty history. There is no editor state yet (the brush is fixed).
- One project only: the app opens the last one and has no gallery until S10. A project that failed to open stays in the library unreachable until then.
- A save rewrites the head every time and compares each layer's strokes by pointer from the start; bytes written are the changed strokes only.
- `leave` waits at most 2 s on the main thread; a longer save continues in the background and is lost if the system kills the process first. A stroke still being drawn when the app is left is not saved.
- Storage failure on the tablet was not provoked (only on the dev machine through the scenario's store switch). The notice text does not distinguish a full disk from other errors.
- GPU tests fail (not skip) on a machine without any adapter; CI relies on software Vulkan.
- Golden hashes are verified on one desktop GPU (Vulkan and DX12); software and mobile adapters are unverified. Only canvas pixels are golden.
- The finished strokes still cost per frame: each evaluation bins every finished op of the scene on the CPU.
- Vulkan on the dev machine: a ~1.5 ms CPU stall every ~4,000 `write_buffer` calls; DX12 shows none. Not seen in the tablet's frame work numbers.
- No view transform: one screen pixel is one canvas pixel. A new canvas takes the size of the canvas area on the first frame; a stored canvas keeps its size, so after rotation part of it is off screen.
- The brush is fixed (12 px, black, opacity 1) until S11. Pressure maps linearly to the diameter; tilt and azimuth are stored, not used. Blending is in sRGB-encoded values.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures. A button that became enabled takes a press only after one frame has laid it out enabled.
- `changed_regions` redraws the whole canvas for any layer property change. History holds strokes only.
- The screen painter is created with the first target's format; a later surface with another format fails with `RenderError::Invalid`.
- On API 28 and 29 the navigation bar stays visible; before API 34 event times for the latency log are whole milliseconds.
- The Gradle and Kotlin daemons outlive `cargo android-build` and hold its output handles: redirect its output to a file.
- No emulator image on the dev machine; Android behaviour is checked over adb on the tablet. `cargo android-install` restarts the app, so history is empty after it.
- Scenarios: a palm is one touch at one point; samples carry no tilt or orientation; button positions are window pixels that move if the bar's layout changes; a failed checkpoint gets a diff only if it passed before on this machine (D19). Screenshots are never compared.

## Pending author decisions

None. Settled 2026-10-02: undo/redo move into the side bar in S11; a hovering pen keeps not counting for palm rejection; undo history is not kept across an app restart (matches Procreate, which clears it when leaving the canvas).
