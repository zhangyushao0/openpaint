//! An artwork carried between its gallery card and the canvas (emerge and retract, D44). Opened,
//! the card's picture stays where it lay until the canvas shows, then the canvas grows out of
//! it (`ViewSettle`); going back, the canvas's picture shrinks from where the canvas showed into
//! its card, which stays empty until it lands. A mode's sheet goes back the same way into the
//! mode's place, with the picture it left with and the mode's marks over it (S144).

use egui::{Color32, Id, LayerId, Order, Rect};
use openpaint_app::{ProjectId, Thumbnail};

use crate::components::{Alpha, animate_from, paint_sheet, picture_texture};
use crate::design::motion::{CLOSE, lerp};

/// The artwork being opened: where its picture lay on screen (points), which one, and the
/// picture.
#[derive(Clone)]
pub struct Opening {
    pub from: Rect,
    pub id: Option<ProjectId>,
    pub picture: Option<egui::TextureHandle>,
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
#[derive(Clone)]
pub(crate) struct Arrival {
    /// Where the canvas showed, in points (a mode's sheet: the part shown).
    pub(crate) from: Rect,
    /// Its card; `None` for the first card (the artwork changed last).
    pub(crate) id: Option<ProjectId>,
    /// A mode's sheet, which goes back into the mode's place (`HomeHook::column`).
    pub(crate) card: bool,
    /// One motion per return.
    pub(crate) run: u64,
    /// The picture a mode's sheet left with (an artwork's is its card's).
    pub(crate) picture: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for Arrival {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arrival")
            .field("from", &self.from)
            .field("id", &self.id)
            .field("card", &self.card)
            .field("run", &self.run)
            .finish_non_exhaustive()
    }
}

/// Paints `picture` as a sheet on the desk at `rect`, over the gallery's cards and under its
/// islands; what paints there.
pub(crate) fn sheet_at(
    ctx: &egui::Context,
    picture: Option<&egui::TextureHandle>,
    rect: Rect,
) -> egui::Painter {
    let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("carried artwork")));
    paint_sheet(&painter, picture, rect, Color32::WHITE);
    painter
}

/// Where `arrival` lies now on its way from where it began into `card`, and whether it has
/// landed.
pub(crate) fn carried(ctx: &egui::Context, arrival: &Arrival, card: Rect) -> (Rect, bool) {
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
    (rect, t >= 1.0)
}

/// Carries `picture` from where `arrival` began into `card`; returns whether it has landed.
pub(crate) fn land(
    ctx: &egui::Context,
    arrival: &Arrival,
    picture: Option<&egui::TextureHandle>,
    card: Rect,
) -> bool {
    let (rect, landed) = carried(ctx, arrival, card);
    sheet_at(ctx, picture, rect);
    landed
}

/// `picture`, the canvas's as it was left, as a texture to carry.
pub(crate) fn texture(ctx: &egui::Context, picture: &Thumbnail) -> egui::TextureHandle {
    let pixels = (picture.size(), picture.pixels(), Alpha::Straight);
    picture_texture(ctx, "carried picture", pixels)
}
