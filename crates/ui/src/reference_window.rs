//! The reference window (Procreate's reference companion, restrained): the chosen image in a
//! window floating over the canvas, first at the lower right, with no title bar. A tap on it
//! shows a slim bar: drag the bar to move the whole window, the corner mark to resize it, ×
//! to close it (the Reference switch turns off; the image stays for this run of the app). The
//! bar hides again after `BAR_IDLE_MS` without a touch on the window, or with a touch outside.
//! In the picture one finger drags the image about and two pinch-zoom it about their middle
//! (1x to 8x), with the canvas's dead zones; the canvas never moves with them. The pen never
//! paints here. With the eyedropper on, a touch on the picture previews the colour under it in
//! the loupe and lifting picks the last one previewed, as on the canvas. Contacts are routed
//! from the raw input (`reference_touch`): egui sees only one contact per batch.

use egui::{Color32, Pos2, Rect, Vec2};
use openpaint_app::{
    Intent, PictureGestures, ReferenceImage, ReferenceIntent, SessionProjection, ViewGesture,
};

use crate::components::{
    FloatingLayout, MAT, ScreenArea, UNDER_ISLANDS, floating, icon_button, ring_of,
};
use crate::design::{Icon, REFERENCE_HEIGHT, REFERENCE_WIDTH, SPACE};

/// How long the bar stays without a touch on the window, in milliseconds.
pub(crate) const BAR_IDLE_MS: i64 = 3000;
/// Zoom of the picture relative to the whole image in the window.
const ZOOM: [f32; 2] = [1.0, 8.0];
/// Least window size, in points; at most this share of the screen.
const LEAST: Vec2 = Vec2::new(240.0, 180.0);
const MOST: f32 = 0.85;

#[derive(Default)]
pub(crate) struct ReferenceWindow {
    /// The image shown and its texture, replaced when another image is chosen.
    texture: Option<(ReferenceImage, egui::TextureHandle)>,
    /// The window in points; `None` until it first shows.
    pub(crate) place: Option<Rect>,
    /// The picture's zoom (`ZOOM`) and the image point (in image pixels) at its middle.
    pub(crate) zoom: f32,
    pub(crate) centre: Vec2,
    /// Until when (input clock, ms) the bar shows.
    pub(crate) bar_until: Option<i64>,
    /// Where the last pass laid the window out, and on what screen (points).
    pub(crate) layout: Option<(FloatingLayout, Rect)>,
    pub(crate) pixels_per_point: f32,
    /// The contacts on the window and the gestures on its picture.
    pub(crate) touch: crate::reference_touch::WindowTouch,
    pub(crate) gestures: PictureGestures,
}

impl std::fmt::Debug for ReferenceWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceWindow")
            .field("place", &self.place)
            .field("zoom", &self.zoom)
            .field("bar_until", &self.bar_until)
            .finish_non_exhaustive()
    }
}

impl ReferenceWindow {
    /// The window while its image is shown; it comes out of `opener` (the Actions button, in
    /// points), where its switch is, and goes back into it.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        (now_ms, opener): (i64, Rect),
        taps: &mut Vec<Intent>,
    ) {
        let transfers = projection.transfers();
        let image = transfers
            .filter(|transfers| transfers.reference_shown)
            .and_then(|transfers| transfers.reference.as_ref());
        let Some(image) = image else {
            self.hide();
            return;
        };
        let texture = self.load(ctx, image);
        let screen = ctx.content_rect();
        self.pixels_per_point = ctx.pixels_per_point();
        let place = self.place.unwrap_or_else(|| {
            let size = body_size(image.size()) + Vec2::splat(2.0 * MAT);
            Rect::from_min_size(screen.max - size - Vec2::splat(SPACE), size)
        });
        let place = kept_on(screen, place);
        self.place = Some(place);
        self.bar_until = self.bar_until.filter(|until| now_ms < *until);
        let shown = floating(
            ctx,
            "reference",
            (place, self.bar_until.is_some(), ring_of(opener)),
            |ui| icon_button(ui, Icon::Close, "Close reference", true).clicked(),
            |ui, picture| self.paint_picture(ui, picture, (image, &texture)),
        );
        if shown.buttons == Some(true) {
            self.bar_until = None;
            push(taps, Intent::Reference(ReferenceIntent::Show(false)));
        }
        self.layout = Some((shown.layout, screen));
    }

    /// No window: nothing of it lies on screen and nothing it held goes on.
    fn hide(&mut self) {
        self.layout = None;
        self.bar_until = None;
        self.touch = crate::reference_touch::WindowTouch::default();
        self.gestures.reset();
    }

    /// Where the window lies, which the canvas must not take contacts from.
    pub(crate) fn area(&self) -> Option<ScreenArea> {
        let (layout, _) = self.layout?;
        Some(ScreenArea::from_points(
            layout.window,
            self.pixels_per_point,
        ))
    }

    /// True while the bar shows: frames must keep coming to hide it on time.
    pub(crate) const fn bar_shown(&self) -> bool {
        self.bar_until.is_some()
    }

    /// The image on white in `picture`, fitted and zoomed about `centre`.
    fn paint_picture(
        &self,
        ui: &egui::Ui,
        picture: Rect,
        (image, texture): (&ReferenceImage, &egui::TextureHandle),
    ) {
        let painter = ui.painter_at(picture);
        painter.rect_filled(picture, 0.0, Color32::WHITE);
        let (shown, _) = self.shown_image(picture, image);
        let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), shown, whole, Color32::WHITE);
    }

    /// Where `image` lies over `picture` (points), and how many points one image pixel is.
    pub(crate) fn shown_image(&self, picture: Rect, image: &ReferenceImage) -> (Rect, f32) {
        let pixels = image_pixels(image);
        let scale = fit(picture.size(), pixels) * self.zoom;
        let min = picture.center() - self.centre * scale;
        (Rect::from_min_size(min, pixels * scale), scale)
    }

    /// The picture's pan and zoom by `gesture` (screen pixels): the image point under the
    /// gesture's start stays under its pivot.
    pub(crate) fn navigate(&mut self, gesture: ViewGesture, picture: Rect, image: &ReferenceImage) {
        let ppp = self.pixels_per_point;
        let pixels = image_pixels(image);
        let fit = fit(picture.size(), pixels);
        let pivot = Pos2::new(gesture.pivot[0], gesture.pivot[1]) / ppp;
        let moved = Vec2::new(gesture.translation[0], gesture.translation[1]) / ppp;
        let held = self.centre + (pivot - moved - picture.center()) / (fit * self.zoom);
        self.zoom = (self.zoom * gesture.zoom).clamp(ZOOM[0], ZOOM[1]);
        let centre = held - (pivot - picture.center()) / (fit * self.zoom);
        self.centre = clamp_centre(centre, pixels, picture.size() / (fit * self.zoom));
    }

    /// The image the window shows now.
    pub(crate) fn image(&self) -> Option<&ReferenceImage> {
        self.texture.as_ref().map(|(image, _)| image)
    }

    /// The texture of `image`, uploaded when the image is new; a new image starts whole and
    /// centred in a window of its shape where the last one was.
    pub(crate) fn load(
        &mut self,
        ctx: &egui::Context,
        image: &ReferenceImage,
    ) -> egui::TextureHandle {
        if let Some((shown, texture)) = &self.texture
            && shown == image
        {
            return texture.clone();
        }
        let [width, height] = image.size().map(|side| side as usize);
        let picture = egui::ColorImage::from_rgba_unmultiplied([width, height], image.pixels());
        let texture = ctx.load_texture("reference", picture, egui::TextureOptions::LINEAR);
        self.zoom = ZOOM[0];
        self.centre = image_pixels(image) / 2.0;
        let size = body_size(image.size()) + Vec2::splat(2.0 * MAT);
        self.place = self.place.map(|place| Rect::from_min_size(place.min, size));
        self.texture = Some((image.clone(), texture.clone()));
        texture
    }
}

/// `place` moved and shrunk onto `screen`, under the islands.
pub(crate) fn kept_on(screen: Rect, place: Rect) -> Rect {
    let top = screen.top() + UNDER_ISLANDS;
    let room = Vec2::new(screen.width(), screen.bottom() - top);
    let size = place.size().min(room * MOST).max(LEAST.min(room));
    let x = place
        .left()
        .clamp(screen.left(), (screen.right() - size.x).max(screen.left()));
    let y = place.top().clamp(top, (screen.bottom() - size.y).max(top));
    Rect::from_min_size(Pos2::new(x, y), size)
}

/// `place` with its bottom-right corner moved by `delta`, its top-left corner kept, at least
/// `LEAST` and at most `MOST` of the screen.
pub(crate) fn resized(screen: Rect, place: Rect, delta: Vec2) -> Rect {
    let most = (screen.size() * MOST).min(screen.max - place.min);
    let size = (place.size() + delta).clamp(LEAST.min(most), most);
    Rect::from_min_size(place.min, size)
}

fn image_pixels(image: &ReferenceImage) -> Vec2 {
    Vec2::new(image.size()[0] as f32, image.size()[1] as f32)
}

/// Points per image pixel when an image of `pixels` fits `area` whole.
fn fit(area: Vec2, pixels: Vec2) -> f32 {
    (area.x / pixels.x).min(area.y / pixels.y)
}

/// The picture area for an image of `size` pixels: `REFERENCE_WIDTH` wide, as tall as the
/// image's shape asks within `REFERENCE_HEIGHT`.
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

    #[test]
    fn the_window_stays_on_the_screen_and_resizes_within_bounds() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        let off = Rect::from_min_size(Pos2::new(900.0, 0.0), Vec2::new(300.0, 200.0));
        let kept = kept_on(screen, off);
        assert_eq!(kept.min, Pos2::new(700.0, UNDER_ISLANDS));
        assert_eq!(kept.size(), off.size());
        let place = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(300.0, 200.0));
        let small = resized(screen, place, Vec2::new(-500.0, -500.0));
        assert_eq!(small.size(), LEAST);
        let large = resized(screen, place, Vec2::new(2000.0, 2000.0));
        assert_eq!(
            (large.min, large.size()),
            (place.min, Vec2::new(850.0, 680.0))
        );
    }
}
