//! The reference window (Procreate's reference companion): the chosen image in a window
//! floating over the canvas, first at the lower right. Drag its title bar to move it; − and +
//! zoom the picture about the window's middle (1x to 8x); dragging the picture looks around the
//! zoomed image; × closes it (the Reference switch turns off; the image stays for this run of
//! the app). With the eyedropper on, touching the picture picks the colour under the hand while
//! it moves; lifting settles it, as on the canvas. Nothing under the window takes a contact
//! that starts on it.

use std::cell::Cell;

use egui::{Color32, Pos2, Rect, Sense, Vec2};
use openpaint_app::{Intent, ReferenceImage, ReferenceIntent, SessionProjection, ToolIntent};
use openpaint_core::Color;

use crate::components::{ScreenArea, UNDER_ISLANDS, floating, icon_button};
use crate::design::{Icon, REFERENCE_HEIGHT, REFERENCE_WIDTH, SPACE, TOUCH_TARGET};

/// The zoom steps of − and +.
const ZOOMS: [f32; 7] = [1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

#[derive(Default)]
pub(crate) struct ReferenceWindow {
    /// The image shown and its texture, replaced when another image is chosen.
    texture: Option<(ReferenceImage, egui::TextureHandle)>,
    /// The window's top-left corner in points; `None` until it first shows.
    origin: Option<Pos2>,
    /// Index into `ZOOMS`.
    zoom: usize,
    /// The image point (in image pixels) at the middle of the picture area.
    centre: Vec2,
    /// The colour picked during the touch on the picture, settled when it lifts.
    picked: Option<Color>,
    /// Where the window lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl std::fmt::Debug for ReferenceWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceWindow")
            .field("origin", &self.origin)
            .field("zoom", &ZOOMS[self.zoom])
            .field("area", &self.area)
            .finish_non_exhaustive()
    }
}

impl ReferenceWindow {
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        let transfers = projection.transfers();
        let image = transfers
            .filter(|transfers| transfers.reference_shown)
            .and_then(|transfers| transfers.reference.as_ref());
        let Some(image) = image else {
            self.area = None;
            self.picked = None;
            return;
        };
        let texture = self.load(ctx, image);
        let picking = projection.brush().is_some_and(|brush| brush.picking);
        let size = body_size(image.size());
        let screen = ctx.content_rect();
        let origin = *self.origin.get_or_insert_with(|| {
            screen.max - Vec2::new(REFERENCE_WIDTH, TOUCH_TARGET + size.y) - Vec2::splat(SPACE)
        });
        // The bar is laid out before the picture: a zoom tapped shows in the same frame.
        let zoom = Cell::new(self.zoom);
        let shown = floating(
            ctx,
            "reference",
            (origin, REFERENCE_WIDTH),
            "Reference",
            |ui| buttons(ui, &zoom),
            |ui| {
                self.zoom = zoom.get();
                self.picture(ui, (image, &texture), size, picking)
            },
        );
        if let Some(intent) = shown.body {
            push(taps, intent);
        }
        if shown.buttons {
            push(taps, Intent::Reference(ReferenceIntent::Show(false)));
        }
        let moved = origin + shown.moved;
        let height = shown.rect.height();
        self.origin = Some(Pos2::new(
            moved
                .x
                .clamp(0.0, (screen.width() - REFERENCE_WIDTH).max(0.0)),
            moved
                .y
                .clamp(UNDER_ISLANDS, (screen.height() - height).max(UNDER_ISLANDS)),
        ));
        self.area = Some(ScreenArea::from_points(shown.rect, ctx.pixels_per_point()));
    }

    /// Where the window lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }

    /// The picture area: the image fitted, zoomed about `centre`, on white. Returns the colour
    /// intent of a pick, if any.
    fn picture(
        &mut self,
        ui: &mut egui::Ui,
        (image, texture): (&ReferenceImage, &egui::TextureHandle),
        size: Vec2,
        picking: bool,
    ) -> Option<Intent> {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let pixels = Vec2::new(image.size()[0] as f32, image.size()[1] as f32);
        let scale = (size.x / pixels.x).min(size.y / pixels.y) * ZOOMS[self.zoom];
        if !picking && response.dragged() {
            self.centre -= response.drag_delta() / scale;
        }
        self.centre = clamp_centre(self.centre, pixels, size / scale);
        let shown = Rect::from_min_size(rect.center() - self.centre * scale, pixels * scale);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::WHITE);
        let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), shown, whole, Color32::WHITE);
        if !picking {
            self.picked = None;
            return None;
        }
        let settled = response.drag_stopped() || response.clicked();
        let held = response.is_pointer_button_down_on();
        if let Some(at) = response.interact_pointer_pos().filter(|_| held || settled) {
            let pixel = (at - shown.min) / scale;
            if pixel.x >= 0.0 && pixel.y >= 0.0 {
                let color = image.color_at([pixel.x as u32, pixel.y as u32]);
                self.picked = color.or(self.picked);
            }
        }
        let color = self.picked?;
        if settled {
            self.picked = None;
        } else if !held {
            return None;
        }
        Some(Intent::Tool(ToolIntent::Color { color, settled }))
    }

    /// The texture of `image`, uploaded when the image is new.
    fn load(&mut self, ctx: &egui::Context, image: &ReferenceImage) -> egui::TextureHandle {
        if let Some((shown, texture)) = &self.texture
            && shown == image
        {
            return texture.clone();
        }
        let [width, height] = image.size().map(|side| side as usize);
        let picture = egui::ColorImage::from_rgba_unmultiplied([width, height], image.pixels());
        let texture = ctx.load_texture("reference", picture, egui::TextureOptions::LINEAR);
        // A new image starts whole and centred.
        self.zoom = 0;
        self.centre = Vec2::new(image.size()[0] as f32, image.size()[1] as f32) / 2.0;
        self.texture = Some((image.clone(), texture.clone()));
        texture
    }
}

/// Close, +, − right to left; − and + step `zoom`. Returns whether close was tapped.
fn buttons(ui: &mut egui::Ui, zoom: &Cell<usize>) -> bool {
    let close = icon_button(ui, Icon::Close, "Close reference", true).clicked();
    let step = zoom.get();
    let (more, less) = (step + 1 < ZOOMS.len(), step > 0);
    if icon_button(ui, Icon::Add, "Zoom in", more).clicked() && more {
        zoom.set(step + 1);
    }
    if icon_button(ui, Icon::Minus, "Zoom out", less).clicked() && less {
        zoom.set(step - 1);
    }
    close
}

/// The picture area for an image of `size` pixels: the window's width, as tall as the image's
/// shape asks within `REFERENCE_HEIGHT`.
fn body_size([width, height]: [u32; 2]) -> Vec2 {
    let tall = REFERENCE_WIDTH * height as f32 / width.max(1) as f32;
    Vec2::new(
        REFERENCE_WIDTH,
        tall.clamp(REFERENCE_HEIGHT[0], REFERENCE_HEIGHT[1]),
    )
}

/// `centre` kept where the view (`view` image pixels large) stays on the image; an axis the view
/// covers whole stays centred.
fn clamp_centre(centre: Vec2, image: Vec2, view: Vec2) -> Vec2 {
    let axis = |c: f32, image: f32, view: f32| {
        if view >= image {
            image / 2.0
        } else {
            c.clamp(view / 2.0, image - view / 2.0)
        }
    };
    Vec2::new(
        axis(centre.x, image.x, view.x),
        axis(centre.y, image.y, view.y),
    )
}

/// egui may run a pass twice; one intent is sent once.
fn push(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_stays_on_the_image() {
        let image = Vec2::new(400.0, 200.0);
        let whole = clamp_centre(Vec2::new(10.0, 10.0), image, Vec2::new(400.0, 300.0));
        assert_eq!(whole, Vec2::new(200.0, 100.0));
        let zoomed = clamp_centre(Vec2::new(10.0, 190.0), image, Vec2::new(100.0, 50.0));
        assert_eq!(zoomed, Vec2::new(50.0, 175.0));
        assert_eq!(body_size([4000, 3000]), Vec2::new(360.0, 270.0));
        assert_eq!(body_size([1000, 4000]), Vec2::new(360.0, 420.0));
    }
}
