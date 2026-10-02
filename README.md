# OpenPaint

Android 优先、手写笔优先的二维绘画软件，Rust + wgpu + egui。目标是在一台平板上独立完成从草稿到交付的插画流程。

## 开始

```bash
cargo gate
```

`cargo gate` 跑 fmt、clippy、全部测试和仓库检查，退出码就是结论。日常迭代用 `cargo check -p <crate>` 和 `cargo test -p <crate>`。

## 文档

- [AGENTS.md](AGENTS.md)：工作方式与规则。
- [docs/STATUS.md](docs/STATUS.md)：现在什么能用。
- [docs/ROADMAP.md](docs/ROADMAP.md)：下一刀做什么。
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)：模块怎么切。
- [docs/DECISIONS.md](docs/DECISIONS.md)：为什么这样做。

只有这四份文档加本文件和 AGENTS.md；规则由 `cargo repo-check` 强制。
