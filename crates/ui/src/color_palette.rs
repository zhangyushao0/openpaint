//! The colour panel's palette and hex entry (S41). The palette is one row-wrapped set of saved
//! swatches under the recent colours (Procreate keeps many palettes; one is enough here): the
//! plus keeps the current colour at its end, a tap on a swatch paints with it, and Edit turns
//! taps into removing until Done. The hex field in the header shows the current colour as
//! `#RRGGBB`; typed into and ended with Enter (the keyboard's Done), a valid colour becomes the
//! colour, anything else says how to write one and changes nothing.

use egui::{Align, Layout, Vec2};
use openpaint_app::{BrushProjection, Intent, MAX_SWATCHES, ToolIntent, hex_of, parse_hex};
use openpaint_core::Color;

use crate::components::{hint, icon_button, swatch, text_button, text_field};
use crate::design::{INK_SECONDARY, Icon, RECENT_SWATCH, SPACE, TOUCH_TARGET};

/// Swatches in one row of the panel.
const PER_ROW: usize = 8;
/// The hex field's width in points: `#RRGGBB` and its margins.
const HEX_WIDTH: f32 = 136.0;

/// The palette's and the hex field's UI-local state.
#[derive(Debug, Default)]
pub(crate) struct PaletteState {
    /// Taps on swatches remove them.
    editing: bool,
    /// The hex field's text: the current colour's while the field has no focus.
    hex: String,
    /// The field gained the focus last pass: select its text, so typing replaces it.
    select: bool,
    /// The last text ended with Enter was no colour.
    wrong: bool,
}

impl PaletteState {
    pub(crate) fn close(&mut self) {
        self.editing = false;
        self.wrong = false;
    }

    /// The hex field at the right of the header row.
    pub(crate) fn hex_field(&mut self, ui: &mut egui::Ui, color: Color, taps: &mut Vec<Intent>) {
        let size = Vec2::new(HEX_WIDTH, TOUCH_TARGET);
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            let id = egui::Id::new(("text field", "Hex"));
            if !ui.memory(|memory| memory.has_focus(id)) {
                self.hex = hex_of(color);
            }
            let field = text_field(ui, "Hex", &mut self.hex, 7, self.select);
            self.select = field.response.gained_focus();
            if field.response.gained_focus() {
                self.wrong = false;
            }
            if field.submitted {
                match parse_hex(&self.hex) {
                    Some(color) => {
                        push(
                            taps,
                            Intent::Tool(ToolIntent::Color {
                                color,
                                settled: true,
                            }),
                        );
                        self.wrong = false;
                    }
                    None => self.wrong = true,
                }
            }
        });
    }

    /// A line under the header after a text that is no colour.
    pub(crate) fn hex_hint(&self, ui: &mut egui::Ui) {
        if self.wrong {
            hint(ui, "Type a colour as #RRGGBB.");
        }
    }

    /// The palette: its header row (Palette, the plus, Edit or Done), then the swatches.
    pub(crate) fn palette(
        &mut self,
        ui: &mut egui::Ui,
        brush: &BrushProjection,
        taps: &mut Vec<Intent>,
    ) {
        let swatches = &brush.swatches;
        if swatches.is_empty() {
            self.editing = false;
        }
        ui.horizontal(|ui| {
            ui.set_height(TOUCH_TARGET);
            ui.add_space(2.0 * SPACE);
            ui.colored_label(INK_SECONDARY, "Palette");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let word = if self.editing { "Done" } else { "Edit" };
                if text_button(ui, word, !swatches.is_empty()).clicked() {
                    self.editing = !self.editing;
                }
                let room = swatches.len() < MAX_SWATCHES && !swatches.contains(&brush.color);
                let add = icon_button(ui, Icon::Add, "Add to palette", room && !self.editing);
                if add.clicked() {
                    push(taps, Intent::Tool(ToolIntent::AddSwatch));
                }
            });
        });
        if swatches.is_empty() {
            hint(ui, "Tap + to keep the colour here.");
            return;
        }
        if self.editing {
            hint(ui, "Tap a colour to remove it.");
        }
        for row in swatches.chunks(PER_ROW) {
            ui.horizontal(|ui| {
                ui.set_height(RECENT_SWATCH);
                for color in row {
                    let current = *color == brush.color && !self.editing;
                    if swatch(ui, *color, "Palette colour", RECENT_SWATCH, current).clicked() {
                        let intent = if self.editing {
                            ToolIntent::RemoveSwatch(*color)
                        } else {
                            ToolIntent::Color {
                                color: *color,
                                settled: true,
                            }
                        };
                        push(taps, Intent::Tool(intent));
                    }
                }
            });
        }
    }
}

/// egui may run a pass twice; one change is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}
