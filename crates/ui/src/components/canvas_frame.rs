//! The canvas frame: the screen area that shows the canvas and owns the contacts that touch
//! down in it (`CanvasInput`). It paints nothing, so the renderer's canvas shows through, and
//! nothing covers it while a stroke is possible.

use openpaint_app::Viewport;

/// A rectangle of screen pixels: `origin` is its top-left pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenArea {
    pub origin: [i32; 2],
    pub size: [u32; 2],
}

impl ScreenArea {
    /// `rect` in points on a screen of `pixels_per_point`, rounded to whole pixels.
    #[must_use]
    pub fn from_points(rect: egui::Rect, pixels_per_point: f32) -> Self {
        let pixel = |points: f32| (points * pixels_per_point).round();
        let [x0, y0, x1, y1] = [rect.min.x, rect.min.y, rect.max.x, rect.max.y].map(pixel);
        Self {
            origin: [x0 as i32, y0 as i32],
            size: [(x1 - x0).max(0.0) as u32, (y1 - y0).max(0.0) as u32],
        }
    }

    /// True when the screen point `[x, y]` (pixels) lies inside.
    #[must_use]
    pub fn contains(&self, [x, y]: [f32; 2]) -> bool {
        let [left, top] = self.origin.map(|side| side as f32);
        let [width, height] = self.size.map(|side| side as f32);
        (left..left + width).contains(&x) && (top..top + height).contains(&y)
    }

    /// This area as the view's viewport on a screen of `pixels_per_point`.
    #[must_use]
    pub fn viewport(&self, pixels_per_point: f32) -> Viewport {
        Viewport {
            origin: self.origin.map(|side| side as f32),
            size: self.size.map(|side| side as f32),
            pixels_per_point,
        }
    }
}

/// Lays out the canvas frame in the space the other panels left and returns it, in points.
pub fn canvas_frame(ui: &mut egui::Ui) -> egui::Rect {
    egui::CentralPanel::no_frame()
        .show(ui, |ui| ui.max_rect())
        .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_area_rounds_points_to_whole_pixels() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 48.0), egui::pos2(1645.7, 1028.6));
        let area = ScreenArea::from_points(rect, 1.75);
        assert_eq!(area.origin, [0, 84]);
        assert_eq!(area.size, [2880, 1716]);
        assert!(area.contains([0.0, 84.0]) && !area.contains([0.0, 83.9]));
        assert_eq!(area.viewport(1.75).size, [2880.0, 1716.0]);
    }
}
