//! Shared widgets. Every button, slider and panel is one of these; screens compose them and
//! never style widgets inline (ARCHITECTURE section 10).

mod canvas_frame;
mod icon_button;
mod list_row;
mod notice;
mod popover;
mod slider;
mod text_button;
mod top_bar;

pub use canvas_frame::{CanvasInput, ScreenArea, canvas_frame};
pub use icon_button::{icon_button, icon_toggle};
pub use list_row::{ListRow, RowResponse, list_row};
pub use notice::notice;
pub use popover::popover;
pub use slider::{SliderChange, slider};
pub use text_button::text_button;
pub use top_bar::{TOP_BAR_HEIGHT, top_bar};
