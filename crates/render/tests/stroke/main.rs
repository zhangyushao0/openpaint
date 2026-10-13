//! GPU tests of stroke evaluation: a golden hash for a fixed stroke, incremental evaluation
//! equal to cold evaluation bit for bit, accumulating deposits, layer compositing order,
//! thumbnails that match, and the screen showing the canvas through the view.

mod blending;
mod compositing;
mod copy_paste;
mod curves;
mod deposit;
mod erase;
mod fill;
mod fill_edges;
mod filter_reference;
mod filters;
mod fixture;
mod golden;
mod groups;
mod incremental;
mod insert;
mod masks;
mod merge;
mod merge_after;
mod records;
mod records_scale;
mod scale;
mod screen;
mod selection;
mod smudge;
mod smudge_room;
mod snapshot;
mod stamps;
mod stamps_scale;
mod taper_tilt;
mod thumbnail;
mod transform;
mod view;
