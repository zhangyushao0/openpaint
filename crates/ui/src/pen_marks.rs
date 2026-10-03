//! What shows where the pen or the picking hand is (D40, D41): the brush's outline at full size
//! while the pen hovers over open canvas; nothing while the brush draws (the ink under the tip
//! is the feedback, as in Procreate); the eraser's outline at its real pressure size while it
//! erases, so the hand sees what goes; and the loupe while a contact picks a colour. The
//! outlines hide over controls and during multi-touch (the canvas input knows when).

use openpaint_app::{BrushProjection, ViewMapping};
use openpaint_render::BrushCursor;

use crate::components::loupe;

/// Smallest radius of the pen's outline, in points, so a thin brush still shows a ring.
const CURSOR_MIN_RADIUS: f32 = 2.0;
/// Width of the pen's outline, in points.
const CURSOR_WIDTH: f32 = 1.0;

/// The outline to draw in the screen blit: around the hovering pen (`hover`, screen pixels) at
/// the brush's `size` (canvas pixels), or around the eraser's tip (`erasing`: centre and radius
/// in canvas pixels) while it erases over open canvas.
pub(crate) fn cursor(
    hover: Option<[f32; 2]>,
    erasing: Option<([f32; 2], f32)>,
    size: Option<f32>,
    mapping: &ViewMapping,
    pixels_per_point: f32,
) -> Option<BrushCursor> {
    let scale = mapping.scale();
    let (centre, diameter) = match hover {
        Some(centre) => (centre, size? * scale),
        None => {
            let (tip, radius) = erasing?;
            (mapping.to_screen(tip), 2.0 * radius * scale)
        }
    };
    Some(BrushCursor {
        centre,
        radius: (diameter / 2.0).max(CURSOR_MIN_RADIUS * pixels_per_point),
        width: CURSOR_WIDTH * pixels_per_point,
    })
}

/// The loupe for the contact picking at `at` (screen pixels), once it previewed a colour.
pub(crate) fn show_loupe(
    ctx: &egui::Context,
    at: Option<[f32; 2]>,
    brush: Option<&BrushProjection>,
    pixels_per_point: f32,
) {
    let Some(brush) = brush.filter(|brush| brush.picking) else {
        return;
    };
    if let (Some([x, y]), Some(under)) = (at, brush.preview) {
        let contact = egui::pos2(x, y) / pixels_per_point;
        loupe(ctx, contact, under, brush.color);
    }
}
