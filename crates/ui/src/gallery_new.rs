//! The gallery's "+" popover: "New canvas", one row per size with its pixels at the right end,
//! then "Open OpenPaint file…", which brings a file in as a new artwork.

use egui::Rect;
use openpaint_app::{CanvasPreset, GalleryIntent, Intent};

use super::{GalleryScreen, ISLAND, Sizes};
use crate::components::carry::Opening;
use crate::components::{Opener, ScreenArea, menu_row, popover, section};
use crate::design::{SPACE, TOUCH_TARGET};

/// The sizes "+" offers, top to bottom.
const PRESETS: [(CanvasPreset, &str); 4] = [
    (CanvasPreset::Screen, "Screen size"),
    (CanvasPreset::Square, "Square"),
    (CanvasPreset::A4Portrait, "A4 portrait"),
    (CanvasPreset::A4Landscape, "A4 landscape"),
];

/// What was chosen in the popover.
enum Chosen {
    Size(CanvasPreset, Rect),
    File,
}

impl GalleryScreen {
    /// The popover under "+". A row makes the artwork or opens the file picker; a tap outside
    /// closes the popover, and the tap that opened it does not. "Screen size" is the canvas
    /// area `under` the top band.
    pub(super) fn sizes_popover(
        &mut self,
        ctx: &egui::Context,
        under: Rect,
        taps: &mut Vec<Intent>,
    ) {
        let area = ScreenArea::from_points(under, ctx.pixels_per_point()).size;
        let mut chosen = None;
        let opener = Opener {
            island: ISLAND,
            button: self.plus.unwrap_or(Rect::ZERO),
        };
        let open = self.sizes != Sizes::Closed;
        let rect = popover(ctx, "new canvas", (opener, open), |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading("New canvas");
            });
            for (preset, label) in PRESETS {
                let [width, height] = preset.size(area);
                let note = format!("{width} × {height}");
                let row = menu_row(ui, label, &note, true);
                if row.clicked() {
                    chosen = Some(Chosen::Size(preset, row.rect));
                }
            }
            section(ui, "From a file");
            if menu_row(ui, "Open OpenPaint file…", "", true).clicked() {
                chosen = Some(Chosen::File);
            }
        });
        let Some(rect) = rect else {
            return;
        };
        let released_outside = ctx.input(|input| {
            input.pointer.any_released()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|at| !rect.contains(at))
        });
        match chosen {
            Some(Chosen::Size(preset, row)) => {
                taps.push(Intent::Gallery(GalleryIntent::Create(preset)));
                self.opening = Some(Opening {
                    from: row,
                    id: None,
                    picture: None,
                });
                self.sizes = Sizes::Closed;
            }
            Some(Chosen::File) => {
                taps.push(Intent::Gallery(GalleryIntent::OpenFile));
                self.sizes = Sizes::Closed;
            }
            None if self.sizes == Sizes::Opened => self.sizes = Sizes::Shown,
            None if released_outside => self.sizes = Sizes::Closed,
            None => {}
        }
    }
}
