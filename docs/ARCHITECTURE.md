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

No public mutable setters. Layers come only from `from_storage` or commands; single values (`Stroke`, `LayerGroup`, `LayerImage`) validate themselves on construction. Landed in S2: canvas, layers, active, ids; `LayerProps { visible, opacity, locked }`; `StrokeStyle` without a recipe; `ArtworkCommand::Layer` only; `Change` without `Reference`. `LayerCommand` is `Create` (above the active layer, becomes active), `Delete`, `Reorder`, `Select`, `Rename`, `SetVisible`, `SetOpacity`, `SetLocked`; a command that changes nothing returns `Change::None` and the same version. Lock blocks content (strokes), not properties or position. Ids are never reused; limits (`MAX_LAYERS`, `MAX_STROKE_SAMPLES`, `MAX_CANVAS_SIDE`, ...) are public constants.

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

The UI reads projections, keeps UI-local state and emits intents. Availability and value ranges come from the projection; the UI never assembles rules itself. Each intent family is one module holding its intents, handlers, projection and tests. Landed in S4: `Intent::Stroke(StrokeIntent::Contact(Contact))`, `Session { new, dispatch, artwork, active }` (no projection yet), `AppError::Core`, and the normalised input types `InputBatch`, `Contact`, `ContactPhase`, `Device`, `InputSample` (in app, because the UI routes them and the stroke intent consumes them). Landed in S6: `Intent::History(HistoryIntent::{Undo, Redo})`; `History` as sketched plus a `budget` (entry bytes = `Artwork::bytes_since` the previous entry; DECISIONS D20); `Session::projection()` returns a small `Copy` `SessionProjection` by value (not an `Arc` yet) with `availability(EditorAction) -> Availability { Available, NothingLeft, StrokeInProgress }`; `AppError::Unavailable { action, availability }`. Landed in S7: `Project { id: ProjectId, artwork }` (no view or editor state yet: nothing to store); `ProjectStore { open_last, create, save(id, &artwork) }`; `PortError { Device, Corrupt, NewerSchema, Invalid, Stopped }`; the autosave flow in `storage.rs` instead of a generic `Flow` (`Session::save_request`, `leave_request`, `save_finished`, `saving`; constructors `new`, `open`, `after_failed_open`); `Intent::Storage(RetrySave | Dismiss)`; projection `storage() -> StorageNotice { None, SaveFailed { retrying }, OpenFailed }` and actions `RetrySave`, `DismissNotice` (availability `Running` while a retry runs). No `AppError::Port`: port errors reach the session as results, not refusals.

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

Evaluation (DECISIONS D16): `Scene::build(&FrameRequest)` flattens visible layers, their strokes and the active stroke into ops in drawing order, each segment op with conservative pixel bounds; `Plan::build(scene, size, regions)` widens the regions to 16 px tiles and lists per tile, in scene order, only the ops that reach it; the compute shader runs one workgroup per tile and one invocation per pixel through that list. `protocol.rs` owns the opcodes and the `Op`, `Tile` and `Params` records and generates their WGSL; a naga-based test checks every offset and size. Landed in S3: `Core { gpu, evaluate }`, `FrameRequest { artwork, active, background }`, `RegionEvaluator { core, target: &CanvasTarget }` (an `Rgba8Unorm` storage texture at canvas size, read back with `CanvasTarget::read`), `PixelRect`, `stroke_region(style, samples)` for the dirty region of new samples, opcodes `Segment`, `EndStroke`, `EndLayer`. Landed in S4 (D17): `RegionEvaluator::new(core, &mut CanvasTarget)`; the target keeps the in-progress stroke's coverage (`R32Float`), the last scene's op buffer and its bindings; opcode `EndActive` and `FRESH`-flagged indices into per-evaluation ops; `ScreenPainter`/`ScreenFrame`/`CanvasPlacement` blit the canvas under the UI; `ScreenTarget` is the harness's offscreen screen. Landed in S6: `changed_regions(before, after)` turns a version switch into the regions callers pass (one per segment of the strokes one version has beyond the other; the whole canvas otherwise).

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

`run_frame` has fixed steps: drain receipts -> platform events -> UI -> dispatch intents -> submit requests -> mount rendering -> schedule repaint. Background work uses only the three primitives; `thread::spawn` never appears directly in product code. Landed in S4: `Runtime` with `motion(&MotionBatch) -> bool` (normalised and queued), `frame()` (input -> egui -> `Shell::route` -> `Session::dispatch` -> evaluate -> present) and the harness's `frame_offscreen`; no workers, no `PlatformServices` or `FrameHook` yet. The S5 scenario runner drives `Runtime` from outside through that same public API (motion, frame, readback), so it needs no hook. Landed in S7: `worker::Latest<Req, Out>` (one thread, a waiting slot a newer request replaces, one unclaimed result, `drain(limit)`, drop serves the waiting request and joins) is the only place a thread starts; `storage::Storage` owns the `ProjectStore` on it (open the last project at start, then saves; the first save creates the project). `Runtime::start(density, Box<dyn ProjectStore>)`; the canvas opens when the library has answered and the area is known; a frame hands `save_request()` to the worker and asks for frames while `storing()`; `leave()` (Android `onPause`) hands over the current or failed version and drains up to 2 s; `projection()` is public. No `OneShot`, `drain_for_lifecycle` or `PlatformServices` yet.

## 4. Input

Hosts deliver platform events as `runtime::MotionBatch` (phase, buttons, pointers with tool kind and batched samples: position, pressure, tilt, orientation, time); the Android host decodes `MotionEvent` arrays into it in `apps/android/src/motion.rs`. The Android host and the harness feed one normaliser (`runtime/src/normalise.rs`): `MotionBatch` -> `app::InputBatch` (per contact: id, device, phase, samples with position, pressure, tilt, azimuth, timestamp; ranges clamped, times monotonic per contact). egui receives its pointer events from these batches on every platform; no windowing library sits in between. The UI's canvas frame owns each contact that touches down inside it and hands it to the session in canvas pixels as `Intent::Stroke`; it never decides what a device does. Gestures, palm rejection and stabilisation live in the app's stroke intent, not in the UI and not in the platform layer. The in-progress stroke travels to the renderer as a separate item and never enters the artwork.

## 5. Persistence

- The store owns the format. The `Project` DTO carries an integer schema version; every version has one pure-data migration step and one frozen sample, and the gate round-trips every sample.
- Editor state (current brush, colour, tool parameters) is stored separately from the artwork; when corrupt it falls back to defaults and never blocks opening the artwork.
- Image assets are reclaimed by reference, touching only the artworks this change involves; assets still referenced by session history are never reclaimed.
- Autosave runs on a worker when a stroke finishes and when the editor is left; failure shows a clear message with a retry entry.
- Landed in S7 (DECISIONS D21): `openpaint_store::Library::at(dir)` implements `ProjectStore` on fjall; `records.rs` is schema 1 (keys and values), `codec.rs` the byte reader and writer, `delta.rs` which stroke records a save writes and removes. Android keeps the library in `getFilesDir()/library`; the harness in `artifacts/`.

## 6. Brushes and rendering

- A brush is a declarative recipe (TOML): shape, size and opacity curves over pressure and speed, grain, dynamics. Rendering evaluates continuously per pixel along the stroke; there is no dab/stamp engine, no node graph, no shader plugin.
- A stroke stores a recipe snapshot; reopening an artwork never silently applies a renamed brush's new behaviour.
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

Scenarios (S5, DECISIONS D19): `tests/scenarios/<name>.json` holds a walk-through (text) and the screen size, density and steps that replay it: `tap`, `drag`, `stroke`, `wait`, `restart` (leave, end the process, start again on the same library), `storage_fails` / `storage_works` (a devtools store wrapper fails every call while switched off), `checkpoint` (canvas hash) and `screenshot`. After every step the runner runs frames until the storage worker is idle. `openpaint_devtools::run` turns gestures into `MotionBatch`es (one batch per frame, 4 samples 4 ms apart, at most 4 px apart) and drives `Runtime` offscreen on a library directory the caller gives; `cargo scenario <name>` (harness) empties `artifacts/scenarios/<name>/library/`, runs, and writes PNGs and failure diffs under `artifacts/scenarios/<name>/`. The gate runs `smoke`. Later slices add the steps they need to `Step`.

## 9. Enforced rules

`cargo repo-check` (`xtask/src/check/rules.rs`): .rs files at most 400 lines; dependency direction per the table in section 2; purity tokens per table (core never mentions serde; devtools never touches `std::fs`); egui in render only under adapter; no cargo features in product crates; crate roots carry `#![forbid(unsafe_code)]` (android excepted); markdown limited to six files plus the archive; no script files; no CRLF. clippy: functions at most 60 lines, no `todo!` / `unimplemented!` / `dbg!`, warnings are errors; the gate also runs clippy on `openpaint-android` for `aarch64-linux-android`, the only build that compiles its JNI code.

## 10. UI conventions

The UI is designed from the user inward (AGENTS.md, "Design from the user inward"). These are the fixed defaults.

- The default interaction model is Procreate's. Before building a feature, read its Procreate Handbook page (entry point, gesture, feedback) and follow it. Deviate only for Android tablet hardware or on the author's instruction. Never copy icons, colours or assets.
- Layout skeleton: the canvas fills the screen. A top bar holds gallery, undo/redo and tools. A side bar holds the size and opacity sliders and can be mirrored for left-handed use. Panels are popovers over the canvas and close with one tap outside. A modal appears only before a destructive action.
- The theme is defined once in `ui/src/design/`: neutral dark greys for chrome (saturated chrome shifts colour perception), an 8 px spacing scale, one type scale, one icon set. Every button, slider and panel is a component in `ui/src/components/`; screens compose components and never style widgets inline.
- Every feature ships with a discoverable entry, visible feedback for success, failure and cancel, undo for anything that changes the artwork, and state that survives reopen. Missing any of these means the slice is not done.
- Checklist that harness screenshots and scenarios must show: the most frequent actions (brush, size, opacity, colour, undo, layer) within two taps; hit targets at least 48 px at tablet density; nothing covers the canvas while a stroke is possible; feedback within one frame; the same component for the same job everywhere.
- Who decides: details are the agent's. The author judges on the tablet at stage exits and sends one-line change requests. Ask the author in advance only for a deviation from Procreate's mental model, for anything the hand touches continuously (side bar position, gesture mapping, pen buttons) or for anything that changes the persistent format.
