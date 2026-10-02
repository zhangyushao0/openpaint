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
| `openpaint-harness` | development harness: offscreen rendering, recorded-input replay, scenario runner, PNG and hash output; never shipped | all | windowing, mouse input, product rules |
| `openpaint-android` | the only product composition root: activity lifecycle, Surface, system pickers, JNI | all | product rules |
| `xtask` | repository tooling: `cargo gate`, `cargo repo-check` | nothing | product code |

Dependencies only point down (`A <- B` means B depends on A):

```text
core <- app <- store
core <- app <- ui <- runtime <- {harness, android}
core <- render <- ui
store, render <- runtime
```

Crates added by later slices: `openpaint-raster` (pure geometry: flood-fill regions, mask to selection; depends on core) and `openpaint-devtools` (scenario runner, observers, statistics; depends on runtime, assembled by hosts on demand). Register them in `LAYERS` in `xtask/src/check/rules.rs` first.

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
pub struct StrokeSample { position: CanvasPoint, pressure: f32, tilt: [f32; 2], time_ms: u32 }
pub struct StrokeStyle { recipe: BrushRecipe, size: f32, opacity: f32, color: Color }
pub enum ArtworkCommand { Layer(LayerCommand), Group(GroupCommand),
                          Reference(ReferenceCommand), Background(BackgroundCommand) }
pub struct Applied { pub artwork: Artwork, pub change: Change }  // Change: None | ActiveLayer | Reference | Content
impl Artwork {
    pub fn apply(&self, cmd: ArtworkCommand) -> Result<Applied, CoreError>;     // one editable_layer() precheck
    pub fn edit_region(&self, edit: RegionEdit) -> Result<Applied, CoreError>;   // commits of selection, fill, transform
    pub fn begin_stroke(&self, begin: BeginStroke) -> Result<StrokeSession, CoreError>;
    pub fn from_storage(parts: ArtworkParts) -> Result<Self, CoreError>;        // the single full-validation entry
}
pub struct StrokeSession { base: Artwork, active: ActiveStroke }   // extend / finish / cancel; the active stroke is not in a layer
```

No public mutable setters. Layers come only from `from_storage` or commands; single values (`Stroke`, `LayerGroup`, `LayerImage`) validate themselves on construction.

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

The UI reads projections, keeps UI-local state and emits intents. Availability and value ranges come from the projection; the UI never assembles rules itself. Each intent family is one module holding its intents, handlers, projection and tests.

### render

```rust
pub struct Core { gpu: Gpu, pipelines: Pipelines, pages: PageCache,
                  residuals: Mutex<smudge::Engine>, grain: GrainTexture }
pub struct Renderer { core: Arc<Core>, instances: Mutex<Instances>, sinks: Sinks }
pub struct FrameRequest { snapshot: Artwork, active: Option<ActiveStroke>, images: ImageSet,
                          view: ViewFrame, viewport_px: [u32; 2], consumer: Consumer }
pub struct RegionEvaluator<'a> { core: &'a Core, executor: &'a mut Executor, target: Target<'a> }  // the single evaluation entry
pub enum Target<'a> { Pages(..), Readback(..), Planes(..), SmudgeTiles(..) }
pub enum Batching { Frame, Bounded { visits: u64, invocations: usize, dispatches: usize } }
#[repr(u32)] pub enum Opcode { Segment, EndLayer, Image, BeginScope, EndScope, Fill, Smudge }  // generates the WGSL constant block
pub enum RenderError { Admission { what: &'static str, limit: u64 }, Cancelled,
                       MissingImage(AssetId), Device(String), Invalid(&'static str) }
```

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

`run_frame` has fixed steps: drain receipts -> platform events -> UI -> dispatch intents -> submit requests -> mount rendering -> schedule repaint. Background work uses only the three primitives; `thread::spawn` never appears directly in product code.

## 4. Input

The Android host and the harness feed one normaliser: platform events (Android `MotionEvent` fields, or recorded samples in the harness) -> `InputBatch` (device kind, phase, position, pressure, tilt, timestamp). egui receives its `RawInput` from this normaliser on every platform; no windowing library sits in between. Gestures, palm rejection and stabilisation live in the app's stroke intent, not in the UI and not in the platform layer. The in-progress stroke travels to the renderer as a separate item and never enters the artwork.

## 5. Persistence

- The store owns the format. The `Project` DTO carries an integer schema version; every version has one pure-data migration step and one frozen sample, and the gate round-trips every sample.
- Editor state (current brush, colour, tool parameters) is stored separately from the artwork; when corrupt it falls back to defaults and never blocks opening the artwork.
- Image assets are reclaimed by reference, touching only the artworks this change involves; assets still referenced by session history are never reclaimed.
- Autosave runs on a worker when a stroke finishes and when the editor is left; failure shows a clear message with a retry entry.

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
| gate | fmt, clippy, all tests, repo-check; target under 60 s warm | before the commit |
| regression | harness scenarios on the dev machine: JSON steps plus pixel checkpoints, seconds | the short scenario runs inside the gate |
| device | one smoke scenario of at most 20 s; full scenarios and manual feel checks only at stage exits | slices marked in ROADMAP |

Test fixtures live in each crate's `tests/` or in a dedicated test crate, never in the product API.

## 9. Enforced rules

`cargo repo-check` (`xtask/src/check/rules.rs`): .rs files at most 400 lines; dependency direction per the table in section 2; purity tokens per table; egui in render only under adapter; no cargo features in product crates; crate roots carry `#![forbid(unsafe_code)]` (android excepted); markdown limited to six files plus the archive; no script files; no CRLF. clippy: functions at most 60 lines, no `todo!` / `unimplemented!` / `dbg!`, warnings are errors.
