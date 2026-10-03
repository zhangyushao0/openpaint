# Architecture

OpenPaint is an Android-first, stylus-first 2D painting app whose goal is to take an illustration from sketch to delivery on one tablet. Android is the only product host. On the development machine a headless harness renders offscreen, replays recorded input and runs scenarios; there is no desktop window and no desktop product.

This page is the design contract: how modules are cut, which way dependencies point, what the core types look like, how threads and errors are handled. When code and this page disagree, change the code or change this page and record a decision; never keep both. Type sketches land slice by slice; once landed, code is the truth and this page follows.

## 1. Principles

1. One concept has one definition and one owner, and the owner is a module, not just a crate. Mutable/immutable, intent/command and persistent/session forms are never kept twice.
2. The artwork is an immutable value. An edit produces a new version, history stores versions, undo switches versions.
3. Product crates have no `cfg(feature)`. Diagnostics, experiments and automation probes live in separate crates and are assembled only by composition roots.
4. The renderer depends on core only. GPU state belongs to the renderer's own `Core`, never to the UI framework; "evaluate a batch of regions into a target" has exactly one entry point; the GPU protocol is defined once in Rust, injected into WGSL and proven by layout tests.
5. Pay per change: saving, history budgets, asset reclamation and projections touch only the data this change involves; the UI thread never does work proportional to artwork size.
6. Errors are types. `String` errors are allowed only in JNI, at system boundaries and in xtask output.
7. One user action has one product path: no double writes, no simple/advanced variants, no silent fallbacks, no modules or abstractions reserved for features not yet accepted.
8. Files at most 400 lines, functions at most 60 lines, tool-enforced; `lib.rs` holds only `mod` and `pub use`.

## 2. Crate map

| crate | Responsibility | May depend on | Must not contain |
| --- | --- | --- | --- |
| `openpaint-core` | artwork semantics: layers, strokes, commands, validation, brush recipe data | nothing | GPU, UI, file system, threads, clocks |
| `openpaint-app` | application flow: intents, session, history, workspace, editor state, capability ports (traits) | core | GPU, UI, file system |
| `openpaint-store` | local artwork library: persistent format, schema migrations, assets; implements the app ports | core, app | GPU, UI |
| `openpaint-render` | derived rendering: wgpu pipelines, stroke programs, region evaluation; export, pick and preview consumers | core | file system; `egui` only under `src/adapter/` |
| `openpaint-ui` | shared interface: egui panels, input ownership, canvas arbitration; reads app projections, emits intents | core, app, render | direct wgpu, file system |
| `openpaint-runtime` | frame loop, input normalisation, bounded workers, lifecycle draining, startup | core, app, store, render, ui | JNI |
| `openpaint-devtools` | diagnostics and automation: scenario files, the scenario runner, checkpoint diffs; later observers and statistics | product crates (uses runtime, render) | file system, product rules; never a dependency of a product crate |
| `openpaint-harness` | development harness: offscreen rendering, recorded-input replay, `scenario` command, PNG and hash output; never shipped | all, devtools | windowing, mouse input, product rules |
| `openpaint-android` | the only product composition root: activity lifecycle, Surface, system pickers, JNI | all | product rules |
| `xtask` | repository tooling: `cargo gate`, `cargo repo-check` | nothing | product code |

Dependencies only point down (`A <- B` means B depends on A):

```text
core <- app <- store
core <- app <- ui <- runtime <- {harness, android}
core <- render <- ui
store, render <- runtime
runtime <- devtools <- harness
```

Crates added by later slices: `openpaint-raster` (pure geometry: flood-fill regions, mask to selection; depends on core). Register it in `LAYERS` in `xtask/src/check/rules.rs` first.

## 3. Core types (target shape)

### core

```rust
pub struct Artwork(Arc<ArtworkState>);           // snapshot as value: clone is O(1), history stores versions
struct ArtworkState { canvas: CanvasSize, background: Background, layers: Vec<Layer>,
                      groups: Vec<LayerGroup>, active: LayerId, ids: IdCounters }
pub struct Layer { id: LayerId, name: Name<80>, props: LayerProps, strokes: StrokeList,
                   image: Option<LayerImage>, edits: Arc<[RegionEdit]> }
pub struct LayerProps { visible: bool, opacity: f32, locked: bool, alpha_locked: bool,
                        clipping: bool, blend: BlendMode, group: Option<GroupId> }
pub struct StrokeList { chunks: Arc<[Arc<[Stroke]>]> }         // appending copies only the last chunk (<= 256 strokes)
pub struct Stroke { id: StrokeId, tool: StrokeTool, style: StrokeStyle,
                    samples: Arc<[StrokeSample]>, constraints: StrokeConstraints }
pub struct StrokeSample { position: CanvasPoint, pressure: f32, tilt: f32, azimuth: f32, time_ms: u32 }  // time from stroke start
pub struct StrokeStyle { recipe: BrushRecipe, size: f32, opacity: f32, color: Color }
pub enum ArtworkCommand { Layer(LayerCommand), Group(GroupCommand),
                          Reference(ReferenceCommand), Background(BackgroundCommand) }
pub struct Applied { pub artwork: Artwork, pub change: Change }  // Change: None | ActiveLayer | Reference | Content
impl Artwork {
    pub fn apply(&self, cmd: ArtworkCommand) -> Result<Applied, CoreError>;     // one editable_layer() precheck
    pub fn edit_region(&self, edit: RegionEdit) -> Result<Applied, CoreError>;   // commits of selection, fill, transform
    pub fn begin_stroke(&self, style: StrokeStyle, first: StrokeSample) -> Result<StrokeSession, CoreError>;  // on the active layer
    pub fn from_storage(parts: ArtworkParts) -> Result<Self, CoreError>;        // the single full-validation entry
    pub fn to_parts(&self) -> ArtworkParts;                                     // plain data for the store
}
pub struct StrokeSession { base: Artwork, active: ActiveStroke }   // extend / finish / cancel; the active stroke is not in a layer
```

No public mutable setters. Layers come only from `from_storage` or commands; single values (`Stroke`, `LayerGroup`, `LayerImage`) validate themselves on construction. Landed in S2: canvas, layers, active, ids; `LayerProps { visible, opacity, locked }`; `StrokeStyle` without a recipe; `ArtworkCommand::Layer` only; `Change` without `Reference`. `LayerCommand` is `Create` (above the active layer, becomes active), `Delete`, `Reorder`, `Select`, `Rename`, `SetVisible`, `SetOpacity`, `SetLocked`; a command that changes nothing returns `Change::None` and the same version. Lock blocks content (strokes), not properties or position. Ids are never reused; limits (`MAX_LAYERS`, `MAX_STROKE_SAMPLES`, `MAX_CANVAS_SIDE`, ...) are public constants. Landed in S11 (D27): `BrushRecipe { softness, size: Dynamics, opacity: Dynamics, grain: Grain }`, `Dynamics { pressure: Curve, speed: Curve }`, `plain()`; `StrokeStyle` holds an `Arc<BrushRecipe>` and is `Clone`, not `Copy`; `RecipeParts` in `StrokeParts`. Landed in S12 (D30): `StrokeTool { Paint, Erase }` in `StrokeStyle` (`with_tool`); `LayerProps::alpha_locked` and `LayerCommand::SetAlphaLocked`; a stroke captures the layer's Alpha Lock when it begins, so `Stroke`/`ActiveStroke::blend()` is a `StrokeBlend { Over, Atop, Erase }` (stored in `StrokeParts::blend`); the eraser on an alpha-locked layer is `CoreError::AlphaLocked`. There is no separate `constraints` field yet.

### app

```rust
pub struct Project { identity: ArtworkIdentity, artwork: Artwork, view: CanvasView, editor: EditorState }  // the only store boundary type
pub enum Intent { Stroke(StrokeIntent), Artwork(ArtworkCommand), Tool(ToolIntent), View(ViewIntent),
                  Selection(SelectionIntent), Fill(FillIntent), Transform(TransformIntent),
                  Brush(BrushIntent), History(HistoryIntent) }
pub enum AppError { Core(CoreError), Busy(BusyReason), Brush(BrushLibraryError), Port(PortError) }
pub trait ProjectStore { /* list, create, load, save, rename, duplicate, delete */ }
pub trait AssetStore   { /* import_image, load_image */ }
pub trait BrushStore   { /* brushes, change_brushes */ }
pub struct History { entries: VecDeque<Entry>, cursor: usize, total_bytes: usize }   // Entry { artwork, bytes }
pub enum Flow<Req> { Idle, Picking, Ready(Req), Running { key: u64 }, Failed(PortError) }  // one field per async flow
impl Session { pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError>;
               pub fn projection(&self) -> Arc<SessionProjection>; }
impl SessionProjection { pub fn availability(&self, action: EditorAction) -> Availability;
                         pub fn limits(&self) -> Limits; }
```

The UI reads projections, keeps UI-local state and emits intents. Availability and value ranges come from the projection; the UI never assembles rules itself. Each intent family is one module holding its intents, handlers, projection and tests. Landed in S4: `Intent::Stroke(StrokeIntent::Contact(Contact))`, `Session { new, dispatch, artwork, active }` (no projection yet), `AppError::Core`, and the normalised input types `InputBatch`, `Contact`, `ContactPhase`, `Device`, `InputSample` (in app, because the UI routes them and the stroke intent consumes them). Landed in S6: `Intent::History(HistoryIntent::{Undo, Redo})`; `History` as sketched plus a `budget` (entry bytes = `Artwork::bytes_since` the previous entry; DECISIONS D20); `Session::projection()` returns a small `Copy` `SessionProjection` by value (not an `Arc` yet) with `availability(EditorAction) -> Availability { Available, NothingLeft, StrokeInProgress }`; `AppError::Unavailable { action, availability }`. Landed in S7: `Project { id: ProjectId, artwork }` (no view or editor state yet: nothing to store); `ProjectStore { open_last, create, save(id, &artwork) }`; `PortError { Device, Corrupt, NewerSchema, Invalid, Stopped }`; the autosave flow in `storage.rs` instead of a generic `Flow` (`Session::save_request`, `leave_request`, `save_finished`, `saving`; constructors `new`, `open`, `after_failed_open`); `Intent::Storage(RetrySave | Dismiss)`; projection `storage() -> StorageNotice { None, SaveFailed { retrying }, OpenFailed }` and actions `RetrySave`, `DismissNotice` (availability `Running` while a retry runs). No `AppError::Port`: port errors reach the session as results, not refusals. Landed in S9 (D23): `Intent::Layer(LayerIntent { Add, Select, Delete, Raise, Lower, SetVisible, SetOpacity { settled } })` in `layers.rs` instead of a raw `Intent::Artwork`, because raise/lower and slider touches need session rules; `History::amend`; `SessionProjection` is `Clone` and carries `layers() -> Option<&LayerList>` (rows bottom to top, active id) plus actions `AddLayer`, `DeleteLayer`, `RaiseLayer`, `LowerLayer`, `EditLayer` for the active layer (`Availability::Full` at `MAX_LAYERS`). Landed in S10 (D25): `ProjectStore { open_last, list, create(artwork, title), load, save, save_thumbnail, close, rename, duplicate, delete }`, `ProjectSummary { id, title, thumbnail }`, `Title` (trimmed, 1 to 80 characters, one line), `Thumbnail` (RGBA bytes, shared), `PortError::Missing`; `gallery.rs`: `Gallery { new, listed, dispatch, request, answer, projection }` with `Intent::Gallery(GalleryIntent { Show, Create, Open, Rename, Duplicate, Delete, Dismiss })`, `LibraryRequest`/`LibraryAnswer` for the worker and `GalleryProjection` (items, listed, availability, notice); `ScreenProjection { Starting, Canvas, Gallery }` is what the UI draws from; `EditorAction::Gallery` (leaving the canvas is refused while a stroke is drawn). The session stays per artwork: its history goes when the canvas is left. Landed in S11 (D27, D28): editor state is app-wide, not in `Project`: `Toolbox` (`BrushLibrary` of built-in TOML brushes parsed by `recipe.rs`, current brush, per-brush size and opacity; `dispatch(&BrushIntent)`, `style`, `projection`, `state`/`restore(EditorState)`, `save_request`) is owned by the runtime, which hands brush intents to it and its style to `Session::set_brush`; sessions start with a `StrokeStyle`; `Intent::Brush`; `SessionProjection::with_brush`/`brush()`; `ProjectStore` adds `editor`, `save_editor`. Landed in S12 (D30): the toolbox has `Tool { Brush, Eraser }`, each with its own brush and per-brush size and opacity (`BrushIntent` acts on the current tool), the colour and recent colours (`palette.rs`: colours rounded to 8 bits, `RECENT_COLORS` = 8, `Hsv`) and the eyedropper flag (`picking`, never stored); `Intent::Tool(ToolIntent { Use(Tool), Eyedropper(bool), Color { color, settled } })` goes to `Toolbox::dispatch_tool`; `EditorState { tool, paint, erase: BrushChoice, color, recent }`; `BrushProjection` adds `tool`, `color`, `recent`, `picking`; `LayerIntent::SetAlphaLock` and `LayerRow::alpha_locked`. Landed in S13 (D31): `transfer.rs` holds `Transfers` (the export and reference flows, owned by the runtime for the app; one system dialog at a time) with the platform ports `ImageSink`/`ImageSource`, `HostRequest`/`HostAnswer` and `TransferJob` for the worker, instead of `AssetStore` and a generic `Flow`; `Intent::Export(ExportIntent)`, `Intent::Reference(ReferenceIntent)`; `ReferenceImage` (opaque RGBA, at most 2048 px, `color_at`); `SessionProjection::with_transfers`; `Project::title`. Landed in S14 (D35): `view.rs` holds `CanvasView` (pan in points from the area's centre, zoom relative to fit, rotation), `Viewport`, `ViewGesture`, `ViewMapping` (the one screen-to-canvas map for input, rendering and picking) and `Intent::View(ViewIntent { Gesture, Fit })`; the session keeps the view, never in history and not stored (`Project` has no view yet); `gestures/` is the pure finger recogniser (`Gestures`: pair, navigation filter, one-finger pan, taps) the UI feeds; `CanvasPreset` and `GalleryIntent::Create(CanvasPreset)`; `SessionProjection::{view, canvas, drawing}`; `Availability::is_offered` is what a control's look follows (`StrokeInProgress` looks offered).

### render

```rust
pub struct Core { gpu: Gpu, evaluate: EvaluatePipeline, pages: PageCache,
                  residuals: Mutex<smudge::Engine>, grain: GrainTexture }
pub struct Renderer { core: Arc<Core>, instances: Mutex<Instances>, sinks: Sinks }
pub struct FrameRequest<'a> { artwork: &'a Artwork, active: Option<&'a ActiveStroke>, background: Color,
                              images: ImageSet, view: ViewFrame, consumer: Consumer }
pub struct RegionEvaluator<'a> { core: &'a Core, executor: &'a mut Executor, target: Target<'a> }  // the single evaluation entry
pub enum Target<'a> { Pages(..), Readback(..), Planes(..), SmudgeTiles(..) }
pub enum Batching { Frame, Bounded { visits: u64, invocations: usize, dispatches: usize } }
#[repr(u32)] pub enum Opcode { Segment, EndStroke, EndLayer, Image, BeginScope, EndScope, Fill, Smudge }  // generates the WGSL constants
pub enum RenderError { Admission { what: &'static str, limit: u64 }, Cancelled,
                       MissingImage(AssetId), Device(String), Invalid(&'static str) }
```

Evaluation (DECISIONS D16): `Scene::build(&FrameRequest)` flattens visible layers, their strokes and the active stroke into ops in drawing order, each segment op with conservative pixel bounds; `Plan::build(scene, size, regions)` widens the regions to 16 px tiles and lists per tile, in scene order, only the ops that reach it; the compute shader runs one workgroup per tile and one invocation per pixel through that list. `protocol.rs` owns the opcodes and the `Op`, `Tile` and `Params` records and generates their WGSL; a naga-based test checks every offset and size. Landed in S3: `Core { gpu, evaluate }`, `FrameRequest { artwork, active, background }`, `RegionEvaluator { core, target: &CanvasTarget }` (an `Rgba8Unorm` storage texture at canvas size, read back with `CanvasTarget::read`), `PixelRect`, `stroke_region(style, samples)` for the dirty region of new samples, opcodes `Segment`, `EndStroke`, `EndLayer`. Landed in S4 (D17): `RegionEvaluator::new(core, &mut CanvasTarget)`; the target keeps the in-progress stroke's coverage (`R32Float`), the last scene's op buffer and its bindings; opcode `EndActive` and `FRESH`-flagged indices into per-evaluation ops; `ScreenPainter`/`ScreenFrame`/`CanvasPlacement` blit the canvas under the UI; `ScreenTarget` is the harness's offscreen screen. Landed in S6: `changed_regions(before, after)` turns a version switch into the regions callers pass (one per segment of the strokes one version has beyond the other; the whole canvas otherwise). Since S9 the runtime passes it for every change other than committing a stroke (undo, redo, layer edits); layers composite bottom to top, Normal source-over with layer opacity, hidden layers contribute no ops. Landed in S10: `Core::thumbnail(&CanvasTarget, max)` reduces the target on the GPU (box means in integers, `thumbnail.wgsl`) and reads back at most `max` pixels; `thumbnail_size` and `thumbnail_reference` (the exact CPU result) are public. Landed in S11: `stroke.rs` owns stroke geometry (per-sample radius and flow from the recipe and pen speed, `stroke_region(style, samples, from)`); `Op` has 9 operands (segment flows and edge, paint grain) whose named slots and `GRAIN_ONE` are generated into WGSL and checked by the layout test; no grain texture: grain is integer hashing in the shader. Landed in S12: paint ops carry a `Blend` (slot `PAINT_BLEND`, values generated into WGSL): over, atop (Alpha Lock: colour changes only where the layer has alpha) and erase (destination out); zero paint leaves a pixel unchanged in each. `CanvasTarget::pick(core, [x, y])` is render picking: one texel of the target the single evaluation entry filled, the same bytes `read` gives there. `protocol_tests.rs` holds the layout test. Landed in S13: `Core::export(artwork, paper)` evaluates the whole canvas into a target of its own and reads it back; it is called from the runtime's transfer worker. Landed in S14 (D35): `CanvasPlacement { target, origin, size, to_canvas }`: the blit clips to the canvas area, maps each screen pixel through the view's affine rows (a uniform per target) and samples the target bilinearly (`Core::blit_sampler`, no mipmaps); outside the canvas the surround shows. The target and every evaluation region stay in canvas pixels.

Directories: `core/ error budget geom adapter/ scene/ program/ gpu/ cache/ view/ tasks/ consumers/ tests/`. `adapter/` is the only directory that depends on egui. Background consumers (export, fill, pick) hold only an `Arc<Core>` and never touch a UI lock. Incremental evaluation and cold evaluation are bit-identical, proven by tests.

### runtime

```rust
pub struct OneShot<T>;                 // spawn / poll / finish; joins on Drop
pub struct Latest<K, R, T>;            // one request slot, one running, one unclaimed
pub struct GestureJob<Job, Out>;       // cancellable, suspendable gesture work
pub fn drain_for_lifecycle(tasks: &mut Tasks, order: &[TaskKind]) -> Result<(), RuntimeError>;  // the only definition of drain order
pub trait PlatformServices { fn input_batch(&mut self) -> Option<InputBatch>; fn request(&mut self, effect: Effect); }
pub trait FrameHook { /* before_ui, after_ui, takeover */ }     // devtools attach here; the product runtime does not know them
```

`run_frame` has fixed steps: drain receipts -> platform events -> UI -> dispatch intents -> submit requests -> mount rendering -> schedule repaint. Background work uses only the three primitives; `thread::spawn` never appears directly in product code. Landed in S4: `Runtime` with `motion(&MotionBatch) -> bool` (normalised and queued), `frame()` (input -> egui -> `Shell::route` -> `Session::dispatch` -> evaluate -> present) and the harness's `frame_offscreen`; no workers, no `PlatformServices` or `FrameHook` yet. The S5 scenario runner drives `Runtime` from outside through that same public API (motion, frame, readback), so it needs no hook. Landed in S7: `worker::Latest<Req, Out>` (one thread, a waiting slot a newer request replaces, one unclaimed result, `drain(limit)`, drop serves the waiting request and joins) is the only place a thread starts; `storage::Storage` owns the `ProjectStore` on it (open the last project at start, then saves; the first save creates the project). `Runtime::start(density, Box<dyn ProjectStore>)`; the canvas opens when the library has answered and the area is known; a frame hands `save_request()` to the worker and asks for frames while `storing()`; `leave()` (Android `onPause`) hands over the current or failed version and drains up to 2 s; `projection()` is public. No `OneShot`, `drain_for_lifecycle` or `PlatformServices` yet. Landed in S8: `motion` passes normalised batches through `PalmRejection`; `MotionPhase::PointerCancel { index }`. The Android host keeps a `PenLatency` probe beside the runtime (`apps/android/src/latency.rs`, pure Rust) and logs one summary per stroke; frames still run on the main thread (D22). Landed in S10: `worker::Queue<Req, Out>` (FIFO, every result kept until taken, `drain`, drop serves what is queued) replaces `Latest`; `storage.rs` serves `Job { Start, Save, Leave { version, thumbnail, close }, Create, Library(..) }` and knows which project is open; `screen.rs` holds `Screen { Opening, Canvas, Gallery }` and turns the worker's answers into screen changes (the canvas is left only after its `Leave` succeeded). New `Runtime` calls: `screen`, `back`, `text(TextInput)`, `keyboard_wanted` (D26). Landed in S12: `Canvas::dispatch(core, intents, toolbox)` hands brush and tool intents to the toolbox and, while the eyedropper is on, turns canvas contacts (pen or finger) into picks read back from the canvas target on the main thread. Landed in S13: `transfer.rs`, a second `worker::Queue` (`openpaint-transfer`, holding an `Arc<Core>`) that exports and decodes reference images; `Runtime::host_request` hands the app's requests to the host, `answer` takes the host's reply. The Android host serves them in `Pickers.kt` and `ShareProvider.kt`. Landed in S14: `clock.rs`, the input clock (newest sample time carried on by the wall clock), handed to the shell each frame (`Shell::set_now`); `frame()` asks for the next frame while `Shell::animating()` (the view status fading, fingers held for a repeating shortcut).

## 4. Input

Hosts deliver platform events as `runtime::MotionBatch` (phase, buttons, pointers with tool kind and batched samples: position, pressure, tilt, orientation, time); the Android host decodes `MotionEvent` arrays into it in `apps/android/src/motion.rs`. The Android host and the harness feed one normaliser (`runtime/src/normalise.rs`): `MotionBatch` -> `app::InputBatch` (per contact: id, device, phase, samples with position, pressure, tilt, azimuth, timestamp; ranges clamped, times monotonic per contact). egui receives its pointer events from these batches on every platform; no windowing library sits in between. The UI's canvas input (`ui/src/canvas_input.rs`) owns each pen contact that touches down on the canvas itself, outside every control, and hands it to the session as `Intent::Stroke` with its samples mapped through the view into canvas pixels (while the eyedropper is on, any contact, picking). Every finger contact goes to `app::Gestures` with whether it touched down on open canvas; what fingers mean (navigation, quick pinch, taps) is decided there, in points, not in the UI's widgets and not in the platform layer; a control that takes a contact resets the trackers. Stabilisation will live in the app's stroke intent. Palm rejection is an app rule too, `app::PalmRejection`, which the runtime applies to every normalised batch before egui or the canvas frame sees it, because a palm must not press buttons either: while a pen contact is down, touches are palms until they lift; a pen landing cancels the touches already down. Platform cancels (`ACTION_CANCEL`, `FLAG_CANCELED` on an up) arrive as `Cancel`; egui gets a cancel as a release far off screen, never a click. The stroke intent drops the zero-pressure lift sample Android reports as the pen leaves the glass. The in-progress stroke travels to the renderer as a separate item and never enters the artwork. Keyboard text reaches egui as IME and key events through `Runtime::text`; the host shows the soft keyboard while `keyboard_wanted` (D26). The back button goes to `Runtime::back`, which the shell answers (canvas to gallery, closing a dialog) or leaves to the system.

## 5. Persistence

- The store owns the format. The `Project` DTO carries an integer schema version; every version has one pure-data migration step and one frozen sample, and the gate round-trips every sample.
- Editor state (current brush, colour, tool parameters) is stored separately from the artwork; when corrupt it falls back to defaults and never blocks opening the artwork.
- Image assets are reclaimed by reference, touching only the artworks this change involves; assets still referenced by session history are never reclaimed.
- Autosave runs on a worker when a stroke finishes and when the editor is left; failure shows a clear message with a retry entry.
- Landed in S7 (DECISIONS D21): `openpaint_store::Library::at(dir)` implements `ProjectStore` on fjall; `records.rs` is schema 1 (keys and values), `codec.rs` the byte reader and writer, `delta.rs` which stroke records a save writes and removes. Android keeps the library in `getFilesDir()/library`; the harness in `artifacts/`. S9 needed no format change (D24). Schema 2 landed in S10 (D25): `catalogue.rs` (entries, thumbnails), `migrate.rs` (the pure step 1 -> 2, applied when a schema-1 library opens), `read.rs` and `write.rs` (one synced batch per call); samples `schema-1.bin` (opens upgraded) and `schema-2.bin` (byte-exact). Schema 3 landed in S11 (D28): recipes in stroke records, the toolbox in `library/editor` (`editor.rs`); `migrate.rs` steps 1 -> 2 -> 3; `schema-3.bin` byte-exact, the older samples open upgraded. Schema 4 landed in S12 (D29): Alpha Lock per layer in the head, a blend byte per stroke, a toolbox of two tools with colour and recent colours; `migrate.rs` steps 1 -> 2 -> 3 -> 4, each seeing the heads and strokes the step before wrote; `schema-4.bin` byte-exact.

## 6. Brushes and rendering

- A brush is a declarative recipe (TOML): shape, size and opacity curves over pressure and speed, grain, dynamics. Rendering evaluates continuously per pixel along the stroke; there is no dab/stamp engine, no node graph, no shader plugin.
- A stroke stores a recipe snapshot; reopening an artwork never silently applies a renamed brush's new behaviour.
- Landed in S11 (D27): stages softness, size and opacity dynamics (pressure x speed curves) and grain; "dynamics" beyond pressure and speed (tilt, azimuth) are not stages yet. Built-ins Pencil, Ink Pen (= the plain round brush, the default) and Soft Brush.
- Export always uses native precision; the screen shows a fast result first and swaps in the refined one when background work completes, without changing extent or strength.

## 7. Errors

`CoreError` (artwork semantics), `AppError` (modes and flows), `PortError` (external capabilities), `RenderError`, `RuntimeError`. The platform layer translates them into user-facing messages; `String` appears only in JNI, system APIs and xtask.

## 8. Testing

| Tier | Content | When |
| --- | --- | --- |
| unit | pure tests for core/app/store; GPU readback tests for render (golden hashes, incremental = cold) | `cargo test -p` |
| gate | fmt, clippy, all tests, the `smoke` scenario, repo-check; target under 60 s warm | before the commit |
| regression | harness scenarios on the dev machine: JSON steps plus pixel checkpoints, seconds | the short scenario runs inside the gate |
| device | one smoke scenario of at most 20 s; full scenarios and manual feel checks only at stage exits | slices marked in ROADMAP |

Test fixtures live in each crate's `tests/` or in a dedicated test crate, never in the product API.

Scenarios (S5, DECISIONS D19): `tests/scenarios/<name>.json` holds a walk-through (text) and the screen size, density and steps that replay it: `tap`, `drag`, `stroke`, `wait`, `restart` (leave, end the process, start again on the same library), `storage_fails` / `storage_works` (a devtools store wrapper fails every call while switched off), `checkpoint` (canvas hash) and `screenshot`; a `stroke` may carry a `lift` (Android's zero-pressure lift sample at that point) and a `palm` (a finger contact at a point, landing `before` the pen or `during` the stroke). After every step the runner runs frames until the storage worker is idle. `openpaint_devtools::run` turns gestures into `MotionBatch`es (one batch per frame, 4 samples 4 ms apart, at most 4 px apart) and drives `Runtime` offscreen on a library directory the caller gives; `cargo scenario <name>` (harness) empties `artifacts/scenarios/<name>/library/`, runs, and writes PNGs and failure diffs under `artifacts/scenarios/<name>/`. The gate runs `smoke`. Later slices add the steps they need to `Step`; S10 added `type` (text committed by the keyboard) and `back`; S14 `touch` (1 to 5 fingers moving together, Android's pointer order) and `view` (a view checkpoint with tolerances). On the tablet, debug builds inject the same kind of gestures through `GestureInstrumentation` (UiAutomation), which `adb shell input` cannot.

## 9. Enforced rules

`cargo repo-check` (`xtask/src/check/rules.rs`): .rs files at most 400 lines; dependency direction per the table in section 2; purity tokens per table (core never mentions serde; devtools never touches `std::fs`); egui in render only under adapter; no cargo features in product crates; crate roots carry `#![forbid(unsafe_code)]` (android excepted); markdown limited to six files plus the archive; no script files; no CRLF. clippy: functions at most 60 lines, no `todo!` / `unimplemented!` / `dbg!`, warnings are errors; the gate also runs clippy on `openpaint-android` for `aarch64-linux-android`, the only build that compiles its JNI code.

## 10. UI conventions

The UI is designed from the user inward (AGENTS.md, "Design from the user inward"). These are the fixed defaults.

- The default interaction model is Procreate's. Before building a feature, read its Procreate Handbook page (entry point, gesture, feedback) and follow it. Deviate only for Android tablet hardware or on the author's instruction. Never copy icons, colours or assets.
- Layout skeleton: the canvas fills the screen. A top bar holds gallery, undo/redo and tools. A side bar holds the size and opacity sliders and can be mirrored for left-handed use. Panels are popovers over the canvas and close with one tap outside. A modal appears only before a destructive action.
- Popovers (S9, `components::popover`): anchored under the top bar's right end, drawn whole in the frame they open. A contact starting on a popover never reaches the canvas (`CanvasInput::route` takes the cover); a contact landing on the canvas outside closes it, and a pen stroke there draws. Components: `icon_button`, `icon_toggle` (highlighted while its panel is open), `text_button`, `list_row` (chosen row highlighted, an icon button at its end), `slider` (value and `settled`), `notice`, `popover`; since S10 `dialog` (over a dimmed screen, centred or under the top bar; a tap outside cancels), `text_field`, `artwork_card`.
- Side bar and brushes (S11): `components::side_panel` floats at the left edge centred on the canvas area (one constant mirrors it), holding two `side_slider`s (vertical, a value bubble while held) and undo and redo; contacts starting on it never reach the canvas (`CanvasInput::route` takes every cover). The brush panel is a popover of `brush_row`s with previews painted by the evaluator.
- Tools and colour (S12): the top bar's right end holds, right to left, the colour button (a `swatch` of the current colour), Layers, Eraser and Brush (Procreate's order); the current tool's button is highlighted and opens its brush panel, the other tool's button switches tools. The colour popover holds a `color_wheel` (hue ring, saturation and value square) and a row of recent swatches. The eyedropper is an `icon_toggle` between the side bar's sliders. Alpha Lock is a `switch_row` in the layer panel's footer. One panel at a time.
- Actions and reference (S13): Actions (a spanner) follows Gallery at the top bar's left end and opens a popover of `menu_row`s and a `switch_row`; progress and results show as a `status` or `notice` in the top bar; the reference picture is a `floating` window (title bar to move, zoom buttons, close) whose contacts never reach the canvas.
- View (S14): the canvas opens at fit on the `SURROUND`; a `view_status` line under the top bar while and briefly after the view moves, giving way to the pen at once.
- Gallery (S10): the canvas's bar starts with Gallery; the gallery screen has Gallery and "+" (since S14 a New canvas popover of `menu_row`s, one per size) in its bar and a left-aligned grid of cards (picture on the bottom of a 256 x 192 point box, title, duplicate and delete buttons), the artwork changed last first. Rename is a dialog under the top bar; delete asks in a centred dialog, the one modal before something that cannot be undone.
- The theme is defined once in `ui/src/design/`: paper-white line art (D32: near-neutral warm paper chrome, ink-line outlines, self-drawn ink-line icons behind a semantic `Icon` enum, one deep ink colour for the active state), an 8 px spacing scale, one type scale, one icon set. Every button, slider and panel is a component in `ui/src/components/`; screens compose components and never style widgets inline. Intuitive (a first-day user needs no instructions) and restrained (UI appears when needed; cues over controls); hidden shortcuts only duplicate a visible control.
- Every feature ships with a discoverable entry, visible feedback for success, failure and cancel, undo for anything that changes the artwork, and state that survives reopen. Missing any of these means the slice is not done.
- Checklist that harness screenshots and scenarios must show: the most frequent actions (brush, size, opacity, colour, undo, layer) within two taps; hit targets at least 48 points at tablet density (the icon is not the hit area); nothing covers the canvas while a stroke is possible; feedback within one frame; the same component for the same job everywhere.
- Lessons carried over from draweave (D33; each was a failed author check or a fix there):
  - Controls have three states: available; unavailable (greyed, nothing to act on); temporarily blocked (normal look and hit area, the action is refused at the intent). An active stroke or a captured panel only ever blocks temporarily; nothing greys out or flickers per stroke.
  - Any control that takes a contact resets the gesture trackers, so a button tap never lingers as a canvas gesture. Gestures read raw touch geometry including the release positions (Android batches the last move with the up).
  - A tap outside a panel closes it and consumes only that contact; the next contact draws. The tap that opens a panel never dismisses it. Panels float and never shrink the canvas; they size to their content.
  - Sliders say what they set (a label or end icons plus the live value), show a preview while dragging (size: a circle at the real size; opacity: a swatch at that alpha), restore the original value on cancel or focus loss, and belong to one contact at a time.
  - The pen's hover shows the brush outline at full size, a 1-point line contrasting with the canvas under it, hidden over controls and during multi-touch. The eyedropper commits the last valid preview, not where the contact drifted on release.
  - Animations advance by elapsed time, never per frame. Notices are brief; quiet success, persistent failure. Every icon-only button has an accessible name and a hover label; new concepts and destructive actions use icon and text.
- Who decides: details are the agent's. The author judges on the tablet at stage exits and sends one-line change requests. Ask the author in advance only for a deviation from Procreate's mental model, for anything the hand touches continuously (side bar position, gesture mapping, pen buttons) or for anything that changes the persistent format.
