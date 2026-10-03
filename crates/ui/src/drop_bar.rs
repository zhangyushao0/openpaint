//! What the screen shows of a colour drop: the drop the hand carries, and the fill bar in the
//! top band: what to do while dropping, the threshold while it is adjusted, then the offer to
//! keep filling.

use openpaint_app::SessionProjection;

use crate::color_drop::{ColorDrop, DropBar};
use crate::components::{ScreenArea, carried_drop, fill_bar};

/// Shows the carried drop and the fill bar for `drop`; returns where the bar lies, in pixels of
/// a screen of `pixels_per_point`.
pub(crate) fn show(
    drop: &mut ColorDrop,
    ctx: &egui::Context,
    projection: &SessionProjection,
    pixels_per_point: f32,
) -> Option<ScreenArea> {
    let ppp = pixels_per_point;
    if let (Some([x, y]), Some(brush)) = (drop.carried(), projection.brush()) {
        carried_drop(ctx, egui::pos2(x / ppp, y / ppp), brush.color);
    }
    let bar = drop.bar()?;
    // A refused or failed drop filled nothing: no offer to keep filling.
    let fill = projection.fill();
    if bar == DropBar::Offered && (fill.refused.is_some() || fill.failed) {
        drop.set_continuing(false);
        return None;
    }
    let (text, action) = match bar {
        DropBar::Dropping(None) => ("Lift to fill. Hold still to adjust.".to_owned(), None),
        DropBar::Dropping(Some(threshold)) => (
            format!("Threshold {:.1} % · slide to adjust", threshold * 100.0),
            None,
        ),
        DropBar::Offered => ("Filled.".to_owned(), Some("Continue filling")),
        DropBar::Continuing => ("Tap an area to fill it.".to_owned(), Some("Done")),
    };
    let taken = projection.selection().tool || projection.transform().tool;
    let (rect, clicked) = fill_bar(ctx, &text, action, taken);
    if clicked {
        drop.set_continuing(bar == DropBar::Offered);
    }
    Some(ScreenArea::from_points(rect, ppp))
}
