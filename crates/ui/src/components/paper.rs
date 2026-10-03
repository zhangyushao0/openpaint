//! A piece of paper that comes and goes (emerge and retract, D44): a pill, the loupe, the
//! reference window. Shown in a frame, it is open and grows out of its source; once its owner
//! stops showing it, it goes back into that source (`chrome`). Its content is laid out at the
//! finished place from the first frame and fades in near the end of the growth.

use egui::{Id, Shape};

use crate::components::chrome::{self, Source};

/// Shows `area` (`id` is its id) as paper growing out of `source`, with `add_contents` inside.
/// Returns what the contents returned and the finished surface's rectangle in points.
pub(crate) fn paper<R>(
    ctx: &egui::Context,
    id: Id,
    source: Source,
    area: egui::Area,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> (R, egui::Rect) {
    let layer = area.layer();
    let reveal = chrome::declare(ctx, id, source, layer.order, true).map_or(1.0, |s| s.reveal);
    let shown = area.show(ctx, |ui| {
        let paper = ui.painter().add(Shape::Noop);
        ui.multiply_opacity(reveal);
        (add_contents(ui), paper)
    });
    let (inner, paper) = shown.inner;
    let rect = shown.response.rect;
    chrome::laid_out(ctx, id, rect, shown.response.layer_id, Some(paper));
    (inner, rect)
}
