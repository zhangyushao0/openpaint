# Architecture

OpenPaint is an Android-first, stylus-first 2D painting app whose goal is to take an illustration from sketch to delivery on one tablet. Android is the only product host. On the development machine a headless harness renders offscreen, replays recorded input and runs scenarios; there is no desktop window and no desktop product.

This page is the design contract: how modules are cut, which way dependencies point, the types and signatures crates meet at, how threads and errors are handled, the rules the tools enforce. What each module holds is its own `//!` (a crate's `lib.rs` maps its modules); this page names a file only where a contract lives in it. When code and this page disagree, change the code or change this page and record a decision; never keep both.

## 1. Principles

1. One concept has one definition and one owner, and the owner is a module, not just a crate. Mutable/immutable, intent/command and persistent/session forms are never kept twice.
2. The artwork is an immutable value. An edit produces a new version, history stores versions, undo switches versions (D08).
3. Product crates have no `cfg(feature)`. Diagnostics, experiments and automation probes live in separate crates and are assembled only by composition roots (D10).
4. The renderer depends on core only. GPU state belongs to the renderer's own `Core`, never to the UI framework. One render core, the raster core, draws every scene for every reader (D92, D93); the GPU protocol is defined once in Rust, injected into WGSL and proven by layout tests.
5. Strokes and operation records are the truth; every pixel is a derived, discardable cache.
6. Pay per change: saving, history budgets and projections touch only the data a change involves; the UI thread never does work proportional to artwork size.
7. Errors are types. `String` errors are allowed only in JNI, at system boundaries and in xtask.
8. One user action has one product path: no double writes, no simple/advanced variants, no silent fallbacks, no modules or abstractions reserved for features not yet accepted.
9. Files at most 400 lines, functions at most 60 lines, tool-enforced; `lib.rs` holds only `mod` and `pub use`.

## 2. Crate map

| crate | Responsibility | May depend on | Must not contain |
| --- | --- | --- | --- |
| `openpaint-core` | artwork semantics: layers, strokes, records, commands, validation, brush recipe data | nothing | GPU, UI, file system, threads, clocks |
| `openpaint-practice` | practice as pure math: the kinds of exercise (generators, answers, checks, readings, marks, How), the course (units, skills, the learner, the schedule, the re-tests; D115), each skill's progress fitted (D124), fits and outlines, colour metrics, a painted copy read from its pixels, sheet records and layout, practice time, the book and its seals, the Pomodoro's blocks, the pack model (D98 to D109, D111) | nothing | GPU, UI, file system, threads, clocks, serde |
| `openpaint-raster` | pure pixel geometry on the CPU: the fill's connected region, a transform's lifted pixels | core | GPU, UI, file system, threads |
| `openpaint-app` | application flow: intents, session (a mode owns one, D119), history, gallery, toolbox, transfers, capability ports (traits) | core | GPU, UI, file system |
| `openpaint-practice-app` | practice's flow (D112): the sheet owning its session (D119), its pens, Compare, a result, a lesson's end, readings, views; the log, the path, the Pomodoro, the book; the practice options | core, practice, app | GPU, UI, file system |
| `openpaint-store` | the local library (storage format 2, D125; format 1 converted once, `convert/`, S163's build), OpenPaint files, kept versions, image files, the display cache, record spaces of opaque bytes their owners keep (D112); implements the app ports | core, app | GPU, UI, the word practice |
| `openpaint-practice-store` | practice's sheet records, lessons' givens, the Pomodoro's records and the blind pairs in the library's record spaces `p`, `g`, `t` and `b`, read only in the shape this version writes, cleared by key (`clear`); its packs beside the library (D112) | core, practice, practice-app, app, store | GPU, UI |
| `openpaint-render` | derived rendering: wgpu, the raster core, the canvas, view tiers, exports, picks, pictures | core | file system; `egui` only under `src/adapter/` |
| `openpaint-ui` | shared interface: egui panels and components, input ownership, canvas arbitration, the hooks a mode fills (`hooks.rs`, D112); reads app projections, emits intents | core, app, render | direct wgpu, file system, the word practice |
| `openpaint-practice-ui` | practice's screens behind the ui's hooks (D112): a sheet's marks, bar, tools, card, result, end and How; the home's column, the path, options and book; their words | core, practice, practice-app, app, ui | direct wgpu, file system |
| `openpaint-runtime` | frame loop, input normalisation, workers, lifecycle, startup, the GPU budget, debug benchmarks; composes practice (`practice/`) | core, app, store, practice, practice-app, practice-store, practice-ui, render, ui, raster | JNI |
| `openpaint-devtools` | scenario files, the scenario runner, checkpoint diffs, the switchable store wrapper `Watched` | product crates | product rules; never a dependency of a product crate |
| `openpaint-harness` | development harness: `scenario` (`--all` on parallel workers, each scenario a process of its own), test art, replay, statistics, PNG and hash output, `key-shifts`; never shipped | all, devtools, sheet | windowing, mouse input, product rules |
| `openpaint-sheet` | the harness's and the pack builder's contact sheets | nothing | product code |
| `openpaint-android` | the only product composition root: activity lifecycle, Surface, system pickers, the replay's video encoder (`video.rs` over JNI, `VideoEncoder.kt`: MediaCodec H.264 through an EGL surface, MediaMuxer; minSdk 29), JNI | all | product rules |
| `xtask` | repository tooling: `cargo gate` (test binaries built once, run twelve at a time longest first, hung ones killed, superseded builds removed), `cargo repo-check`, the Android build and version, device chores over adb, shape textures | nothing | product code |
| `tools/packs` | a workspace of its own outside the gate, built in its `target/`: `cargo practice-pack build|search`, a pack from a list (`lists/`), downloads cached in `artifacts/packs/cache/`; U²-Net via tract-onnx (D106), value masses (D108), crops from full pixels (D105), contact sheets | practice, practice-store, app, store, sheet, by path | product code; a second implementation of the app's computations |

Dependencies only point down (`A <- B` means B depends on A):

```text
core <- app <- store
core <- app <- ui <- runtime <- {harness, android}
core <- render <- ui
core <- raster <- runtime
practice, app <- practice-app <- {practice-store, practice-ui, runtime}; ui, practice <- practice-ui <- runtime; practice <- runtime
practice, store <- practice-store <- {runtime, devtools, harness, android}
store, render <- runtime
runtime <- devtools <- harness
sheet <- {harness, tools/packs}; practice, practice-store, app, store <- tools/packs
```

A new crate is registered in `LAYERS` in `xtask/src/check/rules.rs` first (D02). The app's version is `[workspace.package] version` in the root manifest; `xtask/src/version.rs` hands it to Gradle with the store's `FORMAT` as build metadata and a version code, and the runtime hands the same two to the shell (`Shell::set_version`), which shows them as Actions' last line in the language shown (D79).

## 3. Core types

### core

```rust
pub struct Artwork(Arc<ArtworkState>);           // a value: clone is O(1), history stores versions
struct ArtworkState { canvas: CanvasSize, background, layers: Vec<Layer>, groups: Vec<LayerGroup>,
                      active: LayerId, on_mask: bool, ids: IdCounters }
pub struct Layer { id, name, props: LayerProps, strokes: StrokeList, edits: Arc<[RegionEdit]>,
                   mask: Option<LayerMask> }
pub struct LayerProps { visible, opacity, locked, alpha_locked, clipping, blend: BlendMode, group }
pub struct StrokeList { chunks: Arc<[Arc<[Stroke]>]> }   // appending copies only the last chunk
pub struct Stroke { id, style: StrokeStyle, blend: StrokeBlend, samples: Arc<[StrokeSample]>, clip }
pub struct StrokeSample { position, pressure, tilt, azimuth, time_ms }
pub struct StrokeStyle { tool: StrokeTool, recipe: Arc<BrushRecipe>, size, opacity, color, seed }
pub enum RegionEdit { Fill(FillEdit), Clear(ClearEdit), Transform(TransformEdit),
                      Adjust(AdjustEdit), Nested(NestedEdit) }
pub enum ArtworkCommand { Layer(LayerCommand), Group(GroupCommand), Mask(MaskCommand), .. }
pub struct Applied { pub artwork: Artwork, pub change: Change }   // Applied::content for content edits
```

No public mutable setters. Layers come only from `from_storage` (the single full-validation entry, `ArtworkParts`) or commands; `to_parts` gives plain data for the store. A command that changes nothing returns `Change::None` and the same version. Ids are never reused; limits (`MAX_LAYERS`, `MAX_STROKE_SAMPLES`, `MAX_CANVAS_SIDE`, `MAX_NESTING`, `MAX_STAMPS`) are public constants. The stroke being drawn (`StrokeSession`) lies beside the artwork, never in a layer. A stroke keeps when it began (UTC ms, `began_ms`), an edit its place among the strokes (`before_stroke`, D125); `replay/` turns an artwork into its `DrawingOrder` (every list's strokes and edits as `Step`s in the order they were made) and a moment of it into a `ReplayFrame` (every list cut to a prefix, the stroke being drawn an `ActiveStroke`), consecutive frames sharing their `Arc`s.

### practice

```rust
pub enum Kind { Line(LineTier), Colour(Tone), Combined(Combination), Shape(Tier),
                Scale(ScaleTone, u8), Study, Box(BoxView), FreeBoxes(Horizon),
                Ellipse(Guide), Flat(Estimate), Space(Trick), Solid(Solid),
                Object(Carve), Division(Tier), WholeCopy(Tier), ValueMasses(Massing), Crop(Tier),
                KeyShift(Tendency), Free, Analysis(Tier), Rotation(Rotation), Observation,
                GamutMask(Mask), Placement(Placement), Light(Lit), Shadows(Throw),
                CurvedRotation(CurvedRotation), LitColour(LitColour), Texture(Texture),
                Thumbnail(Emphasis), ColdWarm(Leans), Arrow(Arrow), Contour(Bend),
                Intersection(Meeting), Ghosted(Ghosted) }
                // (code 1..=36 but 5, variant); Massing { Three(Tier), Two(Tier) } (Notan, S156)
pub struct Skill(u8);  pub enum SkillLevel { First, Second, Third }  // a kind's folder (D115)
pub struct Exercise { kind: Kind, seed: u64, picture: Option<ItemRef> }  // original() -> Arc<dyn Original>
trait Described { code, facts, variant, topic(Kind), generate(&Exercise), how(Kind, cells), lesson, window, glyph }
pub trait Original { givens, printed(picture), check, compare, painted, answer, lines, swatch, picture, areas, scale, colour_at }
pub struct Facts { key, aligned, compared, constructed, tools: ToolSet, picks: Picking, needs, kept, counted, grounded, under, hidden, shape, beside, seconds }
impl FlatCheck { fn of(&Flat, lines) }  impl SpaceCheck { fn of(&SpaceTask, lines) }  // metrics
impl ObjectTask { fn generate(Carve, seed); fn picture(side) -> Vec<u8>; fn surface(Point) }  // answer, stages
impl DivisionCheck { fn of(&[(Level, Vec<Point>)], side) }  // lines joined, rows, metrics (S95)
impl Masses { fn generate(seed, aspect); fn picture(width); fn known(Point) }  // division, turning, drawing
impl ValueMasses { fn of(size, rgba); fn edges(); fn read((size, rgba), place) -> MassesCheck }  // S96
impl SpaceTask { fn at(face [u, v]) -> Point; fn on_face(Point) -> [u, v]; fn construction() }
pub struct Frame { origin: Point, size: Point }          // frame units isotropic, one to its width
pub struct Sheet { id, canvas, started, purpose: Purpose /* Day | Copy | Test */, spans: Vec<Span>, lessons: Vec<Lesson> }  // Sheet::begun(id, .., Planned)
pub struct Lesson { started, finished, picked: Picked /* Chosen | First | Retest { unit, end } */, at: u16, exercises: Vec<Laid> }
pub struct Laid { exercise: Exercise, level, slot, frame: [u16; 2], first: Option<First>, compares, result: Option<(Outcome, Judge)>, seconds }
pub fn practised(spans: &mut Vec<Span>, now: u64, hidden: bool) -> bool;  // PAUSE 300 s
impl Book { fn of(records, offset) }  impl Tally { fn of(records); fn seals() -> Vec<Seal> }
pub struct First { at, corrected_from: u64, metrics }  pub struct Givens { slots: Vec<(u16, Given)> }  // Span { from, to }: UTC seconds
impl Learner { fn of(sheets); fn with(&Lesson); fn dots(Skill); fn level(Skill); fn open(unit, &Pictures) }  // derived
impl Progress { fn of(sheets, &[BlindPair], offset) }  impl SkillFit { fn curve(); fn change() }  // WHR, derived (D124)
pub fn ask(&[Skill], (sheets, pairs), (today, offset), &Within) -> Option<PairAsked>;  // the least sure pair; block_pairs(since, ..) a block's end's (S148)
pub fn align(original: &[&[Point]], copy: &[&[Point]]) -> Option<Fit>;  // Fit { scale, shift }
pub fn align_shape(outline: &[&[Point]], copy: &[&[Point]]) -> Option<Fit>;
pub fn outline(size: [u32; 2], rgba: &[u8]) -> Result<Outline, OutlineRefusal>;
pub fn outline_of_matte(size: [u32; 2], rgba: &[u8], matte: &Matte) -> Result<Outline, OutlineRefusal>; // the builder's (D106)
pub fn score_colour(Rgb8, &ColourCopy) -> Option<ColourScore>;
impl Areas { fn found(size, rgba) -> Found; fn read(size, rgba, paper) -> AreasCopy; fn edges(area, other) }
pub fn candidates(size, rgba, Places) -> Vec<Crop>;  pub fn best(Vec<Crop>, count) -> Vec<Crop>;
impl ValueScale { fn read(size, rgba, paper) -> ScaleCheck }
pub fn edges(lines: &[&[Point]]) -> Vec<Edge>;  pub fn corrected(earlier, corrections) -> Vec<Edge>;
// Boxes, free boxes, ellipses, solids, objects, rotations, arrows, contours, intersections, ghosted
// lines: no check, the answer laid for the eye (D123)
enum Light { Sun(Vector), Lamp(Vector) }  // geometry/light.rs (S153): shadows on a plane, a pin's sun,
                                          // vanishing points; the objects' Body casts rays and holds
                                          // balls and capsules (a contour's tube, S156k)
impl SpacePath { fn through(&[Vector]); fn at(s); fn samples(n, up, &[Turn]) -> Vec<Sample> }  // geometry/space_path/ (S155):
                                          // centripetal Catmull-Rom, even by length, rotation-minimising frames
```

Pure, no dependency (D98, D99, D103, D105, D107, D108, D111). Only `lib.rs` lies at the root, re-exporting what other crates use; the rest lies in folders, each `mod.rs` saying what it holds: the shared ones, which never reach into a kind, and `kinds/` with a folder a kind holding its generator, answer, check, reading, marks, How (`demo.rs`) and tile, and in its `mod.rs` its description in two halves (D111): `Described` and the original it generates, its own type behind `Original`. One line in `KINDS` registers it; only a folder matches on `Kind`'s variants; a kind builds on another only where the course does (objects on solids, combined lines on lines). Metric codes, outcome codes and kind codes are stored and never reused; the reader reads only what this version writes (D112; `records/metric.rs`, `kinds/kind.rs`).

### app

```rust
pub struct Project { id, title, artwork: Artwork, saved: SaveId, thumbnail }   // the store boundary type
pub enum Intent { Stroke, Artwork, Layer, Tool, Brush, View, Selection, Fill, Transform, Adjust,
                  Insert, Export, Reference, Clipboard, Gallery, Storage, History, .. }
pub enum AppError { Core(CoreError), Unavailable { action, availability }, FillRefused(FillRefusal),
                    TransformRefused(TransformRefusal), AdjustRefused(LayerRefusal) }
pub trait ProjectStore { /* open_last, list, create, load, save, thumbnails, rename, duplicate,
                           delete, editor, files, versions, record spaces (D112) */ }
pub trait ImageSink; pub trait ImageSource;                 // the platform's files (transfer.rs)
impl Session { pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError>;
               pub fn projection(&self) -> SessionProjection;
               // a mode sets and steps (session/mode.rs, D119): keep_raw, set_bounds, set_brush, view_mut,
               // amend, record, keep_only_current, make_way(&Intent), make_way_for_step
}
```

The UI reads projections, keeps UI-local state and emits intents; availability and value ranges come from the projection (`Availability`, `EditorAction`), so the UI never assembles rules itself. Each intent family is one module holding its intents, handlers, projection and tests (the map: `app/src/lib.rs`). What an intent ends before it applies is one table (`session/drafts.rs`). History costs versions by `bytes_since` within 64 MiB (D20); a mode may keep only the current version (D105). The toolbox is app-wide and owned by the runtime (D70). The gallery's `LibraryRequest`/`LibraryAnswer` serve the storage worker and its `FileJob`s run one at a time, a mode's picker taking the same turn (`Gallery::choose(request, tag)`, answered as `FileJob::Mode`). Transfers: one system dialog at a time, `HostRequest`/`HostAnswer`, `TransferJob`s for the transfer worker, `BytesSource` the one in-memory `ImageSource`. Input: `InputBatch`, `PalmRejection`, the finger recogniser `gestures/`, `CanvasView` and `ViewMapping` (screen to canvas) in `view/`.

The replay (`replay/`, S163): `Replay` lies beside the session while it is open (`Intent::Replay(Open)` goes through the session, the rest are `ReplayIntent`s the composition root hands it): the pure `Timeline` (pen time, pauses cut to 300 ms, sittings), the flow, its `ReplayProjection` (`SessionProjection::with_replay`), and the video: `HostRequest::Video` answered at once with a `VideoEncoder` (the platform's port: `begin`, `frame`, `finish`, `cancel`, all from the video worker's thread), which makes a `VideoJob` of the `VideoFrames`; the session stamps strokes on the UTC clock the runtime sets each frame (`Session::set_utc_clock`).

### practice-app

```rust
pub struct SheetSession { session: Session, sheet: PracticeSheet, brush }   // D119
impl SheetSession { fn open(artwork, (brush, pens), Practising, &Viewport); fn dispatch(Intent);
    fn practise(PracticeIntent, now) -> Result<(), Refused>; fn pen(); fn projection(); fn session(); fn sheet_mut() }
impl Options { fn dispatch(OptionsIntent, &mut Gallery); fn request(); fn chosen(tag, source) -> Option<OptionsFile> }
```

The flow asks a kind's `Facts` and its original's methods, never which kind it is (D111). The sheet's and the options' intents (`PracticeIntent`, `OptionsIntent`, `TimerIntent`) travel apart from app's `Intent` and apply after the frame's painting intents. The screens read `PracticeProjection`, `SheetView`, `PathProjection` and `TimerShown`. The Pomodoro's timer is the practice log's, on a clock the runtime hands in each frame (`PracticeLog::tick`); its states and rules are `pomodoro/mod.rs`'s `//!` (D117).

### render

```rust
pub struct Core { gpu, textures /* paper and shapes */, pace: Mutex<Pace>, raster pipelines, frames, .. }
pub struct FrameRequest<'a> { artwork, active, background, .. }
impl CanvasTarget { pub fn draw(..); pub fn draw_frame(..); pub fn whole(..) -> RgbaImage; }
pub enum RenderError { Admission { what, limit }, Cancelled, MissingImage, Device(String),
                       Invalid(&'static str), Timeout { what, seconds } }   // every GPU wait ends within 30 s
```

One render core, the raster core (D92, D93): every scene is drawn by the GPU's rasterizer as instanced quads computing each pixel's coverage into a float atlas, laid onto float layers by their paint blend, layers ending by fixed-function blending or from a copied backdrop. `protocol/` owns `Op`: its operands, named slots and the WGSL prelude generated from them, checked by a naga layout test. The module map: `render/src/lib.rs`.

`Core::renderer_identity` (the display cache's key) is FNV-1a of the version, the display format and the raster core's, smudge and filter WGSL. Background consumers (export, fill, pick, pictures) hold only an `Arc<Core>` and never touch a UI lock. Incremental and cold drawing are bit-identical, proven by GPU tests (`render/tests/stroke/`). `video/` (S163): a `VideoTarget`, a canvas target of the video worker's own outside the GPU budget, draws the replay's frames one after another, scales them to the video's size by area in integers (`video_reference` its CPU twin) and reads them back as RGBA8; its `card` fits the last picture on the desk above the caption's band.

### runtime

```rust
pub struct Runtime;                                    // the frame loop, owned by the host
pub struct StartOptions { core, cache, memory, language, packs, blank } // Runtime::start(ppp, store, options)
pub(crate) struct Queue<Req, Out>;                     // worker.rs: FIFO, the only place a thread starts
pub fn motion(&mut self, &MotionBatch) -> bool;        // input, never touching the GPU
pub fn frame(&mut self) -> Result<bool, RuntimeError>; // one frame, or a call skipped for the GPU
```

`Runtime::frame` (`frame/`) has fixed steps: receive the workers' answers -> normalised input through palm rejection -> the UI pass (in `Runtime::language`: the toolbox's, else the host's `StartOptions::language`; what it lays over the canvas laid through the view this frame's contacts move it to, `Shell::foreseen`, S161) -> route intents to the canvas or gallery -> the budget's step -> raster answers -> draw the screen -> hand work to the workers. Calls come from one thread (the Android main thread); frames are paced by `pacing.rs` (D97), background work while idle by `ahead.rs` (D96).

Workers, each a `worker::Queue` on its own named thread: `openpaint-store` (the library, saves, leaves, files, versions, practice's jobs; opening the last artwork, else the home, D116), `openpaint-transfer` (exports, decoding pictures), `openpaint-fill`, `openpaint-compare` (Compare's aligned fit and a painted copy's frame drawn and read, off the frame loop; D103, D105), `openpaint-pictures` (layer pictures while the panel shows), `openpaint-pack-pictures` (the practice options' thumbnails, 256 px), `openpaint-brushes` (brush rows and the editor's strip), `openpaint-cache` (the display cache, D96), `openpaint-raster` (tiers' and canvas tiles, snapshots ahead), `openpaint-video` (the replay's video, `video.rs`: every frame drawn by a `VideoTarget`, the end held as the finished picture or the card, handed to the platform's encoder at `k × 1e6 / fps` µs, progress and Cancel in atomics the frame loop reads). The module map: `runtime/src/lib.rs`.

The open canvas holds `Work::Artwork | Work::Sheet` (D119). Practice is composed in `practice/` (D112): `practice::sheet` is the one way to the open sheet, every lesson starts through `lesson_job`, practice's part of the screen fills the shell's hooks, a frame's input on a sheet is a use of its time and `leave` hides it and holds the timer (D109). A library of another format starts the app on the home with `GalleryNotice::OtherLibrary` (D95). The Android host keeps its logs in `apps/android/src/` (`latency.rs`, `pacing.rs`, `navigation.rs`), hands the system's locale to `Native.create` (`Language::of_tag`); after every frame (`MainActivity.drawn`) `TimerHost.kt` (`jni_timer.rs`) posts `wake_in`'s frame, keeps the screen on, plays the chime; `FrameRate.kt` asks the top rate while drawing (D79). The replay (`canvas/replay.rs`, `frame/replay.rs`): while it is open the canvas draws its frame at fit (its stroke being drawn only once no tile waits), every other intent dropped, never on a practice sheet; the shell's caption (`openpaint_ui::video_caption`, an egui context of its own) goes to the video worker, which paints it.

## 4. Input

Hosts deliver platform events as `runtime::MotionBatch` (phase, buttons, pointers with tool kind and batched samples: position, pressure, tilt, orientation, time); the Android host decodes `MotionEvent` arrays into it in `apps/android/src/motion.rs`. The Android host and the harness feed one normaliser (`runtime/src/normalise.rs`): `MotionBatch` -> `app::InputBatch` (per contact: id, device, phase, samples; ranges clamped, times monotonic per contact). egui receives its pointer events from these batches; no windowing library sits in between.
Palm rejection (`app::PalmRejection`) applies to every batch before egui or the canvas sees it: while a pen contact is down, touches are palms until they lift; a pen landing cancels the touches already down. The UI's canvas input (`ui/src/canvas_input.rs`) owns each pen contact that touches down on open canvas and hands it to the session as `Intent::Stroke` mapped through the view into canvas pixels (any contact, picking, while the eyedropper is on); it follows the hovering pen for the brush outline.
Finger contacts go to `app::Gestures`, which decides in points what they mean (navigation, quick pinch, taps, the three-finger swipe). Contacts on the reference window, a ColorDrop drag, a snapped shape's handles, the guide's centre, the transform frame and the brush strip are routed from the raw batches before the canvas. Platform cancels arrive as `Cancel`; egui gets a cancel as a release far off screen. The stroke intent drops Android's zero-pressure lift sample. The pen's side buttons are read from every pen event's button state into `InputBatch::pen` and handed to `app::ButtonTracker` before and after each batch's contacts (D70). StreamLine lives in the stroke intent; the stored samples are the smoothed path. Keyboard text reaches egui as IME and key events through `Runtime::text`; the host shows the soft keyboard while `keyboard_wanted` (the host is the text editor, no `EditText`); the back button goes to `Runtime::back`.

## 5. Persistence

- The store owns the format: one storage format, `openpaint_store::FORMAT` 2 (D125), for the library, OpenPaint files and kept versions. S163's build converts a format-1 library at its first opening in one batch, or a format-1 file on import (`convert/`, deleted by S164); a library or file of another format is refused as `PortError::OtherFormat` before anything is written.
- The library (D77): `Library::at(dir)` implements `ProjectStore` on fjall, keyspaces `library` and `projects`; Android keeps it in `getFilesDir()/library`, the harness in `artifacts/`. Every call that writes is one atomic batch; every save writes a new random save id into the head. No upgrade path (S116). The module map: `store/src/lib.rs`.
- Record spaces (`spaces.rs`, D112): a mode's records in `library` under keys of its byte (`app::Space`; the store's own initials `a c e f l n v` refused), bytes it never reads: `records` (under a prefix), `put_records`, `create_owned` (a project, the open one, and its records, one batch), `remove_owned`, `artwork` (read only); `list` lists every project.
- Files and versions (D62, D63): OpenPaint files are read over the same `read::artwork` as the library (a `Source` both implement); a kept version is a file's bytes with a dated entry.
- Practice (`openpaint-practice-store`, D98, D99, D112): a sheet's record under `['p'][sheet u64]` (tag 0x53, `records.rs`) and a lesson's givens under `['g'][canvas u64][lesson u16]` (`givens/`), read only in this shape, every field required, an unknown tag, kind, metric or outcome refused; `stored.rs`: `sheets` (one unreadable left out, logged), `start` (project, sheet and givens in one batch), `append` (sheet and its new givens, one batch), `save`, `remove` (record, givens and project at once), `trim` (End's cut sheet with a removed lesson's givens, one write, `ProjectStore::rewrite_records`); `blocks.rs` the Pomodoro's records in space `t` (D117); `pairs.rs` the blind pairs in space `b` (D124); `clear.rs` the practice-only clear (S152): every record of `p`, `g`, `b`, the blocks of `t` and every project a sheet's head (`named_project`) or a givens' key names, by key, never decoding, one `remove_owned`. A project (a colour copy, a drawing from life) is a sheet of purpose Copy, its name its project's title (D105); the runtime's `gallery_list` leaves the sheets out and gives their titles.
- Packs (D101, D103, in practice-store): `pack.rs` the file (`PACK_FORMAT` 4; a pack of another format is `OtherFormat`, refused on import and left out of the list; a non-pack file `Corrupt { what: "practice pack" }`), `packs.rs` (`Packs::at(dir)`: `<dir>/<id>.pack` beside the library, import checked whole and renamed into place, never the personal pack).
- The display cache (`display_cache.rs`, D96): each artwork's shown canvas as one LZ4 block with a CRC-32 outside the library, keyed by the save id; derived, never read by files, versions or exports.
- Images (D53, D55): decoding with colour conversion into sRGB; `ImageEncoder` takes bands of rows (PNG streamed, JPEG at the end) and embeds the sRGB profile.
- Editor state (the toolbox) is stored apart from the artworks; when it cannot be decoded it falls back to defaults and never blocks opening an artwork. Autosave runs on the storage worker when a stroke finishes and when the canvas is left; failure shows a notice with Retry.
- Tests: a frozen sample, `store/tests/samples/format-2.bin` (every record kind; `format-1.bin` converting to it until S164), practice's records through the record spaces beside it (`practice-store/tests/clear.rs`); the store's unit tests fail its n-th write through a test-only hook (leaving, a save cut at every byte, a project created or removed with its record).

## 6. Brushes and rendering

- A brush is a declarative recipe (TOML in `crates/app/brushes/`; the practice pens' own in `crates/practice-app/pens/`, their hard tip Hard Paint's file): softness, size and opacity curves over pressure, speed and tilt, grain, deposit, taper, shape (D74). Rendering draws per pixel along the curve through the stroke's knots (D71), its stamps too; there is no node graph or shader plugin. A stroke stores its recipe's snapshot and its seed; reopening never applies a brush's later edits.
- The deposit (D38): covering strokes keep the largest coverage; accumulating ones lay density (the Pencil on canvas-fixed sketch paper, the Soft Brush under a bell). Shapes: stamps belong to the span whose length range holds them, textures from `cargo shape-assets` (`xtask/src/shapes.rs`).
- The smudge reads its neighbours, so it is drawn once per stroke into a residual (D46); Gaussian Blur and Sharpen likewise into arena blocks (D59). Everything else is drawn per pixel by the raster core.
- Exports draw at their scale (D55), the 1x export is the canvas pixel for pixel, a tier tile is its export (D93).

## 7. Errors

`CoreError` (artwork semantics), `AppError` (modes and flows, refusals such as `LayerRefusal`), `PortError` (external capabilities), `RenderError`, `RuntimeError`. The platform layer translates them into user-facing messages; `String` appears only in JNI, system APIs and xtask.

Storage failures are contained per call and per artwork (D77): a failed write leaves the library as before the call (the save's notice with Retry; a failed kept version is only logged), and a record that cannot be read fails only that artwork's opening ("Couldn't open this artwork"), never the library. A library or file of another format is `PortError::OtherFormat` (D95).

A lost GPU device is not an error the user sees (D96): wgpu's lost callback, out of memory or a driver failure mark the `Core` lost, and a frame that finds it so, or fails with `RenderError::Device` or `Timeout`, makes the renderer again on a new device (`runtime/src/recover.rs`) before it goes on; a job running on a worker at the loss fails as any GPU failure does; app state is never touched.

## 8. Testing

| Tier | Content | When |
| --- | --- | --- |
| unit | pure tests for core, app, store; GPU readback tests for render and runtime (formulas, incremental = cold, invariants) | `cargo test -p` |
| gate | fmt, clippy (also for `aarch64-linux-android` on the JNI crate), all tests, the `smoke` scenario, repo-check | before the commit |
| CI | the gate on GitHub's Linux runner, the GPU tests on lavapipe (`OPENPAINT_SOFTWARE_GPU`): what holds only on a hardware GPU (drawings to the bit across paths, golden hashes, frames timed) left out and named, `xtask/src/test_run/software.rs`; the APK; a release on a tag `v<version>` (`.github/workflows/ci.yml`, S162) | every push to main and pull request |
| regression | every harness scenario (`cargo scenarios`): JSON steps plus canvas hash checkpoints | slices that touch behaviour |
| device | `cargo device-smoke` (at most 20 s), benches and manual checks | slices marked in ROADMAP, stage exits |

Test fixtures live in each crate's `tests/` or in devtools, never in the product API. Each GPU test binary opens one device. The runtime's GPU tests are one binary (`runtime/tests/runtime/`, shared steps in `support/`: device, hand, frames, launch, screen, store); the four that change or count their device (`budget`, `dense_open`, `device_loss`, `display_cache`) are binaries of their own taking `support`'s files by path. Tests drive time by the runtime's held clock (`fix_clock`, the `Hand`'s): motion, input, idle and recency follow it, never the wall clock, which only guards against a hang (two minutes). The pure `openpaint-practice` is optimised in dev builds too.

Scenarios (D19, D48): `tests/scenarios/<name>.json` holds a walk-through, the screen, the host's language, the start and the steps that replay it (the step vocabulary: `devtools/src/lib.rs`). A spot is a control's name, a place relative to one, or window pixels on the canvas only; pixels on a control fail the run with the name to write. `openpaint_devtools::run` turns gestures into `MotionBatch`es and drives `Runtime` offscreen on a library directory and a packs directory beside it; the runner holds the clock (batch times, then 16 ms a frame), lets motion settle before steps that look, fixes every stroke's seed and opens one device with an 8-bit screen. The runner lends the replay an encoder at once (`devtools/src/video/`), letting 30 video frames through a frame of its own, and its `video` step checks the last video's calls, frames and size, its frames written as screenshots. `cargo scenario <name>` writes PNGs and failure diffs under `artifacts/scenarios/<name>/`; the gate runs `smoke`. Device chores are `cargo device-*` (`xtask/src/device.rs`: adb with explicit arguments, never a host shell; `DeviceError`).

## 9. Enforced rules

`cargo repo-check` (`xtask/src/check/`, over tracked and new unignored files): .rs files at most 400 lines, a folder at most 64 of them (`folders.rs`); the workspace's `#[path]` siblings held at their count, which may only go down (`paths.rs`: a module past its lines is split into a folder by ownership); dependency direction per section 2; purity tokens per crate (core never mentions serde; devtools never touches `std::fs`; core, raster, render, app, the store and the ui never name practice, comments included, D112); egui in render only under `adapter/`; no cargo features in product crates; crate roots carry `#![forbid(unsafe_code)]` (android and the harness's allocator excepted); markdown only the four documents (within their lines and characters, 1,500 a line: `lengths.rs`), README, AGENTS.md and the archive; no script files; no CRLF; `tools/packs` with the root's lints, its rustfmt and clippy run (`tools.rs`); under `crates/ui/src/` and `crates/practice-ui/src/` outside each one's `text/` and tests no literal reading as words (a capital, then a small letter) and none handed to `Words::made` or `named` (`words.rs`, D100); outside `crates/practice/src/kinds/` no match on `Kind`'s variants in product code, nowhere an enum `Family`, `Original`, `Tools` or `Picks` or its variant (`kinds.rs`, D111).

clippy: functions at most 60 lines, no `todo!` / `unimplemented!` / `dbg!`, warnings are errors; a justified exception is `#[expect(lint, reason = "...")]`, never `#[allow]`.

## 10. UI conventions

The UI is designed from the user inward (AGENTS.md, "Design from the user inward"). These are the fixed defaults. The screens' map: `ui/src/lib.rs`, `ui/src/home/mod.rs`, `practice-ui/src/home/mod.rs`, `practice-ui/src/sheet/mod.rs`.

- The default interaction model is Procreate's. Before building a feature, read its Procreate Handbook page (entry point, gesture, feedback) and follow it. Deviate only for Android tablet hardware or on the author's instruction. Never copy icons, colours or assets.
- Layout skeleton (D40): the canvas shows on the whole screen; the controls float over it as ink-outlined `island`s and nothing spans the width: Home, Actions, Adjustments, Selection and Transform at the top left; Brush, Smudge, Eraser, Layers and the colour at the top right; the side bar (size and opacity sliders, the eyedropper, undo and redo) at the left edge. The canvas fits under the 48-point top band (`CanvasArea { screen, fit }`). Panels are popovers over the canvas, one at a time, and close with one tap outside. A modal appears only before a destructive action. What comes and goes is a `pill` in the top band: notices left of the right island; the fill, selection, transform, shape and copy bars and the view's zoom in its middle.
- Components (`ui/src/components/`, the one for each job listed in its `mod.rs`): screens compose them and never style widgets inline; every shared component registers its name and rectangle in the `ControlMap` (`components/controls.rs`).
- The shell (`ui/src/shell/`) lays out each screen; while the replay is open it lays the replay's (`ui/src/replay/`: Back and Export video… in the top band, the bar along the bottom, the export panel, the notice pills) instead of the studio's, the canvas at fit between them (`canvas_area`, `displayed`), the pen's tip a ring (`cursor`).
- A mode's screens come in through the ui's hooks (`ui/src/hooks.rs`: `CanvasHook`, `HomeHook`), each called exactly where its part of the screen is drawn; a mode's taps are its own, applied after the frame's painting intents (D112, D116).
- Words (D100): every word shown is a `Text` of a table, the ui's in `ui/src/text/`, practice's in `practice-ui/src/text/` (a kind's in `kinds/`, found by key); English and Chinese side by side, functions for words with numbers; names the app and a mode give show in the language (`Shell::set_names`). Components take `Words` and show them in the pass's language; the control map gets the English. A card's Chinese paragraph is line-broken by the UI (`card.rs`).
- The theme is defined once in `ui/src/design/` (D40; practice's never-chrome colours in `practice-ui/src/palette.rs`): paper, desk, ink, the deeper active ink, one outline weight, self-drawn ink-line icons in one 1.5-point weight behind a semantic `Icon`, an 8-point spacing scale, one type scale (18-point body); the press wash fills exactly a control's outline, never the 48-point hit area. One ring rule: the ink ring means chosen, for words as for icons; at rest nothing the hand taps is framed (a bare word in ink is a command, quieter grey is a hint), never a frame inside a frame.
- Every feature ships with a discoverable entry, visible feedback for success, failure and cancel, undo for anything that changes the artwork, and state that survives reopen. Missing any of these means the slice is not done.
- Checklist that harness screenshots and scenarios must show: the most frequent actions (brush, size, opacity, colour, undo, layer) within two taps; hit targets at least 48 points at tablet density; nothing covers the canvas while a stroke is possible; feedback within one frame; the same component for the same job everywhere.
- Motion (D44): motion only explains (where something comes from or goes, what changed, that input arrived); never on the drawing path (strokes, navigation, slider drags follow the contact 1:1); interruptible, retargets instead of queueing; critically damped; about 0.4 s to open, faster to close; elapsed time on the runtime's frame clock. Four patterns, built into the shared components: emerge / retract (panels from their island button sharing one SDF ink outline with the island, dialogs, notices, the loupe, the reference window, a card into the canvas); fade through (switching panels); settle (layer rows, a dragged row's neighbours, quick pinch to fit, a `choices` group's mark sliding to the item chosen, never across groups); acknowledge (a toggle's ring, switches, swatches). No motion for canvas content changes, the ColorDrop dot, slider drags or the hover outline. Taps always reach the final layout at once.
- Lessons carried over from draweave: controls have three states (available; unavailable, greyed; temporarily blocked, normal look, refused at the intent: an active stroke only ever blocks temporarily); any control that takes a contact resets the gesture trackers; gestures read raw touch geometry including release positions; a tap outside a panel closes it and consumes only that contact; the tap that opens a panel never dismisses it; panels never shrink the canvas; sliders say what they set, preview while dragging and belong to one contact; the pen's hover shows the brush outline at full size, hidden over controls and during multi-touch; the eyedropper commits the last valid preview; notices are brief, success quiet, failure persistent; every icon-only button has an accessible name and a hover label.
- Who decides: details are the agent's. The author judges on the tablet at stage exits. Ask the author in advance only for a deviation from Procreate's mental model, for anything the hand touches continuously (side bar position, gesture mapping, pen buttons) or for anything that changes the persistent format.
