# Status

Updated 2026-10-02. Current facts only; rewritten at the end of every slice, never appended.

## Stage

G2 slices done: S9 (layers), S10 (artwork library), S11 (brush recipes), S12 (eraser, Alpha Lock, colour) and S13 (export, share, reference image). The G2 exit (W1) awaits the author on the tablet; no further slice starts before it. G1 exit passed 2026-10-02.

## Working features

- The canvas: a dark top bar with Gallery and Actions (a spanner) at its left end and, at its right end, right to left, the colour button (a round swatch of the current colour), Layers, Eraser and Brush; below it a white canvas. Over the canvas's left edge, centred on it, floats the side bar: the size slider, the eyedropper, the opacity slider, then undo and redo. A finger draws nothing, by decision (D18). One screen pixel is one canvas pixel. One panel (actions, colour, layers, brushes) at a time.
- Actions (S13, D31): a panel under the spanner: header "Actions"; under Share: Save PNG… (Lossless), Save JPEG… (Smaller), Share PNG…, Share JPEG…; under Canvas: the Reference switch and Choose reference image… (Procreate's Actions > Share and Canvas > Reference).
- Export (S13): Save opens the system's save dialog suggesting "<title>.png" or ".jpg"; the artwork is rendered at its full canvas size on a worker while drawing goes on ("Saving PNG…" in the top bar), then "Saved “Untitled Artwork.png”." with OK (it also goes when a contact lands on the canvas). PNG holds every canvas pixel as on screen; JPEG is quality 92 on the white paper. Share writes the file into the app's share directory and opens the system's share sheet; Back closes it and nothing is sent. Cancel in a dialog says nothing; a write that fails says "Couldn't write the PNG file. Is there space left?" with OK. Exporting is no history step.
- Reference image (S13): turning Reference on with no picture yet opens the system's picker (PNG and JPEG); "Opening the image…" while it is read (pictures over 2048 px on the long side are scaled down; files over 64 MiB or 50 MP and non-images are refused with a notice). A window titled Reference floats over the canvas's lower right: drag the title bar to move it (kept below the top bar and on screen), + and − zoom about its middle from 1x to 8x, drag the picture to pan, × closes it and turns the switch off; on again shows the same picture without the picker. The eyedropper picks the picture's pixel exactly. A contact starting on the window never reaches the canvas. The reference lives for this run of the app (kept across artworks, gone after the process ends); it is never stored.
- Tools (S12, D30): the pen paints with the Brush tool or takes paint away with the Eraser tool; tapping the current tool's button opens its brush panel, the other's switches tools. The eraser uses the same three recipes with its own brush and its own size and opacity per brush; destination out on the active layer only.
- Colour (S12): the colour button opens "Colour": a hue ring around a saturation/value square, then up to 8 recent colours, newest first. Colours are 8-bit; black is the default.
- Eyedropper (S12): the side-bar button between the sliders; the next contact on the canvas or the reference window, pen or finger, picks the colour under it while it moves; lifting settles it and the tool used before returns.
- Alpha Lock (S12): a switch in the layer panel's footer for the active layer; strokes begun while it is on paint only where the layer has paint. The eraser on an alpha-locked layer does nothing. Toggling is an undoable step.
- Brushes (S11, D27): Pencil, Ink Pen (the default, 12 px, 100 %), Soft Brush, each row with a stroke the evaluator painted; the panel stays until a contact lands on the canvas outside it. Brush, tool and colour survive restarts (stored apart from artworks).
- Side bar (S11): vertical sliders, 0 at the bottom; a bubble shows "12 px" or "56 %" while held; size follows a square curve over each brush's range. Undo and redo (S6): one version per tap; grey while the pen draws.
- Gallery (S10, D25): tap Gallery or press back on the canvas: the artwork and its thumbnail are stored, then the gallery replaces the canvas. Cards, changed last first: picture, title (tap for Rename), copy, bin (asks before deleting).
- Reopen: the app reopens where it was left: on the canvas of the artwork that was open, or in the gallery.
- Layers (S9, D23): header "Layers" with a plus, rows top of the stack first, then for the active layer Opacity, Alpha Lock, move up, move down and delete (undo restores).
- Palm rejection (S8): while the pen is on the glass every touch is a palm until it lifts; a pen landing cancels touches already down.
- Autosave (S7): every finished stroke, layer change, undo and redo is saved in the background; leaving the app hands over what is left plus the thumbnail (waits up to 2 s).
- app (S13): `transfer.rs` (`Transfers` owns the export and reference flows, one system picker at a time; `ImageFormat`, `Delivery`, ports `ImageSink`/`ImageSource`, `HostRequest { SaveFile, ShareFile, Send, OpenImage }`, `HostAnswer`, `TransferJob`, `ExportError`, `ImageError`, `TransferProjection`), `export.rs` (`ExportIntent`, `ExportJob`, `ExportNotice`, `file_name`), `reference.rs` (`ReferenceIntent`, `ReferenceImage::fitted`/`color_at`, limits); `Intent::Export`, `Intent::Reference`; `EditorAction::{Export, ChooseReference}`; `SessionProjection::with_transfers`; `Project::title`.
- render: `Core::export(artwork, paper)`: a target of its own, the whole canvas through `RegionEvaluator`, read back; nothing shared with the screen's target.
- store: `image_file.rs` (`encode_image`, `decode_image`, `open_reference`) on the `image` crate; schema unchanged (4); `load` returns the catalogue title.
- runtime: `transfer.rs`, the `openpaint-transfer` worker (`worker::Queue`, holds an `Arc<Core>`): render, encode, write; read, decode, scale down; panics become failures. `Runtime::host_request`, `answer`, `read_canvas`.
- ui: `actions_panel.rs`, `reference_window.rs`, `notices.rs`; components `floating`, `menu_row`, `status`; icons Actions, Close, Minus, Done.
- android: `Pickers.kt` (ACTION_CREATE_DOCUMENT, ACTION_OPEN_DOCUMENT, ACTION_SEND chooser), `ShareProvider.kt` (read-only `content://dev.openpaint.share/<name>` from `cacheDir/share`, no AndroidX), `jni_transfer.rs` (detached descriptors become `File`s), `transfer.rs` (`FileSink`, `FileSource`, `share_file`, request lines).
- harness and scenarios: `cargo scenario <name>`; scenarios `smoke`, `s4-first-stroke`, `s6-history`, `s7-resume`, `s8-device-loop`, `s9-layers`, `s10-gallery`, `s11-brushes`, `s12-colour-eraser`, `s13-export-reference`. Steps added in S13: `system` (`cancel`, `text_file`, `test_image`, `choose_file`, `full_storage`, `share_sheet`; devtools `system.rs` plays the platform) and `exported` (a PNG must equal the canvas, a JPEG stay within a mean difference). At 2880 x 1800 and 1.75: Actions (226, 42), Save PNG (288, 295), Save JPEG (288, 379), Share PNG (288, 464), Reference switch (500, 687); colour (2810, 42), Layers (2712, 42), Eraser (2614, 42), Brush (2516, 42), notice OK (2412, 42), Retry (2397, 42); side bar x 70: size slider y 760 (0) to 494 (1), eyedropper 837, opacity slider 1180 (0) to 914 (1), undo 1271, redo 1369; brush rows Pencil (2450, 252), Ink Pen (2450, 364), Soft Brush (2450, 476); other positions as in the scenario files.
- Build: `cargo android-build` / `cargo android-install`. Gate steps: fmt, clippy, clippy-android, test, scenario smoke, repo-check.
- Pinned (D14): wgpu 30.0.1, egui 0.36.2, egui-wgpu 0.36.2, pollster 1.0.1, png 0.18.1, image 0.25.10 (png, jpeg only), serde 1.0.229, serde_json 1.0.151, fjall 3.1.11, toml 1.1.6, jni 0.22.4, log 0.4.34, android_logger 0.15.1, raw-window-handle 0.6.2.

## Verified facts

| Item | Result | Date | Notes |
| --- | --- | --- | --- |
| `cargo gate` (dev machine, Windows) | pass, 35.8 s, 246 tests | 2026-10-02 | exit 0; repo-check 249 tracked files before the docs |
| Exported PNG equals the screen (runtime `tests/export.rs`) | pass | 2026-10-02 | through the frame loop: the file decodes to the canvas size and equals `read_canvas` byte for byte, incremental = cold after exporting, every screen row right of the side bar equals the file; the eyedropper picks the reference picture's colour exactly and draws nothing |
| `cargo scenario s13-export-reference` (debug) | pass, 7 of 7 checkpoints and exports, 6,254 frames, 10.1 s | 2026-10-02 | screenshots reviewed (`artifacts/scenarios/s13-export-reference/`): actions panel, picker asked and cancelled, not-an-image notice, reference window, moved, zoomed, panned, picked blue, closed and shown again, saved, save cancelled, save failed and dismissed, reopened without the window |
| Older scenarios (debug) | pass, hashes unchanged | 2026-10-02 | s4, s6, s7, s8, s9, s10, s11, s12 |
| Store tests: schema samples (4) | pass, unchanged | 2026-10-02 | S13 changed no format |
| S13 on the tablet, earlier build of the slice (injected input) | pass | 2026-10-02 | MovinkPad 14, a test artwork: picker (`s13-picker.png`), cancel, reference window moved and zoomed (`s13-moved-zoomed.png`), blue picked from it and painted (`s13-picked-blue.png`), PNG and JPEG saved through the save dialog (`s13-saved.png`, `s13-saved-jpeg.png`), share sheet opened and closed with Back (`s13-share-sheet.png`, `s13-share-closed.png`), reopen (`s13-reopened.png`) |
| S13 on the tablet, final build (injected input) | pass | 2026-10-02 | Save PNG into a test folder: 2880 x 1716, 35,663 bytes, rendered in 65 ms, encoded and written in 57 ms; the pulled file equals the canvas on screen (ImageMagick AE 0, `s13-final-exported.png` against `s13-final-saved.png`); share sheet showed the picture and closed with Back, nothing sent (`s13-final-share-sheet.png`, `s13-final-share-closed.png`); reference chosen in the picker and (0, 0, 0) picked from it exactly (`s13-final-picked.png`). No panic or error in logcat |
| The author's artwork in the final build | pass | 2026-10-02 | backup `artifacts/device/s13-library-before.tar`; canvas and side bar identical to before the install (AE 0 below the top bar, `s13-before-install.png`, `s13-final-state.png`) |
| State left on the tablet | as found, but recents | 2026-10-02 | the test artwork deleted in the gallery; the two test folders under Pictures and Download deleted; Brush, Ink Pen at the author's size and opacity, black; the recent colours keep blue and black from the tests; rotation free; S13 final build installed |
| `replay`, release build | not_run this slice | — | golden `1f94cbfbd4b11e02` last confirmed in S9 |
| Brush and eraser feel with the real pen; colour, eyedropper, panels, export and reference by hand | not_run | — | the author judges at the G2 exit |
| Pen latency, palm rejection on the tablet | unchanged since S8 | 2026-10-02 | event to present call p50 7.5 to 9.4 ms; queue to glass p50 about 28 ms (D22) |
| CI (GitHub Actions `gate`) | not_run | — | no remote yet |

## Known limits

- Export: the flattened canvas only, on white; no layers, PSD, transparent background or other sizes. A shared file stays in the app's cache until the next share. While a system dialog is open no second export or picker starts.
- Reference: one picture, not stored with the artwork or across restarts; no pinch zoom inside the window (needs multi-touch routing), no resizing, no Canvas or Face modes. Scaling down uses box means in sRGB values.
- Brushes: three built-ins, no brush editing or import. Coverage is a union (D16), so a stroke never builds up over itself. Injected `adb` strokes have constant pressure.
- Colour: one HSV wheel and recents only; on black or grey a tap on the hue ring alone changes no visible colour. The eyedropper has no loupe; a canvas pick reads one texel back on the main thread. The pen's eraser end and buttons do nothing.
- Alpha Lock is captured per stroke; a layer property change re-evaluates the whole canvas.
- The side bar covers about 126 px of the canvas's left edge on the tablet and is not mirrored for the left hand. On windows under about 450 points it runs off the bottom; under about 700 points wide the top-bar notices no longer fit beside the buttons.
- Gallery: duplicate and delete are buttons on each card, not swipe; order is by last change only; no selection of several, no folders, no canvas size menu.
- Text: the UI font has no CJK glyphs, so a Chinese title or layer name shows as boxes. Closing the keyboard with its own button leaves the field focused.
- Layer panel: reorder by move up / move down only; no rename, no thumbnails, no blend modes.
- Palm rejection is basic: a hovering pen does not count; no contact size or shape. With injected input a touch landing mid-stroke cancels the pen's gesture and the stroke is dropped (ROADMAP candidate). On the tablet `adb` injected taps were refused by the system ("all windows would just receive ACTION_OUTSIDE") while a real pen was in use nearby; they worked again afterwards.
- Presentation dominates latency: about 28 ms from queueing a frame to the glass at 120 Hz (D22).
- Blending is in sRGB-encoded values. A save rewrites the head and the catalogue entry every time. Upgrading a library reads every stroke record into memory once.
- `leave` waits at most 2 s on the main thread; a stroke still being drawn when the app is left is not saved.
- GPU tests fail (not skip) on a machine without any adapter. Golden hashes are verified on one desktop GPU; only canvas pixels are golden.
- Each evaluation bins every finished op on the CPU; Vulkan on the dev machine stalls ~1.5 ms every ~4,000 `write_buffer` calls.
- No view transform: a stored canvas keeps its size, so after rotation part of it is off screen.
- egui gets pointer events from the first contact of each batch only; no multi-touch gestures.
- The screen painter is created with the first target's format. On API 28 and 29 the navigation bar stays visible. Gradle and Kotlin daemons hold `cargo android-build`'s output handles: redirect it to a file. No emulator; Android behaviour is checked over adb on the tablet.
- Scenarios: button positions are window pixels that move when a layout changes; a tap always spans two frames; screenshots are never compared; a slider can only be dragged, not held; the system's dialogs are played by devtools, never shown.

## Pending author decisions

1. G2 exit (W1): finish an inked and flat-coloured piece on the tablet, and judge with the real pen the feel of the brushes and the eraser, the colour wheel, the eyedropper and the panels, export (save and share) and the reference window. Record `pass` / `fail` here.
2. Layer panel (S9, D23): reorder by up/down buttons instead of dragging rows; opacity slider and Alpha Lock in the panel footer instead of behind a per-layer menu. Recommended: keep until used; drag-to-reorder is a ROADMAP candidate.
3. Gallery (S10): duplicate and delete as buttons on every card instead of swipe-left; Rename as a dialog under the top bar instead of editing in place; order by last change only. Recommended: keep until used.
4. Brushes (S11, D27): the brush panel stays open after a choice; the size slider follows a square curve over each brush's range; Ink Pen is the default. Recommended: keep until drawn with.
5. Eyedropper and gestures (S12, D30): only the side-bar button is wired. Recommended: Procreate's touch-and-hold with a finger, together with two-finger undo, as one gesture slice after G2.
6. The pen's eraser end (S12): today it does nothing. Recommended: erase with the eraser's brush (an Android-hardware choice; Procreate has none).
7. Reference image (S13, D31): kept for the app's run only, not stored with the artwork (Procreate keeps it per canvas, which would be a format change and a stored image asset). Recommended: keep until used; storing it belongs with the asset store (S22 or a later slice).
8. Actions entry (S13, D31): a spanner right of Gallery at the top bar's left end, as in Procreate; Save and Share each offer PNG and JPEG as four rows instead of one Share row with a format list. Recommended: keep until used.
