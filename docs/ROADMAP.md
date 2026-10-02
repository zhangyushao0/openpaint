# 路线图

## 产品定义

Android 优先、手写笔优先的二维绘画软件，在一台平板上独立完成：草稿 → 线稿 → 分层铺色 → 涂抹塑造 → 选区修订 → 蒙版叠色 → 调色与成品 → 导出 → 中断后继续。交互默认参考 Procreate 的用户心智，再按 Android 平板取舍，不复制外观与素材。

非目标（G0–G5）：网络、账号、云同步、社区、动画、3D、多端、插件系统、桌面产品。

## 规则

- 一刀 = 一个会话能做完、能独立验证和回滚的纵向切片。按编号顺序做，不跳。
- 每刀写明目标、涉及 crate、完成标准。完成标准必须可执行或可观察，不写"更好"。
- 刀做完：STATUS 改写、本刀移到文末"已完成"（一行）、必要时 DECISIONS 加条目。
- 阶段出口需要作者在平板上人工判断的项，写 `pass` / `fail` / `not_run`，不用开发机结果替代。
- 范围外的发现写进"候选"，不顺手做。

## G0 骨架

**S0 桌面宿主与依赖**（desktop, render, ui, runtime）
- 加入并固定版本：wgpu、egui、egui-wgpu、winit（版本写进 DECISIONS）。
- desktop 开一个窗口：egui 顶栏 + 画布区域由 render 的 adapter 用 wgpu 清屏；窗口尺寸变化不崩。
- 完成：`cargo run -p openpaint-desktop` 可见窗口；`cargo gate` 热缓存 < 60 s；没有新的 markdown。

**S1 Android 宿主**（android, runtime）
- 先决定宿主形态（`android-activity` + winit，或自写 Activity + JNI），记 DECISIONS；加 `aarch64-linux-android` target、`cargo-ndk`、最小 Gradle 工程、`cargo android-build` 别名。
- 完成：APK 装到作者平板，显示与桌面相同的界面；手写笔事件带压力到达 Rust 并打印；前后台切换、旋转不崩。作者确认安装成功。

## G1 画一条线的闭环

出口：平板上用笔压画线、撤销、杀掉应用重开，线还在；笔到像素延迟有实测数字；作者在平板判断基础笔感 `pass`/`fail`。

**S2 core 作品值**（core）
- `Artwork(Arc<State>)`、`Layer`、`Stroke`、`StrokeSample`、ID；`LayerCommand`（新建、删除、重排、属性）；`apply` → `Applied{artwork, change}`；`StrokeSession` begin/extend/finish/cancel；`from_storage` 全量校验；`CoreError`。
- 完成：每个命令有成功与错误用例；没有公开可变 setter；`cargo test -p openpaint-core` 通过；core 无外部依赖。

**S3 渲染核心**（render, desktop）
- `Core`（设备、队列、管线）、`FrameRequest`、`Scene`/`Plan`、笔画几何 → 圆头连续笔刷的 coverage 着色器、`RegionEvaluator` 唯一入口、读回。
- 完成：GPU 测试：固定笔画渲染结果的像素哈希固定；增量求值 = 冷求值逐位一致；desktop 显示一条写死的笔画；render 不含 `egui`（adapter 除外）。

**S4 输入到笔画**（runtime, app, ui, desktop）
- 规范化器：winit 笔/鼠标事件 → `InputBatch`（压力、倾斜、时间戳）；app `Session` + `Intent::Stroke`；UI 画布把触点交给会话；进行中笔画单独送渲染。
- 完成：桌面用鼠标或笔连续绘画；一笔 10,000 样本时帧时间 < 8 ms（开发机）；规范化器有单元测试。

**S5 历史**（app, ui）
- `History` 版本栈 + 字节预算；Undo/Redo 意图；顶栏按钮；投影给出可用性。
- 完成：撤销/重做 20 步往返作品相等；预算裁剪有测试；UI 按钮按可用性禁用。

**S6 存储与自动保存**（store, runtime, desktop）
- fjall 作品库；`Project` DTO schema 1 与冻结样本；实现 `ProjectStore`；笔画结束后 worker 自动保存；启动打开上次作品；保存失败提示与重试。
- 完成：桌面画几笔、杀进程、重开后内容一致；样本往返测试；门禁 < 60 s。

**S7 Android 真机闭环**（android, runtime）
- 接通 S2–S6；Android 触摸/笔事件进同一规范化器；掌触拒绝基础版；笔到像素延迟打点。
- 完成：平板画线、撤销、重开；延迟数字写进 STATUS；作者判断笔感。G1 出口。

## G2 作品与图层

出口：作者在平板完成一张线稿加平涂（W1）。

**S8 桌面自动回归**（devtools 新 crate, desktop, xtask）
- 无头场景运行器：JSON 步骤（点、拖、笔画、撤销、保存、重开）+ 像素检查点（读回哈希）；`cargo scenario <name>`；门禁跑 `smoke` 场景。
- 完成：`smoke` 覆盖画线、撤销、保存重开，< 5 s；检查点失败时输出差异图路径（放 `artifacts/`，不跟踪）。

**S9 图层**（core, app, ui, render, store）
- 多图层：新建、删除、重排、显隐、不透明度、选中；Normal 合成；图层面板；schema 2 与迁移。
- 完成：合成顺序 GPU 测试；面板操作走意图且可撤销；schema 1 样本升级通过。

**S10 作品库**（app, store, ui, render）
- Gallery：列表、新建、打开、重命名、复制、删除、缩略图（导出小图）；每作品自动保存。
- 完成：桌面场景覆盖新建、改名、删除、重开；缩略图与作品一致。

**S11 笔刷配方 v1**（core, render, ui, store）
- `BrushRecipe` TOML：形状、尺寸/不透明度随压力与速度的曲线、颗粒；内置铅笔、墨线笔、软笔；笔刷选择器与大小、不透明度滑杆；笔画保存配方快照。
- 完成：三支笔的 GPU 哈希测试；配方解析错误有类型；改内置配方不改变旧作品渲染（测试）。

**S12 橡皮、Alpha Lock、颜色**（core, app, ui, render）
- 橡皮擦用同一套配方；Alpha Lock；HSV 色轮与最近颜色；吸管经 render 取色。
- 完成：擦除与 Alpha Lock 的 GPU 测试；取色与导出像素一致。

**S13 导出与参考图**（app, store, render, ui, android）
- PNG/JPEG 导出走 `RegionEvaluator` 读回、在 worker 执行；参考图浮窗（移动、缩放、取色）；Android 系统选图与分享在宿主。
- 完成：导出图与屏幕逐像素一致（测试）；平板选图、导出、分享可用。G2 出口由作者判断。

## G3 编辑与塑造

出口：作者在平板完成厚涂明暗（W2）与复杂构图修订（W3）。

**S14 选区**（core, app, ui, render）：手绘与矩形选区；选区内绘画；取消选区；选区是工具状态不进作品。完成：选区外像素不变的 GPU 测试。
**S15 变换**（core, app, ui, render）：移动、缩放、旋转图层或选区，预览走渲染器，提交为 `RegionEdit`。完成：预览与提交结果一致。
**S16 填色**（raster 新 crate, core, app, ui）：连通域填色、容差、选区相交。完成：软边、断线、细孔的用例。
**S17 涂抹**（core, render, ui）：涂抹笔刷拖动当前图层颜色；增量残差分块；增量 = 冷求值逐位一致。完成：GPU 一致性测试；平板涂抹 ≥ 60 Hz。
**S18 图层组、剪贴蒙版、混合模式**（core, app, ui, render, store）：单层组、剪贴链、Multiply/Screen/Overlay；schema 升级。完成：合成公式 GPU 测试。
**S19 图层蒙版**（core, app, ui, render, store）：添加、停用、反转、清空、删除蒙版及撤销；蒙版绘制。完成：蒙版与不透明度、剪贴、组叠加的 GPU 测试。

## G4 色彩、调整、交换

**S20 色彩管理**：P3 显示、导入解释、导出转换分开定义。**S21 调整**：色相/饱和度/亮度、曲线、模糊、锐化，各有预览、取消、历史。**S22 工程交换**：自有格式完整原稿；OpenRaster 导入导出。**S23 作品库管理**：搜索、版本恢复、外部备份。
出口：作者完成修改、恢复与交付（W5）。

## G5 稳定与发布

设备分级、长会话、前后台、存储不足、升级失败；GPU 设备丢失恢复与显存预算；真机冒烟场景 ≤ 20 s；签名发行与版本策略。出口：声明支持的设备矩阵通过，没有已知作品损坏或静默丢失。

## G6 稳定后的独立扩展

湿画材质、双笔刷、笔刷生态、动画、多端、云同步、插件：各自立项，不自动进入主线。

## 候选

（范围外的发现写在这里，一行一条，带日期。）

## 已完成

（一刀一行：编号、日期、提交。满 30 行移到 `docs/archive/ROADMAP-<YYYY-MM>.md`。）
