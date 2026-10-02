# 架构

OpenPaint 是 Android 优先、手写笔优先的二维绘画软件，目标是在一台平板上独立完成从草稿到交付的插画流程。桌面宿主只用于开发与自动回归，不是产品承诺。

本页是设计合同：模块怎么切、依赖怎么走、核心类型长什么样、线程与错误怎么处理。代码偏离这里时，改代码或改这里并在 DECISIONS 记一条，不能两边都留。类型草图随刀落地，落地后以代码为准、本页同步。

## 1. 设计原则

1. 一个概念只有一个定义、一个所有者，所有者落到模块粒度。可变/不可变、意图/命令、持久/会话不各存一份。
2. 作品是不可变的值。编辑产生新版本，历史保存版本，撤销是版本切换。
3. 产品 crate 没有 `cfg(feature)`。诊断、实验、自动化探针放在独立 crate，只在组合根组装。
4. 渲染器只依赖 core。GPU 状态归渲染器自己的 `Core`，不寄存在 UI 框架里；"把一批区域求值到目标"只有一个入口；GPU 协议在 Rust 定义一次、注入 WGSL、用布局测试证明。
5. 按变化量付费：保存、历史预算、资产回收、投影只处理本次变化涉及的数据；UI 线程不做随作品规模增长的工作。
6. 错误是类型。`String` 错误只允许出现在 JNI、系统边界与 xtask 输出。
7. 一个用户动作只有一条产品路径：不双写、不保留简单版/高级版、不静默回退、不为未接受的功能预留模块或抽象。
8. 文件 ≤ 400 行、函数 ≤ 60 行，工具强制；`lib.rs` 只放 `mod` 与 `pub use`。

## 2. crate 地图

| crate | 职责 | 可依赖 | 不得出现 |
| --- | --- | --- | --- |
| `openpaint-core` | 作品语义：图层、笔画、命令、校验、笔刷配方的数据定义 | 无 | GPU、UI、文件系统、线程、时钟 |
| `openpaint-app` | 应用流程：意图、会话、历史、工作区、编辑器状态、能力 port（trait） | core | GPU、UI、文件系统 |
| `openpaint-store` | 本地作品库：持久格式、版本迁移、资产；实现 app 的 port | core, app | GPU、UI |
| `openpaint-render` | 派生渲染：wgpu 管线、笔画程序、区域求值、导出/取色/预览消费者 | core | 文件系统；`egui` 只在 `src/adapter/` |
| `openpaint-ui` | 共享界面：egui 面板、输入所有权、画布编排；只读 app 投影，只发意图 | core, app, render | 直接的 wgpu、文件系统 |
| `openpaint-runtime` | 帧循环、输入规范化、有界 worker、生命周期排空、启动 | core, app, store, render, ui | JNI |
| `openpaint-desktop` | 开发宿主：窗口、鼠标/笔输入、无头回归入口 | 全部 | — |
| `openpaint-android` | 唯一产品组合根：Activity 生命周期、Surface、系统 picker、JNI | 全部 | 产品规则 |
| `xtask` | 仓库工具：`cargo gate`、`cargo repo-check` | 无 | 产品代码 |

依赖只能向下（`A ← B` 表示 B 依赖 A）：

```text
core ← app ← store
core ← app ← ui ← runtime ← {desktop, android}
core ← render ← ui
store, render ← runtime
```

后续按刀新增：`openpaint-raster`（纯几何：填色连通域、mask 转选区；依赖 core）、`openpaint-devtools`（场景运行器、观察器、统计；依赖 runtime，由组合根按需组装）。新增时同步 `xtask/src/check/rules.rs` 的 `LAYERS`。

## 3. 核心类型（目标形态）

### core

```rust
pub struct Artwork(Arc<ArtworkState>);           // 快照即值：clone O(1)，历史直接保存版本
struct ArtworkState { canvas: CanvasSize, background: Background, layers: Vec<Layer>,
                      groups: Vec<LayerGroup>, active: LayerId, ids: IdCounters }
pub struct Layer { id: LayerId, name: Name<80>, props: LayerProps, strokes: StrokeList,
                   image: Option<LayerImage>, edits: Arc<[RegionEdit]> }
pub struct LayerProps { visible: bool, opacity: f32, locked: bool, alpha_locked: bool,
                        clipping: bool, blend: BlendMode, group: Option<GroupId> }
pub struct StrokeList { chunks: Arc<[Arc<[Stroke]>]> }         // 追加只复制最后一块（≤ 256 笔）
pub struct Stroke { id: StrokeId, tool: StrokeTool, style: StrokeStyle,
                    samples: Arc<[StrokeSample]>, constraints: StrokeConstraints }
pub struct StrokeSample { position: CanvasPoint, pressure: f32, tilt: [f32; 2], time_ms: u32 }
pub struct StrokeStyle { recipe: BrushRecipe, size: f32, opacity: f32, color: Color }
pub enum ArtworkCommand { Layer(LayerCommand), Group(GroupCommand),
                          Reference(ReferenceCommand), Background(BackgroundCommand) }
pub struct Applied { pub artwork: Artwork, pub change: Change }  // Change: None | ActiveLayer | Reference | Content
impl Artwork {
    pub fn apply(&self, cmd: ArtworkCommand) -> Result<Applied, CoreError>;     // 统一 editable_layer() 前置检查
    pub fn edit_region(&self, edit: RegionEdit) -> Result<Applied, CoreError>;   // 选区、填色、变换的提交
    pub fn begin_stroke(&self, begin: BeginStroke) -> Result<StrokeSession, CoreError>;
    pub fn from_storage(parts: ArtworkParts) -> Result<Self, CoreError>;        // 唯一的全量校验入口
}
pub struct StrokeSession { base: Artwork, active: ActiveStroke }   // extend / finish / cancel；活动笔画不在图层里
```

没有公开的可变 setter。图层只能经 `from_storage` 或命令产生；单个值（`Stroke`、`LayerGroup`、`LayerImage`）在构造时检查自身。

### app

```rust
pub struct Project { identity: ArtworkIdentity, artwork: Artwork, view: CanvasView, editor: EditorState }  // 唯一的 store 边界类型
pub enum Intent { Stroke(StrokeIntent), Artwork(ArtworkCommand), Tool(ToolIntent), View(ViewIntent),
                  Selection(SelectionIntent), Fill(FillIntent), Transform(TransformIntent),
                  Brush(BrushIntent), History(HistoryIntent) }
pub enum AppError { Core(CoreError), Busy(BusyReason), Brush(BrushLibraryError), Port(PortError) }
pub trait ProjectStore { /* list, create, load, save, rename, duplicate, delete */ }
pub trait AssetStore   { /* import_image, load_image */ }
pub trait BrushStore   { /* brushes, change_brushes */ }
pub struct History { entries: VecDeque<Entry>, cursor: usize, total_bytes: usize }   // Entry { artwork, bytes }
pub enum Flow<Req> { Idle, Picking, Ready(Req), Running { key: u64 }, Failed(PortError) }  // 每条异步流程一个字段
impl Session { pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError>;
               pub fn projection(&self) -> Arc<SessionProjection>; }
impl SessionProjection { pub fn availability(&self, action: EditorAction) -> Availability;
                         pub fn limits(&self) -> Limits; }
```

UI 只读投影、维护 UI-local 状态、发意图。可用性与数值范围来自投影，UI 不自己拼规则。每个意图族一个模块，意图、处理函数、投影、测试放在一起。

### render

```rust
pub struct Core { gpu: Gpu, pipelines: Pipelines, pages: PageCache,
                  residuals: Mutex<smudge::Engine>, grain: GrainTexture }
pub struct Renderer { core: Arc<Core>, instances: Mutex<Instances>, sinks: Sinks }
pub struct FrameRequest { snapshot: Artwork, active: Option<ActiveStroke>, images: ImageSet,
                          view: ViewFrame, viewport_px: [u32; 2], consumer: Consumer }
pub struct RegionEvaluator<'a> { core: &'a Core, executor: &'a mut Executor, target: Target<'a> }  // 唯一的求值入口
pub enum Target<'a> { Pages(..), Readback(..), Planes(..), SmudgeTiles(..) }
pub enum Batching { Frame, Bounded { visits: u64, invocations: usize, dispatches: usize } }
#[repr(u32)] pub enum Opcode { Segment, EndLayer, Image, BeginScope, EndScope, Fill, Smudge }  // 生成 WGSL 常量块
pub enum RenderError { Admission { what: &'static str, limit: u64 }, Cancelled,
                       MissingImage(AssetId), Device(String), Invalid(&'static str) }
```

目录：`core/ error budget geom adapter/ scene/ program/ gpu/ cache/ view/ tasks/ consumers/ tests/`。`adapter/` 是唯一依赖 egui 的目录。后台消费者（导出、填色、取色）只拿 `Arc<Core>`，不碰 UI 锁。增量求值与冷求值逐位一致，用测试证明。

### runtime

```rust
pub struct OneShot<T>;                 // spawn / poll / finish；Drop 时 join
pub struct Latest<K, R, T>;            // 一个请求槽、一个在跑、一个未取
pub struct GestureJob<Job, Out>;       // 可取消、可挂起的手势作业
pub fn drain_for_lifecycle(tasks: &mut Tasks, order: &[TaskKind]) -> Result<(), RuntimeError>;  // 唯一定义排空顺序的函数
pub trait PlatformServices { fn input_batch(&mut self) -> Option<InputBatch>; fn request(&mut self, effect: Effect); }
pub trait FrameHook { /* before_ui, after_ui, takeover */ }     // devtools 挂在这里，产品 runtime 不知道它们存在
```

`run_frame` 固定步骤：排空回执 → 平台事件 → UI → 分派意图 → 提交请求 → 挂载渲染 → 排程重绘。后台写法只有三种原语，`thread::spawn` 不直接出现在产品代码里。

## 4. 输入

Android 与桌面走同一个规范化器：原始事件 → `InputBatch`（设备种类、相位、位置、压力、倾斜、时间戳）。笔势、掌触拒绝、稳定器在 app 的笔画意图里处理，不在 UI 也不在平台层。进行中的笔画作为单独一项送给渲染器，不进入作品。

## 5. 持久化

- store 拥有格式。`Project` DTO 带整数 schema 版本；每个版本一个纯数据迁移步骤和一份冻结样本，门禁跑全部样本的往返。
- 编辑器状态（当前笔刷、颜色、工具参数）与作品分开记录，损坏即退回默认，不阻止打开作品。
- 图片资产按引用回收，只处理本次变化涉及的作品；会话历史仍引用的资产不回收。
- 自动保存在笔画结束与离开编辑器时由 worker 执行；失败有明确提示与重试入口。

## 6. 笔刷与渲染

- 笔刷是声明式配方（TOML）：形状、尺寸与不透明度随压力和速度的曲线、颗粒、动态。渲染沿笔画连续逐像素求值；没有图章/dab 引擎、节点图、shader 插件。
- 笔画保存配方快照；作品重开不会静默套用同名笔刷的新行为。
- 导出始终用原生精度；屏幕先显示快速结果，后台精算完成后替换，范围与强弱不变。

## 7. 错误

`CoreError`（作品语义）、`AppError`（模式与流程）、`PortError`（外部能力）、`RenderError`、`RuntimeError`。平台层把它们翻译成用户可见的提示；`String` 只出现在 JNI、系统 API 与 xtask。

## 8. 测试

| 级 | 内容 | 何时 |
| --- | --- | --- |
| 单元 | core/app/store 纯测试；render 的 GPU 读回测试（golden 哈希、增量 = 冷求值） | `cargo test -p` |
| 门禁 | fmt、clippy、全部测试、repo-check；热缓存目标 < 60 s | 提交前 |
| 回归 | 桌面无头场景：JSON 步骤 + 像素检查点，几秒跑完 | 门禁内跑短场景 |
| 真机 | 一条 ≤ 20 s 冒烟场景；完整场景与人工笔感只在阶段出口 | ROADMAP 标明的刀 |

测试夹具放在各 crate 的 `tests/` 或独立测试 crate，不进产品 API。

## 9. 强制规则

`cargo repo-check`（`xtask/src/check/rules.rs`）：.rs ≤ 400 行；依赖方向按第 2 节表；纯度禁词按表；render 的 egui 只在 adapter；产品 crate 无 feature；crate 根 `#![forbid(unsafe_code)]`（android 除外）；markdown 只有六份加 archive；无脚本文件；无 CRLF。clippy：函数 ≤ 60 行、无 `todo!`/`unimplemented!`/`dbg!`，警告即错误。
