//! The canvas screen's top islands: Gallery, Actions, Adjustments, Selection and Transform at
//! the left (Procreate's order);
//! Brush, Smudge, Eraser, Layers and the colour at the right; the notice pill left of the right
//! one.

use openpaint_app::{EditorAction, GalleryIntent, Intent, SessionProjection};

use super::Shell;
use crate::components::{Place, ScreenArea, island, text_button};
use crate::design::SPACE;
use crate::notices::notice_pill;
use crate::panels::Panel;

impl Shell {
    /// The left island (Gallery, Actions, Selection, Transform), the right one (Brush, Smudge,
    /// Eraser, Layers, the colour) and, left of it, the notice pill. Opening one panel closes
    /// the others.
    pub(super) fn top_islands(&mut self, ctx: &egui::Context, projection: &SessionProjection) {
        let panels = &mut self.panels;
        let ((), left) = island(ctx, "left island", Place::top_left(), |ui| {
            let enabled = projection.availability(EditorAction::Gallery).is_offered();
            if text_button(ui, "Gallery", enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
            if panels.actions.button(ui) {
                panels.close_but(Panel::Actions);
                self.adjust.close_menu();
            }
            if self.adjust.button(ui, projection, &mut self.taps) {
                panels.close();
            }
            if self.selection.button(ui, projection, &mut self.taps) {
                panels.close();
                self.adjust.close_menu();
            }
            if self.transform.button(ui, projection, &mut self.taps) {
                panels.close();
                self.adjust.close_menu();
            }
        });
        let ((), right) = island(ctx, "right island", Place::top_right(0.0), |ui| {
            if panels.brushes.buttons(ui, projection, &mut self.taps) {
                panels.close_but(Panel::Brushes);
                self.adjust.close_menu();
            }
            if panels.layers.button(ui) {
                panels.close_but(Panel::Layers);
                self.adjust.close_menu();
            }
            if panels.colors.button(ui, projection) {
                panels.close_but(Panel::Colors);
                self.adjust.close_menu();
            }
        });
        let place = Place::top_right(right.width() + SPACE);
        let notice = notice_pill(ctx, projection, place, &mut self.taps);
        let ppp = self.pixels_per_point;
        self.islands = [Some(left), Some(right), notice]
            .into_iter()
            .flatten()
            .map(|rect| ScreenArea::from_points(rect, ppp))
            .collect();
    }
}
