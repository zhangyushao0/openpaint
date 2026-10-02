# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G1 one stroke, end to end. S0 to S3 done; S1 is confirmed on the tablet. Next slice: S4 in ROADMAP.

## Working features

- core: `Artwork` is an immutable value (`Arc` state: canvas, layers bottom to top, active layer, id counters); `Artwork::new(canvas)` starts with "Layer 1". `apply(ArtworkCommand::Layer(..))` returns `Applied { artwork, change }` for create, delete, reorder, select, rename, visibility, opacity and lock; `StrokeSession` (begin on the active layer, extend all-or-nothing, finish as one version, cancel to the base); `to_parts` / `from_storage` with full validation; typed `CoreError`. Layer strokes are a chunked `StrokeList` (256 per chunk) shared between versions. No dependencies; used by render and the harness.
- render, devices: `Core::headless()` opens a wgpu device without a window, keeps instance and adapter so windows attach later through `Core::create_surface(window, size)` (a `SurfaceWindow`: `raw-window-handle` window and display handles; Fifo, first non-sRGB format), and builds the evaluator's compute pipeline. `Core::render_clear` clears an offscreen `Rgba8Unorm` target and reads back an `RgbaImage` with an FNV-1a 64 `content_hash`. `adapter/` holds `UiPainter` (egui-wgpu) and `UiFrame`; `Core::present` clears the surface to the canvas colour and paints the UI over it, `Core::render_ui` does the same offscreen with readback.
- render, evaluation: `RegionEvaluator::new(&core, &target).evaluate(&FrameRequest { artwork, active, background }, &[PixelRect])` is the single entry. `CanvasTarget` is an `Rgba8Unorm` storage texture at canvas size that keeps its pixels between evaluations; `read` returns an `RgbaImage`. Strokes are round and continuous: each pair of samples is a tapered capsule (radius = size / 2 x pressure), a stroke's coverage is the maximum over its capsules with one-pixel antialiasing, its opacity applies once; layers composite with their opacity, hidden layers are skipped, the active stroke draws above its layer's finished strokes. `stroke_region(style, samples)` gives the pixels new samples can change. Ops are binned into 16 px tiles; the opcode and record layouts are generated into WGSL from `protocol.rs` (DECISIONS D16).
- ui: `shell(ui)` draws a 48-point top bar labelled "OpenPaint"; the rest of the screen is the canvas.
- runtime: `Runtime` is the frame loop a host drives from one thread: `start(density)`, `attach(window, size)`, `resize(size, density)`, `detach()`, `motion(&MotionBatch)` (logs one line per batch under target `openpaint::input`), `frame() -> wants another frame`, `screenshot(size)`. Typed input: `MotionBatch { phase, buttons, pointers }`, `Pointer { id, tool, samples }`, `PointerSample { position, pressure, tilt, orientation, time_ms }`; history samples come first, the current one last. The canvas is cleared white (`CANVAS`); strokes reach the screen in S4.
- android: Kotlin `MainActivity` (no AndroidX, no GameActivity, no winit) owns a full-screen `SurfaceView`, paces frames with `Choreographer` on the main thread, hides system bars on API 30+, handles rotation through `configChanges`, and forwards every `MotionEvent` (touch and generic, so hover too) as primitive arrays: action, action index, button state, pointer ids, tool types, per step and pointer x/y/pressure/tilt/orientation, per step time in ms. Rust decodes them in `motion.rs` (host-tested) and hands them to the runtime; JNI errors and panics are logged under tag `openpaint`, never thrown into Java.
- Build: `cargo android-build` runs cargo-ndk (arm64-v8a, API 28, release) into `target/android-jni/` and the Gradle 8.13 wrapper jar (`assembleDebug`, AGP 8.13.2, Kotlin 2.2.21, compile/target SDK 35); `cargo android-install` also runs `adb install -r` and starts the activity. SDK from `ANDROID_HOME` / `ANDROID_SDK_ROOT` / Android Studio's default path; NDK from `ANDROID_NDK_HOME` or the newest under `<sdk>/ndk/`.
- harness: `render-smoke` writes `artifacts/harness/smoke.png` (hash `261e692a5ece1106`); `render-stroke` evaluates a fixed 48-sample wave (size 40, 80 % opacity, pressure 0.1 -> 1 -> 0.1) on a 320 x 200 white canvas and writes `artifacts/harness/stroke.png` (hash `ba8523102ad38e3a`); `shell-smoke` writes `artifacts/harness/shell.png`, one runtime frame at 2560 x 1600, density 2.
- Gate steps: fmt, clippy, clippy-android (`openpaint-android` for `aarch64-linux-android`; needs the rustup target, not the NDK), test, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2. No windowing crate in the graph.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows, warm cache) | pass, 11.1 s | 2026-10-02 | fmt, clippy, clippy-android, test, repo-check |
| render GPU test `clear_readback` | pass on Vulkan and DX12 | 2026-10-02 | RTX 5070; same golden hash on both backends |
| render GPU tests `stroke` (golden wave, flat seamless body, pressure taper, hidden layer, incremental = cold, scattered regions = full) | pass on Vulkan and DX12 | 2026-10-02 | RTX 5070; golden `ba8523102ad38e3a` identical on both backends |
| render protocol layout test | pass | 2026-10-02 | naga offsets and sizes of `Op`, `Tile`, `Params` equal the Rust records; opcode constants equal the enum |
| `render-stroke` PNG | pass | 2026-10-02 | inspected: smooth S-curve, tapered round ends, one flat translucent body without seams |
| Evaluator pipeline on the tablet | pass | 2026-10-02 | built at app start on Adreno 735 (Vulkan); no error or panic in logcat. Evaluation and readback on the device: not_run |
| runtime GPU test `shell_frame` | pass | 2026-10-02 | top bar rows covered, canvas rows white, second frame identical |
| `shell-smoke` PNG | pass | 2026-10-02 | inspected: dark top bar with "OpenPaint" over a white canvas |
| `motion.rs` decoding tests | pass | 2026-10-02 | history order, hover, pointer index, malformed shapes |
| `cargo test -p openpaint-core` | pass, 33 tests | 2026-10-02 | success and error cases per layer command, stroke session, storage round trip and rejections, value ranges |
| `cargo android-build` / `android-install` | pass | 2026-10-02 | dev machine, NDK 29.0.14206865, JDK 17; 7 JNI symbols exported; no Gradle or Kotlin warnings |
| APK install, top bar on the tablet | pass | 2026-10-02 | MovinkPad 14 (Android 15, Adreno 735, Vulkan), 2880 x 1800 at density 1.75; `artifacts/device/s1-launch.png` |
| Real pen pressure, tilt, hover and buttons (author by hand, logcat) | pass | 2026-10-02 | 1,034 Stylus batches over 9 strokes; 364 distinct pressures, max 0.728; tilt 0.35 to 0.82 rad; orientation reported; hover enter/move/exit; stylus_primary, stylus_secondary and tertiary buttons |
| Rotation, both landscape orientations and portrait | pass | 2026-10-02 | surface resized 2880 x 1800 <-> 1800 x 2880 in the same process; no error, FATAL or panic; `artifacts/device/s1-portrait-now.png` |
| Background/foreground (HOME, then relaunch) | pass | 2026-10-02 | surface detached and reattached, same process, no error; `artifacts/device/s1-resume.png` |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet; workflow installs lavapipe for GPU tests |

## Known limits

- GPU tests fail (not skip) on a machine without any adapter; CI relies on software Vulkan.
- Golden hashes are verified on one desktop GPU (Vulkan and DX12) only; software and mobile adapters are unverified.
- Evaluation rebuilds the scene from the whole artwork and uploads every op on each call, and each pixel walks every op of its tile: cost grows with stroke count and length. No caches or bounded batching yet; S4's 8 ms budget for a 10,000-sample stroke will need them.
- The target is the canvas at one pixel per canvas pixel; there is no view transform yet (zoom, pan, rotation).
- Pressure maps linearly to the diameter; a pressure of 0 draws nothing. Tilt and azimuth are not used. Blending is in sRGB-encoded values.
- Input is logged only; egui gets no touch input yet (the normaliser is S4), so the top bar does not react.
- Frames and input run on the Android main thread; a slow frame delays input delivery. Latency is measured in S8.
- Event times are milliseconds (`getEventTime`); nanosecond times need API 34.
- The UI painter is created with the first surface's format; a later surface with another format fails with `RenderError::Invalid` (not seen on any device yet).
- On API 28 and 29 the navigation bar stays visible (the system-bar hiding needs API 30).
- The Gradle and Kotlin daemons outlive `cargo android-build` and hold its output handles: piping the command into `tail` or `grep` waits for them. Redirect to a file instead.
- No emulator image is installed on the dev machine; Android lifecycle is checked by hand over adb on the tablet.
- The harness renders offscreen only, so window surface creation is covered by the `SurfaceWindow` bound (a window without a display handle does not compile) and by the device, not by a host test.
- core: a stroke style is size, opacity and an opaque sRGB colour only (recipes arrive in S11); layer lock blocks strokes only; drawing on a hidden layer is allowed (and invisible).
- core: every layer command clones the layer table (O(layers), strokes shared); fine at the 256-layer limit.

## Pending author decisions

- None.
