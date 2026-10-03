//! An artwork carried between its gallery card and the canvas (emerge and retract, D44). Opened,
//! the card's picture stays where it lay until the canvas shows, then the canvas grows out of
//! it (`ViewSettle`); going back, the canvas's picture shrinks from where the canvas showed into
//! its card, which stays empty until it lands.

use egui::{Color32, Id, LayerId, Order, Rect};
use openpaint_app::ProjectId;

use crate::components::{animate_from, paint_sheet};
use crate::design::motion::{CLOSE, lerp};

/// The artwork being opened: where its picture lay on screen (points), which one, and the
/// picture.
#[derive(Clone)]
pub(crate) struct Opening {
    pub(crate) from: Rect,
    pub(crate) id: Option<ProjectId>,
    pub(crate) picture: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for Opening {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opening")
            .field("from", &self.from)
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// The artwork coming back from the canvas into its card.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Arrival {
    /// Where the canvas showed, in points.
    pub(crate) from: Rect,
    /// Its card; `None` for the first card (the artwork changed last).
    pub(crate) id: Option<ProjectId>,
    /// One motion per return.
    pub(crate) run: u64,
}

/// Paints `picture` as a sheet on the desk at `rect`, over the gallery's cards and under its
/// islands.
pub(crate) fn sheet_at(ctx: &egui::Context, picture: Option<&egui::TextureHandle>, rect: Rect) {
    let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("carried artwork")));
    paint_sheet(&painter, picture, rect, Color32::WHITE);
}

/// Carries `picture` from where `arrival` began into `card`; returns whether it has landed.
pub(crate) fn land(
    ctx: &egui::Context,
    arrival: &Arrival,
    picture: Option<&egui::TextureHandle>,
    card: Rect,
) -> bool {
    let t = animate_from(ctx, Id::new(("arrival", arrival.run)), 0.0, 1.0, CLOSE);
    let from = arrival.from;
    let rect = Rect::from_min_max(
        egui::pos2(
            lerp(from.min.x, card.min.x, t),
            lerp(from.min.y, card.min.y, t),
        ),
        egui::pos2(
            lerp(from.max.x, card.max.x, t),
            lerp(from.max.y, card.max.y, t),
        ),
    );
    sheet_at(ctx, picture, rect);
    t >= 1.0
}
