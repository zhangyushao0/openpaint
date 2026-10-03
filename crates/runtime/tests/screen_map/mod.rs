//! Shared by the runtime's GPU tests that look at where the canvas shows: the view's map
//! between screen and canvas pixels, and reading one pixel at a point.

use openpaint_app::{ViewMapping, Viewport};
use openpaint_render::RgbaImage;
use openpaint_runtime::Runtime;
use openpaint_ui::TOP_BAND;

/// The map between screen and canvas pixels as the runtime on a screen of `size` at
/// `pixels_per_point` shows its canvas now: fitted under the top band, through the session's
/// view.
pub fn mapping(runtime: &Runtime, size: [u32; 2], pixels_per_point: f32) -> ViewMapping {
    let top = (TOP_BAND * pixels_per_point).round();
    let viewport = Viewport {
        origin: [0.0, top],
        size: [size[0] as f32, size[1] as f32 - top],
        pixels_per_point,
    };
    let projection = runtime.projection();
    let canvas = projection.canvas().expect("an open canvas");
    projection.view().mapping(canvas, &viewport)
}

/// The RGBA pixel of `image` that holds the point `at` (pixels of that image).
pub fn pixel_at(image: &RgbaImage, at: [f32; 2]) -> [u8; 4] {
    let [x, y] = at.map(|v| v.floor() as u32);
    let index = ((y * image.size()[0] + x) * 4) as usize;
    let mut out = [0; 4];
    out.copy_from_slice(&image.pixels()[index..index + 4]);
    out
}
