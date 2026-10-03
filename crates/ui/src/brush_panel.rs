//! The brush picker and the tool buttons: Brush and Eraser sit in the right island before Layers
//! (Procreate's order). The current tool's button opens the picker as a popover over the canvas
//! (Procreate's brush library, reduced to one set): header "Brushes" or "Eraser", then one row
//! per brush with its name and a stroke it painted, the tool's brush outlined in ink. The eraser
//! chooses from the same brushes. A tap chooses the brush; the panel stays until a contact lands
//! on the canvas outside it, and a pen stroke there draws with the brush just chosen. Under the
//! rows, the chosen brush's StreamLine slider (0 to 100 %).

use std::fmt;

use openpaint_app::{BrushIntent, BrushKey, Intent, SessionProjection, Tool, ToolIntent};
use openpaint_render::RgbaImage;

use crate::components::{ScreenArea, brush_row, icon_toggle, popover, slider};
use crate::design::{INK, Icon, SPACE, TOUCH_TARGET};

/// The panel's UI-local state and the brushes' preview pictures.
#[derive(Default)]
pub(crate) struct BrushPanel {
    open: bool,
    /// Where the open panel lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
    /// Previews waiting for the first pass, which loads them as textures.
    pending: Vec<(BrushKey, RgbaImage)>,
    textures: Vec<(BrushKey, egui::TextureHandle)>,
}

impl fmt::Debug for BrushPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrushPanel")
            .field("open", &self.open)
            .field("area", &self.area)
            .field("previews", &(self.pending.len() + self.textures.len()))
            .finish()
    }
}

impl BrushPanel {
    pub(crate) fn set_previews(&mut self, previews: Vec<(BrushKey, RgbaImage)>) {
        self.pending = previews;
        self.textures.clear();
    }

    /// The Brush and, right of it, the Eraser button, in the right island; the one of the pen's
    /// tool carries the ink ring (Procreate's model). Tapping the other tool's button makes the
    /// pen use it; tapping the current tool's button opens or closes its brushes. Returns
    /// whether a tap opened the panel.
    pub(crate) fn buttons(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) -> bool {
        let Some(brush) = projection.brush() else {
            return false;
        };
        let mut opened = false;
        for (tool, icon, label) in [
            (Tool::Brush, Icon::Brush, "Brushes"),
            (Tool::Eraser, Icon::Eraser, "Eraser"),
        ] {
            let current = brush.tool == tool;
            if !icon_toggle(ui, icon, label, current).clicked() {
                continue;
            }
            if current {
                self.open = !self.open;
                opened = self.open;
            } else {
                self.close();
                taps.push(Intent::Tool(ToolIntent::Use(tool)));
            }
        }
        opened
    }

    /// The panel when it is open, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.load(ctx);
        let brush = projection.brush().filter(|_| self.open);
        let Some(brush) = brush else {
            self.area = None;
            return;
        };
        let textures = &self.textures;
        let rect = popover(ctx, "brushes", |ui| {
            ui.horizontal(|ui| {
                ui.set_height(TOUCH_TARGET);
                ui.add_space(2.0 * SPACE);
                ui.heading(match brush.tool {
                    Tool::Brush => "Brushes",
                    Tool::Eraser => "Eraser",
                });
            });
            for row in brush.brushes.iter() {
                let preview = textures
                    .iter()
                    .find(|(key, _)| *key == row.key)
                    .map(|(_, texture)| texture.id());
                let selected = row.key == brush.current;
                if brush_row(ui, &row.name, preview, selected).clicked() {
                    let intent = Intent::Brush(BrushIntent::Choose(row.key));
                    if taps.last() != Some(&intent) {
                        taps.push(intent);
                    }
                }
            }
            streamline(ui, brush.streamline.amount(), taps);
        });
        self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
    }

    /// Where the open panel lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
    }

    fn load(&mut self, ctx: &egui::Context) {
        for (key, image) in self.pending.drain(..) {
            let [width, height] = image.size().map(|side| side as usize);
            let picture = egui::ColorImage::from_rgba_unmultiplied([width, height], &ink(&image));
            let name = format!("brush preview {}", key.as_str());
            let texture = ctx.load_texture(name, picture, egui::TextureOptions::LINEAR);
            self.textures.push((key, texture));
        }
    }
}

/// The chosen brush's StreamLine under the rows (Procreate keeps it in the brush's settings):
/// its name and value, then a slider from off to the strongest. It applies from the next
/// stroke on and is the brush's own, per tool.
fn streamline(ui: &mut egui::Ui, amount: f32, taps: &mut Vec<Intent>) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.set_height(TOUCH_TARGET);
        ui.add_space(2.0 * SPACE);
        ui.label("StreamLine");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0 * SPACE);
            ui.label(format!("{} %", (amount * 100.0).round()));
        });
    });
    if let Some(change) = slider(ui, "StreamLine", amount, true) {
        let intent = Intent::Brush(BrushIntent::StreamLine {
            amount: change.value,
            settled: change.settled,
        });
        // egui may run a pass twice; one value is sent once.
        if taps.last() != Some(&intent) {
            taps.push(intent);
        }
    }
}

/// A preview painted in black on white as `INK` with the paint's strength as alpha, so it sits
/// on any panel. The evaluator blends on encoded values: a pixel's share of the way from white
/// to black is that strength.
fn ink(image: &RgbaImage) -> Vec<u8> {
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let mut out = Vec::with_capacity(image.pixels().len());
    for [value, ..] in pixels {
        out.extend_from_slice(&[INK.r(), INK.g(), INK.b(), 255 - value]);
    }
    out
}
