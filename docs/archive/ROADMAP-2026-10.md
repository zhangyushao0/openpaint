# Roadmap archive, 2026-10

Finished slices moved out of `docs/ROADMAP.md` "Done" (G0 to G2).

- S0 2026-10-02 `feat(render): headless GPU device, offscreen clear and readback smoke (S0)`
- S1 2026-10-02 `feat(android): Kotlin host, surface frames with an egui top bar, typed motion batches (S1)`; confirmed on the tablet
- S2 2026-10-02 `feat(core): immutable artwork value, layer commands, stroke sessions, validated storage entry (S2)`
- S3 2026-10-02 `feat(render): region evaluator with tile-binned ops and a continuous round brush (S3)`
- S4 2026-10-02 `feat(runtime): pen input to strokes on the first screen (S4)`; confirmed on the tablet with injected input
- S5 2026-10-02 `feat(devtools): scenario runner with canvas checkpoints, cargo scenario, smoke in the gate (S5)`
- S6 2026-10-02 `feat(app): history with a byte budget, undo and redo in the top bar (S6)`; confirmed on the tablet with injected input
- S7 2026-10-02 `feat(store): fjall library with per-stroke records, autosave worker, save-failure retry (S7)`; confirmed on the tablet with injected input
- S8 2026-10-02 `feat(android): palm rejection, round stroke ends, pen latency probe on the tablet (S8)`; draw, undo, reopen and latency confirmed on the tablet with injected input; G1 exit awaits the author (pen feel, palm by hand)
- S9 2026-10-02 `feat(app): layers with a popover panel, undoable layer actions, ordered compositing (S9)`; confirmed on the tablet with injected input; no schema 2 (D24, the author to confirm)
- S10 2026-10-02 `feat(store): artwork library with a gallery, schema 2, thumbnails, rename by keyboard (S10)`; confirmed on the tablet with injected input and Gboard; the author's schema-1 library upgraded unchanged
- S11 2026-10-02 `feat(render): brush recipes, brush panel, side bar with size, opacity and history, schema 3 (S11)`; confirmed on the tablet with injected input; the author's schema-2 library upgraded pixel-identical
- S12 2026-10-02 `feat(render): eraser, Alpha Lock, colour wheel with recent colours, eyedropper by render picking, schema 4 (S12)`; confirmed on the tablet with injected input; the author's schema-3 library upgraded pixel-identical
- S13 2026-10-02 `feat(android): export, share and a floating reference image (S13)`; exported PNG equals the screen (test and tablet, AE 0); picking, saving and sharing confirmed on the tablet with injected input; G2 exit awaits the author
- S14 2026-10-03 `feat(ui): canvas view, navigation gestures, finger undo and redo, canvas size presets (S14)`; gestures, pen at zoom and rotation, taps and quick pinch confirmed on the tablet through the debug gesture instrumentation; no format change
- S15 2026-10-03 `feat(raster): ColorDrop fill with threshold hold, continue filling, fills as artwork content, schema 5 (S15)`; drop, hold-and-slide, fill tap, undo, hidden-layer refusal and reopen confirmed on the tablet with injected input; the author's schema-4 library upgraded pixel-identical
- S16 2026-10-03 `feat(render): accumulating pencil on canvas-fixed paper, StreamLine in the stroke intent, schema 6 (S16)`; pencil, ink and StreamLine confirmed on the tablet with injected pen paths; the author's schema-5 library upgraded pixel-identical
- S17 2026-10-03 `feat(ui): paper-white line art in floating islands, system fonts, pen hover outline, self-explanatory side bar (S17)`; option A chosen from three style sets; walk-through confirmed on the tablet with injected pen, hover and finger input; the author's canvas unchanged pixel for pixel
- G2 exit 2026-10-03: the author's W1 piece on the tablet, `pass`
- G2 follow-ups 2026-10-03 `feat(ui): eyedropper loupe, eraser outline while pressed, gesture-driven reference window (G2 follow-ups)`; walk-through confirmed on the tablet with injected pen and finger input; the author's library byte-identical
