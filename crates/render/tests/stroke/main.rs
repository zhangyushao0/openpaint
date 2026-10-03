//! GPU tests of stroke evaluation: a golden hash for a fixed stroke, incremental evaluation
//! equal to cold evaluation bit for bit, layer compositing order, thumbnails that match, and the
//! screen showing the canvas through the view.

mod compositing;
mod erase;
mod fill;
mod fixture;
mod golden;
mod incremental;
mod screen;
mod thumbnail;
