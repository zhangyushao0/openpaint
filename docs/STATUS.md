# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 in progress: S9 (layers) and S10 (artwork library) done; next slice: S11 in ROADMAP. G1 exit passed 2026-10-02 (the author judged pen feel and palm rejection on the tablet).

## Working features

- The canvas: a dark top bar with, left to right, Gallery (a button), undo and redo arrows, and at its right end the Layers button (a stack of sheets); below it a white canvas. Draw with the pen and a 12-pixel black round stroke follows the tip with pressure, ending in the brush's round tip where the ink stopped. A finger draws nothing, by decision (D18). One screen pixel is one canvas pixel.
- Gallery (S10, D25): tap Gallery or press the back button on the canvas: the artwork and its thumbnail are stored, then the gallery replaces the canvas (if storing fails, the canvas stays with Retry). Bar: Gallery at the left, "+" at the right end (a new blank artwork the size of the canvas area, opened at once). Cards in a left-aligned grid, the artwork changed last first: the picture on the bottom of its box with an outline ("No preview" until the artwork has been left once), the title, a copy button and a bin. Tap a picture to open it (history starts empty); tap a title for the Rename dialog under the top bar with the soft keyboard (the title selected, typing replaces it; Done or the keyboard's Done renames, Done is grey while empty; Cancel, back or a tap outside keep it); the copy button puts a copy with the same title first; the bin asks "Delete “title”? This can't be undone." with Cancel and Delete. Back in the gallery closes a dialog, otherwise it is the system's back. A failed library call shows a notice with OK in the bar. Titles: trimmed, 1 to 80 characters, one line; new and upgraded artworks are "Untitled Artwork".
- Reopen: the app reopens where it was left: on the canvas of the artwork that was open, or in the gallery. An empty library starts on a blank canvas, stored with its first stroke.
- Layers (S9, D23): the Layers button opens a panel under it at the right edge: header "Layers" with a plus, rows top of the stack first (active highlighted, "34 %" when not opaque, an eye), then for the active layer Opacity with a slider, move up, move down and delete (no confirmation; undo restores). Unavailable buttons are grey. A contact starting on the panel never paints; a contact on the canvas outside closes it, and a pen stroke there also draws. Compositing bottom to top, Normal with layer opacity.
- Undo and redo (S6): one version per tap, the canvas updates in the same frame. Every layer change is one step; choosing the active layer is not. Grey while the pen draws. History belongs to one visit of an artwork.
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts; a pen landing cancels touches already down; a system-cancelled touch never taps.
- Autosave (S7): every finished stroke, layer change, undo and redo is saved in the background; leaving the app hands over what is left plus the thumbnail (waits up to 2 s). A failed save shows "Couldn't save. Your drawing is still here." and Retry; an unreadable last artwork starts a blank one with "Couldn't open the last drawing. This is a new one." and OK (the unreadable one stays listed in the gallery and can be deleted).
- app: `Session` (per artwork) with stroke, history, storage, layer and gallery intents; `Gallery` flow with `LibraryRequest`/`LibraryAnswer`, `GalleryProjection`; `ScreenProjection { Starting, Canvas, Gallery }`; `Title`, `Thumbnail`, `ProjectSummary`; `ProjectStore { open_last, list, create, load, save, save_thumbnail, close, rename, duplicate, delete }`; `PortError` adds `Missing`.
- store: `Library::at(dir)` on fjall, schema 2 (D25): catalogue entries (title, touched), a change counter, `last` = the artwork left open, thumbnails as derived records; head and stroke records as in schema 1 (D21); one synced batch per call; schema 1 upgrades in one batch on open (`migrate.rs`, pure).
- render: `Core::headless()`, `RegionEvaluator` as the single evaluation entry (D16, D17), `changed_regions`, `ScreenPainter`, `Core::thumbnail` (GPU box means, `thumbnail_reference` reproduces them exactly), FNV-1a 64 `content_hash`.
- ui: theme in `design` (adds `CARD_PICTURE`, `CARD_GAP`, `DIALOG_WIDTH`, `BACKDROP`); icons add Duplicate; components add `dialog`, `text_field`, `artwork_card`; `Shell { show(&ScreenProjection), canvas_area, route, back }` with the layer panel and the gallery screen.
- runtime: `Runtime::start(density, Box<dyn ProjectStore>)` with the storage worker on `worker::Queue` (FIFO; the only place a thread starts); `Screen { Opening, Canvas, Gallery }`; host calls `attach`, `resize`, `detach`, `leave`, `motion`, `text(TextInput)`, `keyboard_wanted`, `back`, `frame`; for tests and the harness `frame_offscreen`, `read_offscreen`, `read_canvas`, `screenshot`, `projection`, `screen`, `storing`, `artwork`.
- android: Kotlin `MainActivity` and `CanvasView` (the surface, and the soft keyboard's editor while a field has the focus, D26), `Native.create/leave/attach/resize/detach/frame/motion/text/deleteSurrounding/key/keyboard/back`; back goes to the app first; `PenLatency` logs per stroke under tag `openpaint`.
- harness and scenarios: `cargo scenario <name>` with steps `tap`, `drag`, `stroke` (`lift`, `palm`), `type`, `back`, `wait`, `restart`, `storage_fails`, `storage_works`, `checkpoint`, `screenshot` (D19). Scenarios: `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery` (2880 x 1800 at 1.75: Gallery (100, 42), "+" (2810, 42); card k: picture (280 + 504k, 380), title (178 + 504k, 518), copy (378 + 504k, 518), bin (462 + 504k, 518); Rename Done (1699, 350); delete dialog Cancel (1538, 956), Delete (1691, 956)).
- Build: `cargo android-build` / `cargo android-install` (cargo-ndk arm64-v8a API 28, Gradle 8.13, AGP 8.13.2, Kotlin 2.2.21, SDK 35). Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, serde 1.0.229, serde_json 1.0.151, fjall 3.1.11, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 43.5 s, 188 tests | 2026-10-02 | exit 0; `cargo repo-check` exit 0 with the new files staged (197 tracked files) |
| `cargo scenario s10-gallery` (debug) | pass, 7 of 7 checkpoints, 2,005 frames, 3.0 s | 2026-10-02 | Vulkan, RTX 5070. Screenshots reviewed (`artifacts/scenarios/s10-gallery/`): cards with matching pictures, Rename dialog above the keyboard's place with the title selected, typed text replacing it, copy listed first, delete dialog centred over the dimmed gallery, gallery reopened after a restart, changed artwork listed first. Hit targets 48 points; dialogs whole in their first frame |
| `cargo scenario smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers` (debug) | pass, hashes unchanged | 2026-10-02 | the Gallery button moved undo and redo right; their old tap points still hit them |
| Thumbnail GPU test, runtime gallery GPU test | pass | 2026-10-02 | the GPU thumbnail equals the CPU box means byte for byte at three sizes; through the frame loop on a real library: the stored thumbnail equals the box means of the canvas left, the title field asks for the keyboard, committed text and Enter rename, the reopened artwork equals a cold evaluation |
| Store tests: gallery (6), schema samples (2) | pass | 2026-10-02 | list order, rename keeps the place, duplicate is independent, delete leaves no record, `last` after close and load, unreadable thumbnail left out, unreadable entry refused; schema-1 sample opens upgraded with every record kept; schema-2 sample byte-exact |
| Upgrade on the tablet: the author's schema-1 library opened by the S10 build | pass | 2026-10-02 | backup `artifacts/device/s10-library-before.tar`; log "library upgraded from schema 1 (1 projects)"; the drawing reopened unchanged (`s10-before-upgrade.png`, `s10-upgraded.png`) and shows in the gallery as Untitled Artwork with its thumbnail (`s10-gallery-author.png`) |
| S10 walk-through on the tablet (injected input, Gboard) | pass | 2026-10-02 | MovinkPad 14: Gallery, "+", pen strokes, back to the gallery, Rename with the soft keyboard (Gboard key taps compose and commit, `adb shell input text` types, Gboard backspace and Done), copy, `am force-stop` + `am start -W` reopens the gallery, back closes the delete dialog, both test artworks deleted, the author's drawing opened unchanged, back in the gallery leaves the app (`s10-*.png`). No error or panic in logcat. The installed build is S10 with the author's drawing as the only artwork, open on the canvas (`s10-library-after.tar`) |
| `replay`, release build | not_run this slice | — | golden `1f94cbfbd4b11e02` last confirmed in S9; the evaluator did not change |
| Real pen and hand on the gallery and the layer panel | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Gallery: duplicate and delete are buttons on each card, not Procreate's swipe; order is by last change only; no selection of several, no folders, no canvas size menu (a new artwork fills the canvas area). A thumbnail is written when an artwork is left, so after a crash it can show an older state.
- Text: egui's fonts have no CJK glyphs (a Chinese title shows boxes). Closing the keyboard with its own button leaves the field focused; tapping the field does not bring the keyboard back (Cancel and rename again does).
- Layer panel: reorder by move up / move down only; no rename, no thumbnails, no blend modes; a layer property change re-evaluates the whole canvas (an opacity slide every frame).
- Palm rejection is basic: a hovering pen does not count; no contact size or shape. With injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate).
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22).
- Editor state is not stored. The brush is fixed (12 px, black, opacity 1) until S11; pressure maps linearly to the diameter; blending is in sRGB-encoded values.
- A save rewrites the head and the catalogue entry every time; stroke records are compared by pointer from the start of each layer.
- `leave` waits at most 2 s on the main thread; a stroke still being drawn when the app is left is not saved. Storage failure on the tablet was not provoked (only through the scenario's store switch).
- GPU tests fail (not skip) on a machine without any adapter. Golden hashes are verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- No view transform: a stored canvas keeps its size, so after rotation part of it is off screen.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures.
- The screen painter is created with the first target's format. On API 28 and 29 the navigation bar stays visible. Gradle and Kotlin daemons hold `cargo android-build`'s output handles: redirect it to a file. No emulator; Android behaviour is checked over adb on the tablet.
- Scenarios: button positions are window pixels that move when a layout changes; a palm is one touch at one point; screenshots are never compared; a checkpoint needs a canvas on screen.

## Pending author decisions

- Gallery details that deviate from Procreate: duplicate and delete as buttons on every card instead of swipe-left; Rename as a dialog under the top bar (so the soft keyboard never covers it) instead of editing the title in place; the gallery ordered by last change instead of by hand. Recommended: keep until the author has used it on the tablet.
- Reorder by buttons instead of dragging rows (D23), and the opacity slider in the panel footer instead of behind a per-layer menu: both deviate in detail from Procreate's layer panel; drag-to-reorder is a ROADMAP candidate.
