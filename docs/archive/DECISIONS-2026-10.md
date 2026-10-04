# Archived decisions, 2026-10

Superseded decisions moved here whole from `docs/DECISIONS.md`. Written, never read.

## D24 2026-10-02 S9 keeps schema 1: layers needed no format change

- Decision: S9 ships without a schema 2. Schema 1 (D21) already stores everything S9 edits: the layer order, each layer's name, visibility, opacity and lock, and the active layer in the head; strokes keyed by layer id. The first migration step 1 -> 2 and its frozen sample land with the first slice that changes the format (S11 stores recipe snapshots). Pending the author's confirmation (STATUS).
- Reason: a schema 2 identical to schema 1 would add a migration that changes nothing; adding fields for later slices (alpha lock, blend, groups) would reserve format for features not yet accepted. Proven instead: the schema-1 sample still opens with its three layers, layer edits round-trip through the library, and the tablet's library written by the S8 build opened unchanged in the S9 build.
- Consequences: supersedes D21's line "schema 2 (S9) adds a migration over these records"; ROADMAP's S9 criterion "the schema 1 sample upgrades" is met by the unchanged sample test. D25 settles the pending question: schema 2 came with S10, the first real format change. Superseded by D25.

## D21 2026-10-02 Schema 1: one record per stroke, hand-written little-endian values, saves rewrite only what changed

- Decision: the library is one fjall database with keyspaces `library` (`last`, `next` project id) and `projects`: per project a head record (schema, canvas, counters, layer table with stroke counts) and one record per stroke keyed `[project][s][layer][index]` (big-endian keys). Values are fixed-width little-endian fields written by `store/src/codec.rs`, no serialization crate. A save is one atomic batch synced with `SyncData`: the head plus the stroke records after each layer's common prefix with the version the store last wrote (compared by sample pointer, id and style); after any failure the next save rewrites the project whole. The frozen sample `crates/store/tests/samples/schema-1.bin` is a dump of every record; the gate writes the sample artwork and compares bytes, and opens the sample.
- Autosave (app `storage.rs`): one save in flight on the runtime's storage worker; a failed version is not retried by itself (only after the next change, a Retry tap, or leaving the app); a library that cannot be opened starts a blank artwork in a new project and leaves the old records untouched, with a notice.
- Reason: principle 5 (pay per change): rewriting a whole artwork after each stroke would cost megabytes per stroke on the tablet's flash; per-stroke records make a stroke cost its own bytes. An owned byte layout cannot change with a dependency upgrade, and the frozen bytes prove it.
- Consequences: schema 2 (S9) adds a migration over these records and its own sample. History and editor state are not stored. Layer reorder and property changes rewrite only the head; deleting a layer removes its records.
- Archived 2026-10-04 (S23): summarised by D49 "Schema history".

## D25 2026-10-02 Schema 2: a catalogue for the gallery, thumbnails as derived records

- Decision: keyspace `library` gains `schema` (the whole library's), `clock` (a change counter) and per project a catalogue entry `['a'][id]`: title and `touched` (the clock at create, duplicate or save; the gallery lists the highest first). `last` now names the artwork open when the app was left and is removed when the user leaves for the gallery, so the app reopens where it was left. Keyspace `projects` gains `[id]['t']`, the thumbnail: RGBA at most 448 x 336, each pixel the rounded mean of its canvas box, computed on the GPU and reproduced exactly by `render::thumbnail_reference`. Head and stroke layouts are unchanged; heads say 2.
- Migration 1 -> 2 (`store/src/migrate.rs`) is pure: from the heads it adds an "Untitled Artwork" entry per project touched in id order, the clock and the schema, and marks the heads 2; the library applies it in one batch when it opens a schema-1 library. `schema-1.bin` stays (it opens upgraded with every old record kept); `schema-2.bin` is the new frozen sample.
- Thumbnails are derived, not artwork: written whenever an artwork is left (for the gallery or by leaving the app), never used to rebuild it; a missing or unreadable one shows "No preview" and is rewritten on the next leave.
- Reason: the gallery needs titles, an order and pictures without reading any stroke; outside the head, a rename or a listing never touches artwork records. Rendering thumbnails when the gallery opens would evaluate every artwork each time.
- Consequences: the storage worker is a FIFO queue (`worker::Queue`, replacing `Latest`): gallery calls must never replace one another. Leaving the canvas waits for its save; if it fails the canvas stays with Retry. After a crash a thumbnail can show the version left last, not the last stroke.
- Archived 2026-10-04 (S23): summarised by D49 "Schema history".

## D28 2026-10-02 Schema 3: every stroke carries its recipe; the toolbox is a separate record

- Decision: a stroke record holds its recipe after the colour (softness, four curves as count and points, grain strength and scale), so reopening never applies a changed brush (D07). `library/editor` holds the toolbox (current key, per brush key, size, opacity); an unreadable one is `Corrupt`, logged, and the brushes start at their defaults; unknown keys or values out of a brush's range fall back per brush.
- Migration 2 -> 3 (`migrate.rs`, pure) inserts the plain round brush's recipe, frozen in `migrate.rs`, into every stroke and marks heads 3; a schema-1 library runs 1 -> 2 -> 3, each step seeing the heads the one before wrote, in one batch. `schema-1.bin` and `schema-2.bin` stay as inputs (they open upgraded with every record kept); `schema-3.bin` is the frozen sample, toolbox included.
- Reason: per-stroke records (D21) keep saves paying per stroke; a recipe costs about 70 to 150 bytes against kilobytes of samples, and `from_storage` shares equal recipes again in memory.
- Consequences: an upgrade reads every stroke record into memory once. Verified on the tablet: the author's schema-2 drawing reopened pixel-identical.
- Archived 2026-10-04 (S23): summarised by D49 "Schema history".

## D29 2026-10-02 Schema 4: Alpha Lock in the head, a blend per stroke, a toolbox of two tools with colours

- Decision: each layer entry of a head gains Alpha Lock (u8) after its lock; each stroke record gains its blend (u8: 0 paint over, 1 paint atop, 2 erase) after its colour, before the recipe; `library/editor` becomes the tool (u8), the brush tool's choice (schema 3's whole record), the eraser's choice (an empty key is the default brush), the colour and up to 8 recent colours.
- Migration 3 -> 4 (`migrate.rs`, pure): heads re-encoded with Alpha Lock off per layer (a head that does not parse whole stays as it is), strokes given blend 0, the toolbox record kept with the brush tool current, the eraser's default choice, black and no recents appended. The library runs every step from the stored schema on, each step seeing the heads and strokes the one before wrote, in one batch. `schema-1/2/3.bin` stay as inputs and open upgraded with every record kept; `schema-4.bin` is the frozen sample (an alpha-locked layer, an atop and an eraser stroke, a toolbox with colours).
- Reason: the blend must be stored per stroke (D30); one byte keeps stroke records paying per stroke; the editor state stays apart from artworks (D09).
- Consequences: verified on the tablet: the author's schema-3 drawing reopened pixel-identical after the upgrade, its toolbox kept.
- Archived 2026-10-04 (S23): summarised by D49 "Schema history".

## D32 2026-10-02 The UI is paper-white line art, intuitive and restrained

- Decision: the chrome is light, near-neutral warm paper with ink-line outlines and self-drawn ink-line icons (no icon library); the white canvas sits on the paper; one deep ink colour marks the active state; no dark theme. Two rules: intuitive (a first-day tablet user needs no instructions; every control explains itself) and restrained (UI appears when needed and recedes; cues over controls, e.g. a cut-off last row and an edge fade instead of a scrollbar, an indicator only while scrolling). Hidden shortcuts such as gestures only duplicate a visible control. Supersedes "neutral dark greys for chrome" in ARCHITECTURE section 10.
- Reason: the author's direction; the dark, engineering look of Photoshop, Procreate and Clip Studio is not wanted. Near-neutral paper keeps colour judgement honest.
- Consequences: S17 applies it to every component through `ui/src/design/`; style screenshots go to the author before the full pass.
- Archived 2026-10-04 (S28): carried by D40 (the chosen style set) and ARCHITECTURE section 10 (the two rules).

## D34 2026-10-02 Gestures follow Procreate's defaults as draweave tuned them

- Decision: pinch zoom and rotate, two-finger pan, quick pinch to fit, two-finger tap undo, three-finger tap redo; dead zones, thresholds and snapping take draweave's tested values. Touch never paints (D18); the pen never navigates.
- Reason: the author's direction; feel lives in the dead zones and thresholds, which draweave already tuned on this kind of tablet.
- Consequences: S14 adds multi-pointer routing and scenario steps; touch-and-hold eyedropper and pen-button mapping remain separate decisions.
- Archived 2026-10-04 (S28): carried by D35 (which narrowed it: no rotation snapping) and D18.

## D31 2026-10-02 Export, share and the reference image: app flows, platform ports, a worker with its own target; the reference is not stored

- Decision: Procreate's Actions panel (a spanner right of Gallery) holds Save PNG/JPEG, Share PNG/JPEG, the Reference switch and Choose reference image. `app::Transfers` owns both flows for the app, not per artwork; the app never sees a path or URI: it asks the host through `HostRequest` and receives `ImageSink`/`ImageSource` ports in `HostAnswer`. Export is `render::Core::export`, a canvas target of its own evaluated whole through `RegionEvaluator` on the `openpaint-transfer` worker (an `Arc<Core>`, nothing the frame loop holds), then encoded and written there; JPEG quality 92 on the paper colour. The Android host serves the save dialog and the picker through the Storage Access Framework (detached descriptors to Rust) and shares through its own read-only `ShareProvider` over `cacheDir/share` (no AndroidX). PNG and JPEG go through the `image` crate in store (D14). The reference image is decoded on the worker, scaled to at most 2048 px, kept for the app's run and never stored: the format stays schema 4.
- Reason: a pixel never depends on which regions are evaluated with it (D16), so a separate target gives the screen's pixels exactly without touching the screen's target or stalling drawing. Ports keep the app and the harness free of Android; devtools plays the platform in scenarios. Storing the reference would add an image asset to the format, which needs the author (ARCHITECTURE section 10) and the asset store.
- Consequences: a new export format is an `ImageFormat` value plus an encoder; layered or PSD export needs its own job. One system dialog at a time (`Availability::Running`). Storing the reference with the artwork is a later format change with its own migration.
- Archived 2026-10-04 (S29): its export rendering (one whole target) is replaced by D55 (bands at a scale, PNG streamed through png); the flows, ports, worker and unstored reference are carried there.
