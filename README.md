# OpenPaint

An Android-first, stylus-first 2D painting app in Rust with wgpu and egui. The goal is to take an illustration from sketch to delivery on one tablet.

## Getting started

```bash
cargo gate
```

`cargo gate` runs fmt, clippy, every test and the repository checks; the exit code is the verdict. Day-to-day iteration uses `cargo check -p <crate>` and `cargo test -p <crate>`.

## Documents

- [AGENTS.md](AGENTS.md): how work is done here, and the rules.
- [docs/STATUS.md](docs/STATUS.md): what works now.
- [docs/ROADMAP.md](docs/ROADMAP.md): what the next slice is.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): how the modules are cut.
- [docs/DECISIONS.md](docs/DECISIONS.md): why things are the way they are.

These four documents plus this file and AGENTS.md are the only markdown in the repository; `cargo repo-check` enforces it.
