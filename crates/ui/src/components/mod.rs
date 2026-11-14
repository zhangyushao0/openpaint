//! Shared widgets. Every button, slider and panel is one of these; screens compose them and
//! never style widgets inline (ARCHITECTURE section 10). A mode's screens compose them too.
//!
//! The one for each job: `island` (and `pill`, what comes and goes in the top band;
//! `Place::middle_unless` puts a bar in the band's middle unless another holds it), `popover` (grown out
//! of its island button, usable from the frame it opens), `icon_button`, `icon_toggle`,
//! `text_button` (a command: a bare word in ink with the press wash; `pill_button` in a pill),
//! `choices` (mutually exclusive items owning their one mark: `text_choice`, `text_choice_wide`,
//! `icon_choice`; `word_columns` lays words in two columns with a `Lead`), `divider`, `hint` (a
//! quieter line), `command_row`, `menu_row`, `label_row`, `panel_heading` / `panel_heading_with`,
//! `list_row` (the chosen row outlined, an icon at its end; `picture_row`, a picture small beside
//! its title and a line), `slider`, `side_slider`, `setting_slider`, `switch_row`, `swatch`,
//! `color_wheel` (`color_compare`, two colours as a picture; `wheel_picture`, a demonstration's
//! small wheel), `curve_editor`, `preview_strip`, `brush_row`, `notice`, `status`, `dialog`
//! (`dialog_of_width` for a page of two columns, `naming_dialog` with `Naming`, `delete_dialog`),
//! `text_field`, `artwork_card`, `card` (`card_text`, `card_columns`, `card_step`), `floating`,
//! `loupe`, `fill_bar`, `view_status`, `canvas_frame`, `picture`, `paper`, `chrome::declare`; the
//! motion they share in `animate.rs`, `chrome*.rs`, `surface.rs`, `carry.rs`, `view_settle.rs`
//! (D44). Every one registers its name and rectangle in the `ControlMap` (`controls.rs`); taps go
//! out through the crate's `taps.rs` (`push_once`, `push_brush`).

mod animate;
mod artwork_card;
mod brush_row;
mod canvas_frame;
mod card;
pub mod carry;
mod choices;
pub mod chrome;
mod chrome_frame;
mod color_wheel;
pub mod controls;
mod curve_editor;
mod dialog;
mod fill_bar;
mod floating;
mod icon_button;
mod island;
mod list_row;
mod loupe;
mod menu_row;
mod notice;
mod paper;
mod picture;
mod popover;
mod preview_strip;
mod setting_slider;
mod side_panel;
mod side_slider;
mod slider;
mod status;
mod surface;
mod swatch;
mod switch_row;
mod text_button;
mod text_field;
mod view_settle;
mod view_status;

pub use animate::{animate, animate_from, keep_moving, moving};
pub use artwork_card::{ArtworkCard, CardTaps, artwork_card, card_size, paint_sheet};
pub use brush_row::{BRUSH_ROW, BrushRowView, brush_row};
pub use canvas_frame::{CanvasArea, ScreenArea, canvas_frame, canvas_point, screen_point};
pub use card::{card, card_columns, card_step, card_text};
pub use choices::{Choices, choices};
pub use chrome::{Source, ring_of};
pub use color_wheel::{WheelChange, color_compare, color_wheel, wheel_picture};
pub use controls::{Control, ControlMap, Unresolved};
pub use curve_editor::{CurveChange, DynamicsCurve, curve_editor, dynamics_editor};
pub use dialog::{
    Naming, Placement, answer_row, delete_dialog, dialog, dialog_of_width, naming_dialog,
};
pub use fill_bar::{carried_drop, fill_bar};
pub use floating::{Floating, FloatingLayout, MAT, floating};
pub use icon_button::{icon_button, icon_choice, icon_toggle};
pub use island::{ISLAND_TOP, Place, TOP_BAND, UNDER_ISLANDS, island, pill};
pub use list_row::{
    ListRow, PICTURE_ROW, PictureRow, RowPicture, RowResponse, fitted, list_row, paint_picture,
    picture_row,
};
pub use loupe::{LOUPE, LOUPE_OFFSET, loupe, loupe_origin};
pub use menu_row::{
    command_row, hint, label_row, menu_row, panel_heading, panel_heading_with, section,
};
pub use notice::{confirmation, notice};
pub use paper::paper;
pub use picture::{Alpha, picture_texture};
pub use popover::{Opener, popover, popover_left};
pub use preview_strip::{StripResponse, preview_strip};
pub use setting_slider::{SettingSlider, setting_slider};
pub use side_panel::side_panel;
pub use side_slider::{Preview, SideSlider, side_slider, side_slider_height};
pub use slider::{SliderChange, slider, value_row};
pub use status::status;
pub use swatch::{color32, swatch};
pub use switch_row::switch_row;
pub use text_button::{
    Lead, divider, heading_button, pill_button, text_button, text_choice, text_choice_wide,
    word_columns,
};
pub use text_field::{FieldResponse, field_id, text_field};
pub(crate) use view_settle::ViewSettle;
pub use view_status::{view_status, view_text};
