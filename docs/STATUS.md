# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G0 skeleton. S0 and S1 done in code; S1's device criteria wait for the author (below). Next slice: S2 in ROADMAP.

## Working features

- render: `Core::headless()` opens a wgpu device without a window and keeps instance and adapter so windows attach later through `Core::create_surface(window, size)` (any `raw-window-handle` window, Fifo, first non-sRGB format). `Core::render_clear` clears an offscreen `Rgba8Unorm` target and reads back an `RgbaImage` with an FNV-1a 64 `content_hash`. `adapter/` holds `UiPainter` (egui-wgpu) and `UiFrame`; `Core::present` clears the surface to the canvas colour and paints the UI over it, `Core::render_ui` does the same offscreen with readback.
- ui: `shell(ui)` draws a 48-point top bar labelled "OpenPaint"; the rest of the screen is the canvas.
- runtime: `Runtime` is the frame loop a host drives from one thread: `start(density)`, `attach(window, size)`, `resize(size, density)`, `detach()`, `motion(&MotionBatch)` (logs one line per batch under target `openpaint::input`), `frame() -> wants another frame`, `screenshot(size)`. Typed input: `MotionBatch { phase, buttons, pointers }`, `Pointer { id, tool, samples }`, `PointerSample { position, pressure, tilt, orientation, time_ms }`; history samples come first, the current one last. The canvas is cleared white (`CANVAS`) until S3.
- android: Kotlin `MainActivity` (no AndroidX, no GameActivity, no winit) owns a full-screen `SurfaceView`, paces frames with `Choreographer` on the main thread, hides system bars on API 30+, handles rotation through `configChanges`, and forwards every `MotionEvent` (touch and generic, so hover too) as primitive arrays: action, action index, button state, pointer ids, tool types, per step and pointer x/y/pressure/tilt/orientation, per step time in ms. Rust decodes them in `motion.rs` (host-tested) and hands them to the runtime; JNI errors and panics are logged under tag `openpaint`, never thrown into Java.
- Build: `cargo android-build` runs cargo-ndk (arm64-v8a, API 28, release) into `target/android-jni/` and the Gradle 8.13 wrapper jar (`assembleDebug`, AGP 8.13.2, Kotlin 2.2.21, compile/target SDK 35); `cargo android-install` also runs `adb install -r` and starts the activity. SDK from `ANDROID_HOME` / `ANDROID_SDK_ROOT` / Android Studio's default path; NDK from `ANDROID_NDK_HOME` or the newest under `<sdk>/ndk/`.
- harness: `render-smoke` writes `artifacts/harness/smoke.png` (hash `261e692a5ece1106`); `shell-smoke` writes `artifacts/harness/shell.png`, one runtime frame at 2560 x 1600, density 2.
- Gate steps: fmt, clippy, clippy-android (`openpaint-android` for `aarch64-linux-android`; needs the rustup target, not the NDK), test, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2. No windowing crate in the graph.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows, warm cache) | pass, 10.5 s | 2026-10-02 | fmt, clippy, clippy-android, test, repo-check |
| render GPU test `clear_readback` | pass on Vulkan and DX12 | 2026-10-02 | RTX 5070; same golden hash on both backends |
| runtime GPU test `shell_frame` | pass | 2026-10-02 | top bar rows covered, canvas rows white, second frame identical |
| `shell-smoke` PNG | pass | 2026-10-02 | inspected: dark top bar with "OpenPaint" over a white canvas |
| `motion.rs` decoding tests | pass | 2026-10-02 | history order, hover, pointer index, malformed shapes |
| `cargo android-build` | pass | 2026-10-02 | dev machine, NDK 29.0.14206865, JDK 17; 16 MB debug APK, 7 JNI symbols exported; no Gradle or Kotlin warnings |
| APK install, top bar on the tablet | not_run | — | no device attached to the dev machine; author to confirm |
| Stylus pressure and tool type in logcat | not_run | — | needs the tablet |
| Background/foreground and rotation without a crash | not_run | — | needs the tablet |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet; workflow installs lavapipe for GPU tests |

## Known limits

- GPU tests fail (not skip) on a machine without any adapter; CI relies on software Vulkan.
- Golden hashes are verified on one desktop GPU only; software and mobile adapters are unverified.
- Input is logged only; egui gets no touch input yet (the normaliser is S4), so the top bar does not react.
- Frames and input run on the Android main thread; a slow frame delays input delivery. Latency is measured in S8.
- Event times are milliseconds (`getEventTime`); nanosecond times need API 34.
- The UI painter is created with the first surface's format; a later surface with another format fails with `RenderError::Invalid` (not seen on any device yet).
- On API 28 and 29 the navigation bar stays visible (the system-bar hiding needs API 30).
- The Gradle and Kotlin daemons outlive `cargo android-build` and hold its output handles: piping the command into `tail` or `grep` waits for them. Redirect to a file instead.
- No emulator image is installed on the dev machine; Android lifecycle has had no automated run.

## Pending author decisions

- Confirm S1 on the tablet: install, top bar visible, stylus log lines, background/foreground and rotation.
