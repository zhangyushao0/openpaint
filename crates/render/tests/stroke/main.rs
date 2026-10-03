//! GPU tests of stroke evaluation: a golden hash for a fixed stroke, incremental evaluation
//! equal to cold evaluation bit for bit, layer compositing order, and thumbnails that match.

mod compositing;
mod erase;
mod fixture;
mod golden;
mod incremental;
mod thumbnail;
