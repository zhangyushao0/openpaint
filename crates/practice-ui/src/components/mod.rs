//! Practice's own widgets, composed of the ui's (`openpaint_ui::components`) and drawn as they
//! are: the Today card, the road of a stage's sessions, the seals, a kind's tile, the colour table
//! and the radar.

mod colour_table;
mod kind_tile;
mod radar;
mod road;
mod seal;
mod today_card;

pub(crate) use colour_table::colour_table;
pub(crate) use kind_tile::{KindTile, kind_tile};
pub(crate) use radar::{RadarAxis, radar};
pub(crate) use road::{Dot, curve, curve_y, inked, paint_dot, paint_stretch, pencilled};
pub(crate) use seal::{paint_seal, seal_turn};
pub(crate) use today_card::{TodayCard, today_card};
