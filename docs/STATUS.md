# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G0 skeleton. S0 done. Next slice: S1 in ROADMAP.

## Working features

- render: `Core::headless()` opens a wgpu device without a window (`WGPU_BACKEND` selects the backend); `Core::render_clear(size, color)` clears an offscreen `Rgba8Unorm` texture and reads it back as a tightly packed `RgbaImage` with an FNV-1a 64 `content_hash`. Errors are `RenderError::{Device, Invalid}`.
- harness: `cargo run -p openpaint-harness -- render-smoke` writes `artifacts/harness/smoke.png` (300 x 200) and prints adapter, path and hash `261e692a5ece1106`.
- Pinned in the workspace (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1. egui and egui-wgpu are not used by any crate yet. No windowing crate in the graph (`cargo tree -i winit` finds nothing).

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows, warm cache) | pass, 3.8 s | 2026-10-02 | fmt, clippy, test, repo-check |
| render GPU test `clear_readback` | pass on Vulkan and DX12 | 2026-10-02 | RTX 5070; same golden hash on both backends |
| `render-smoke` PNG | pass | 2026-10-02 | inspected: uniform dark grey 300 x 200 |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet; workflow installs lavapipe for GPU tests |
| Android build | not_run | — | the host arrives in S1 |

## Known limits

- GPU tests fail (not skip) on a machine without any adapter; CI relies on software Vulkan.
- The golden hash is verified on one desktop GPU only; software and mobile adapters are unverified.
- `rust-toolchain.toml` has no `aarch64-linux-android` target yet; S1 adds it.

## Pending author decisions

None.
