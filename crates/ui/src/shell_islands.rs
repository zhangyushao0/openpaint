//! The canvas screen's top islands: Gallery, Actions, Adjustments, Selection and Transform at
//! the left (Procreate's order);
//! Brush, Smudge, Eraser, Layers and the colour at the right; the notice pill left of the right
//! one. On a practice sheet All tools follows Gallery, and while the exercise's own tools show
//! (`practice_tools.rs`) the left island holds only those two and there is no right one. The
//! calls that hand the panels what the renderer drew (brush previews, layer pictures).

use openpaint_app::{BrushKey, EditorAction, GalleryIntent, Intent, SessionProjection};
use openpaint_core::LayerId;
use openpaint_render::RgbaImage;

use super::Shell;
use crate::brush_strip::StripStrokes;
use crate::components::{Place, ScreenArea, island, text_button};
use crate::design::SPACE;
use crate::layer_panel::picture_pixels;
use crate::notices::notice_pill;
use crate::panels::Panel;
use crate::practice_tools;
use crate::text as t;

impl Shell {
    /// The left island (Gallery, Actions, Selection, Transform), the right one (Brush, Smudge,
    /// Eraser, Layers, the colour) and, left of it, the notice pill; on a practice sheet with its
    /// own tools only Gallery and All tools. Opening one panel closes the others.
    pub(super) fn top_islands(&mut self, ctx: &egui::Context, projection: &SessionProjection) {
        let own = practice_tools::own_tools(projection);
        let ((), left) = island(ctx, "left island", Place::top_left(), |ui| {
            let enabled = projection.availability(EditorAction::Gallery).is_offered();
            if text_button(ui, t::GALLERY, enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
            if let Some(practice) = projection.practice() {
                practice_tools::all_tools(ui, practice, &mut self.taps);
            }
            if !own {
                self.full_left(ui, projection);
            }
        });
        // The exercise's own tools lie beside its frame (`practice_tools.rs`).
        let right = (!own).then(|| self.full_right(ctx, projection));
        // On a practice sheet its bar holds the band's middle: the notice goes under it.
        let place = if projection.practice().is_some() {
            Place::middle_unless(true)
        } else {
            Place::top_right(right.map_or(0.0, |right| right.width()) + SPACE)
        };
        let shown = (place, &mut self.reset_notice);
        let notice = notice_pill(ctx, projection, shown, &mut self.taps);
        let ppp = self.pixels_per_point;
        self.islands = [Some(left), right, notice]
            .into_iter()
            .flatten()
            .map(|rect| ScreenArea::from_points(rect, ppp))
            .collect();
    }

    /// The full interface's part of the left island: Actions, Adjustments, Selection, Transform.
    fn full_left(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) {
        let panels = &mut self.panels;
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
    }

    /// The full interface's right island (Brush, Smudge, Eraser, Layers, the colour); where it
    /// lies, in points.
    fn full_right(&mut self, ctx: &egui::Context, projection: &SessionProjection) -> egui::Rect {
        let panels = &mut self.panels;
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
        right
    }

    /// The brush picker's preview of each brush, drawn by the renderer at the screen's density
    /// (`design::BRUSH_PREVIEW` points); loaded as textures by the next pass.
    pub fn set_brush_previews(&mut self, previews: Vec<(BrushKey, RgbaImage)>) {
        self.panels.brushes.set_previews(previews);
    }

    /// One brush's preview, made again since the brush changed or was copied.
    pub fn set_brush_picture(&mut self, key: BrushKey, image: RgbaImage) {
        self.panels.brushes.set_picture(key, image);
    }

    /// Whether the brush list shows: only then does it want pictures of changed brushes.
    #[must_use]
    pub const fn brushes_listed(&self) -> bool {
        self.panels.brushes.listed()
    }

    /// While the brush editor shows: its preview strip's size in pixels and the pen's strokes
    /// on it, for the renderer to draw with the brush as set now.
    #[must_use]
    pub fn strip(&self) -> Option<StripStrokes> {
        self.panels.brushes.strip()
    }

    /// The renderer's picture of the strip (opaque RGBA, its size in pixels), shown from the
    /// next pass on.
    pub fn set_strip_picture(&mut self, image: RgbaImage) {
        self.panels.brushes.set_strip_picture(image);
    }

    /// Whether the layer panel is open: only then does it want pictures of its layers.
    #[must_use]
    pub const fn layers_open(&self) -> bool {
        self.panels.layers.is_open()
    }

    /// The largest layer picture the panel shows, in pixels at the last pass's density: the
    /// canvas fits inside it.
    #[must_use]
    pub fn layer_picture_pixels(&self) -> [u32; 2] {
        picture_pixels(self.pixels_per_point)
    }

    /// The picture of `layer`'s paint, or with `mask` its mask's grey, as premultiplied RGBA:
    /// its row shows it from the next pass on, until a newer one comes.
    pub fn set_layer_picture(&mut self, (layer, mask): (LayerId, bool), image: RgbaImage) {
        self.panels.layers.set_picture((layer, mask), image);
    }

    /// Lets every layer picture go (the GPU memory budget's last step); rows show none
    /// until new ones come.
    pub fn forget_layer_pictures(&mut self) {
        self.panels.layers.drop_pictures();
    }

    /// Bytes of the layer pictures' textures.
    #[must_use]
    pub fn picture_bytes(&self) -> u64 {
        self.panels.layers.picture_bytes()
    }

    /// The GPU device was lost and made again: the notice pill says the display was
    /// reset until OK is tapped or the pen starts a stroke.
    pub fn display_reset(&mut self) {
        self.reset_notice = true;
    }
}
