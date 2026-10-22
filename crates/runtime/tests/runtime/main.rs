//! The runtime's GPU tests through the whole frame loop, one binary on one device
//! (`support::gpu`). The tests that change their device for everything on it (its frame budget,
//! its memory budget's counts, its loss) run in binaries of their own beside this directory.

mod support;

mod adjust;
mod artwork_files;
mod autosave;
mod bench_app;
mod brushes;
mod drawing;
mod export;
mod fill;
mod fill_edges;
mod gallery;
mod history;
mod interrupted;
mod layers;
mod other_format;
mod pacing;
mod palm;
mod quickshape;
mod shell_frame;
mod tools;
