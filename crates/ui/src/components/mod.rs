//! Shared widgets. Every button, slider and panel is one of these; screens compose them and
//! never style widgets inline (ARCHITECTURE section 10).

mod artwork_card;
mod brush_row;
mod canvas_frame;
mod color_wheel;
mod dialog;
mod floating;
mod icon_button;
mod list_row;
mod menu_row;
mod notice;
mod popover;
mod side_panel;
mod side_slider;
mod slider;
mod status;
mod swatch;
mod switch_row;
mod text_button;
mod text_field;
mod top_bar;

pub use artwork_card::{ArtworkCard, CardTaps, artwork_card, card_size};
pub use brush_row::brush_row;
pub use canvas_frame::{CanvasInput, ScreenArea, canvas_frame};
pub use color_wheel::{WheelChange, color_wheel};
pub use dialog::{Placement, dialog};
pub use floating::{Floating, floating};
pub use icon_button::{icon_button, icon_toggle};
pub use list_row::{ListRow, RowResponse, list_row};
pub use menu_row::{menu_row, section};
pub use notice::{confirmation, notice};
pub use popover::{popover, popover_left};
pub use side_panel::side_panel;
pub use side_slider::{SideSlider, side_slider};
pub use slider::{SliderChange, slider};
pub use status::status;
pub use swatch::{color32, swatch};
pub use switch_row::switch_row;
pub use text_button::text_button;
pub use text_field::{FieldResponse, text_field};
pub use top_bar::{TOP_BAR_HEIGHT, top_bar};
