//! The brush picker: the Brush button in the top bar, left of Layers, opens it as a popover over
//! the canvas (Procreate's brush library, reduced to one set): header "Brushes", then one row
//! per brush with its name and a stroke it painted, the current one highlighted. A tap chooses
//! the brush; the panel stays until a contact lands on the canvas outside it, and a pen stroke
//! there draws with the brush just chosen.

use std::fmt;

use openpaint_app::{BrushIntent, BrushKey, Intent, SessionProjection};
use openpaint_render::RgbaImage;

use crate::components::{ScreenArea, brush_row, icon_toggle, popover};
use crate::design::{CHROME, Icon, ON_CHROME, SPACE, TOUCH_TARGET};

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

    /// The Brush button, inside the top bar. Returns whether it opened the panel.
    pub(crate) fn button(&mut self, ui: &mut egui::Ui) -> bool {
        if icon_toggle(ui, Icon::Brush, "Brushes", self.open).clicked() {
            self.open = !self.open;
            return self.open;
        }
        false
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
                ui.heading("Brushes");
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

/// A preview painted in `ON_CHROME` on `CHROME` as that ink with the paint's strength as alpha,
/// so it sits on any row face. The evaluator blends on encoded values: a pixel's share of the way
/// from the grey to the ink is that strength.
fn ink(image: &RgbaImage) -> Vec<u8> {
    let (paper, ink) = (CHROME.r(), ON_CHROME.r());
    let span = f32::from(ink - paper);
    let (pixels, _) = image.pixels().as_chunks::<4>();
    let mut out = Vec::with_capacity(image.pixels().len());
    for [value, ..] in pixels {
        let strength = f32::from(value.saturating_sub(paper)) / span;
        let alpha = (strength * 255.0).round().min(255.0) as u8;
        out.extend_from_slice(&[ink, ink, ink, alpha]);
    }
    out
}
