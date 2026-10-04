//! GPU tests of stroke evaluation: a golden hash for a fixed stroke, incremental evaluation
//! equal to cold evaluation bit for bit, accumulating deposits, layer compositing order,
//! thumbnails that match, and the screen showing the canvas through the view.

mod blending;
mod compositing;
mod deposit;
mod erase;
mod fill;
mod fixture;
mod golden;
mod groups;
mod incremental;
mod insert;
mod masks;
mod screen;
mod selection;
mod smudge;
mod thumbnail;
mod transform;
