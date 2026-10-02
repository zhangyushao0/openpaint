# OpenPaint 仓库规则

本文件是 agent 与维护者共用的入口，规定默认工作方式。作者在当前任务里的明确指示优先于本文件。

## 读取顺序

1. `docs/STATUS.md`：现在什么能用、什么经过验证。
2. `docs/ROADMAP.md`：找到"下一刀"，只做这一刀。
3. `docs/ARCHITECTURE.md`：只读与本刀相关的 crate 一节。
4. 受影响的源码。文件都在 400 行以内，用 Read 读整个文件；用 Grep 定位。
5. `docs/DECISIONS.md`：只在要改变已有决定、或需要知道"为什么"时读。

不要从源码重建全局认识，模块地图在 ARCHITECTURE；不要翻 git 历史找进度，进度在 STATUS。

## 一个会话一刀

- 一个会话只做 ROADMAP 里的一刀。门禁通过、提交、文档更新后就结束，不自动开始下一刀。
- 开始前看 `git status --short --branch`，不碰已有的未提交修改。
- 范围由 ROADMAP 规定。范围外的问题写进 STATUS 的"已知限制"或 ROADMAP 的候选，不顺手做。
- 可以停下来问作者。先把不依赖答案的部分做完，再列出具体选项并给出推荐。

## 读与改

- 读源码用 Read，搜索用 Grep / Glob。不用 cat、sed、grep 命令读源码。
- 改源码用 Edit；新文件和整文件重写用 Write。不写 Python、sed、heredoc 脚本改代码。
- 仓库里不允许 .py / .sh / .ps1 文件；工具代码只写在 `xtask`（Rust）。`cargo repo-check` 会拒绝。
- Bash 只用于 cargo、git 和运行程序。

## 验证三级，按顺序

| 级 | 命令 | 何时 |
| --- | --- | --- |
| 1 | `cargo check -p <crate>` | 每次连贯修改后，几秒 |
| 2 | `cargo test -p <crate> [过滤器]` | 本刀涉及的测试 |
| 3 | `cargo gate` | 提交前一次：fmt、clippy、全部测试、repo-check |

- 级 3 失败时回到级 1 修，不反复跑级 3。
- 看退出码。不要用 `| tail`、`| grep` 截断门禁输出后判断通过。
- clippy 报错就改代码，不加 `#[allow]`；确需例外写 `#[expect(lint, reason = "...")]`。
- 真机验证是单独一级，只在 ROADMAP 标明的刀上做。没做就在 STATUS 写 `not_run`，不写"通过"。

## 提交

- Conventional Commits，scope 是 crate 名，末尾带刀号：`feat(core): artwork value type (S2)`。
- 一刀通常一个提交。逐文件暂存，提交前看 `git diff --cached --stat`；禁止 `git add -A`。
- 门禁没过不提交。push、rebase、amend 只按作者明确指示做。

## 文档：四份，有上限，repo-check 强制

| 文件 | 上限 | 内容 | 不写什么 |
| --- | --- | --- | --- |
| docs/STATUS.md | 150 行 | 当前事实：能用的功能、验证结果与日期、已知限制、not_run、待作者决定 | 历史、过程、计划 |
| docs/ROADMAP.md | 250 行 | 产品定义、阶段、每一刀的目标与完成标准；已完成的刀各一行 | 实现细节、结果 |
| docs/ARCHITECTURE.md | 300 行 | crate 地图、依赖方向、核心类型、线程与错误模型、强制规则 | 迁移计划、审计发现 |
| docs/DECISIONS.md | 300 行 | 约束未来代码且不自明的决定，每条 ≤ 15 行：日期、决定、理由、影响 | 调研报告、备选方案长文 |

- 仓库里没有其他 markdown（README.md ≤ 60 行与本文件 ≤ 150 行除外）。没有 CHANGELOG，历史在 git log。没有 ADR 目录、research 目录、本地笔记目录。
- 每刀结束时：STATUS 改写成新的当前事实，删掉过时的句子，不追加；ROADMAP 把这一刀移到"已完成"，一行；只有产生了新决定才改 DECISIONS。
- 归档：某份文档超过上限时先删无用内容；仍超过，把已完成的刀或已被替代的决定整段移到 `docs/archive/<文件名>-<YYYY-MM>.md`。archive 只写不读，没有上限。
- 调研、审计、方案比较不写成文件。结论进 DECISIONS（≤ 15 行）或提交信息，过程留在会话里。

## 输出去哪里

agent 的输出应落在 `crates/` 的产品代码和测试里。以下都算浪费，默认不做：为未来功能预留的模块或抽象、通用插件系统、产品 crate 里的 feature 开关、第二条实现路径或静默回退、脚本、报告、笔记、每步一篇的记录。

- 思考档位由作者在会话开始时设定：设计与取舍用高档位，拆文件、改调用点、修编译错误用低档位。
- 子代理最多 2 个，不嵌套，不派审查员群。能在主会话做的就在主会话做。
- 代码规则（repo-check 与 clippy 强制）：.rs 文件 ≤ 400 行；函数 ≤ 60 行；依赖只能向下（见 ARCHITECTURE）；core 不得出现 GPU、UI、文件系统、线程；`Result<_, String>` 只能出现在平台边界和 xtask；产品 crate 没有 `cfg(feature)`；每个 crate 根 `#![forbid(unsafe_code)]`（`apps/android` 的 JNI 除外）。

## 交付说明

结束时用中文说明：做了什么、验证了什么（命令与结果）、没做什么、需要作者决定什么。不复述文档内容。
