//! Practice's own widgets, composed of the ui's (`openpaint_ui::components`) and drawn as they
//! are: a skill's tile with its dots, a result's mark, the seals and the colour table.

mod colour_table;
mod marks;
mod seal;
mod skill_tile;

pub(crate) use colour_table::colour_table;
pub(crate) use marks::{DOTS, paint_dots, paint_result};
pub(crate) use seal::{paint_seal, seal_turn};
pub(crate) use skill_tile::{SkillTile, skill_tile};
