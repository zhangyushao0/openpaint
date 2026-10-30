//! The practice options' Practice section (S91): each ready stage a row, its name at the left and
//! Start a session at the right (the row tapped starts the stage's next session at once, as the
//! stages did before: the next stage early, an earlier one to revisit), and under it the stage's
//! kinds as tiles (`components::kind_tile`), in ink once compared anywhere, in pencil before,
//! ringed while a drill of the kind is left unfinished. A tile tapped starts a drill of its kind,
//! or continues the one left unfinished; the Shape tile without any pack picture, Division and
//! Whole copy without a composition (S95), or Value masses without one whose masses read (S96),
//! say where to import a pack instead, and nothing starts.

use egui::{Align2, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};
use openpaint_app::{DrillShown, GalleryIntent, Language, TodayProjection};
use openpaint_practice::Family;

use super::practice::{Tapped, note};
use crate::components::{KindTile, controls, kind_tile, section};
use crate::design::{INK, Outline, SPACE, TOUCH_TARGET, paint_pressed};
use crate::text::{self as t, Words};

/// The least width a tile takes with its name under it, in points.
const TILE_CELL: f32 = 72.0;

/// The Practice section; `needs_pack` is set to the kind whose tile was tapped without its pack's
/// pictures (its note shows under its stage's tiles). Returns what was tapped.
pub(super) fn practice_section(
    ui: &mut egui::Ui,
    today: &TodayProjection,
    needs_pack: &mut Option<Family>,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    section(ui, t::PRACTICE);
    for stage in 1..=today.ready {
        let name = t::stage(language, stage);
        let english = t::stage(Language::English, stage);
        if stage_row(ui, Words::named(&english, &name)).clicked() {
            chosen = Some(Tapped::Intent(GalleryIntent::Practice(stage), true));
        }
        let tiles: Vec<&DrillShown> = (today.drills.iter())
            .filter(|tile| tile.stage == stage)
            .collect();
        if let Some(tapped) = tiles_of(ui, &tiles) {
            let pictures = !today.pictures.is_empty();
            match (tapped.family, tapped.open) {
                (Family::Shape, None) if !pictures => *needs_pack = Some(Family::Shape),
                (Family::Division | Family::WholeCopy, None) if !today.compositions => {
                    *needs_pack = Some(tapped.family);
                }
                (Family::ValueMasses, None) if !today.masses => {
                    *needs_pack = Some(tapped.family);
                }
                (_, Some(sheet)) => chosen = Some(Tapped::Intent(GalleryIntent::Open(sheet), true)),
                (family, None) => chosen = Some(Tapped::Intent(GalleryIntent::Drill(family), true)),
            }
        }
        let needed = needs_pack.filter(|family| tiles.iter().any(|tile| tile.family == *family));
        match needed {
            Some(Family::Shape) => note(ui, t::DRILL_NEEDS_PACK.into(), false),
            Some(Family::ValueMasses) => note(ui, t::DRILL_NEEDS_MASSES.into(), false),
            Some(_) => note(ui, t::DRILL_NEEDS_COMPOSITIONS.into(), false),
            None => {}
        }
    }
    chosen
}

/// A stage's row: its name at the left, Start a session in ink at the right, the whole row one
/// touch target high and named by the stage; pressed, the wash fills the words' pill.
fn stage_row(ui: &mut egui::Ui, name: Words<'_>) -> egui::Response {
    let shown = name.shown(ui.ctx());
    let size = Vec2::new(ui.available_width(), TOUCH_TARGET);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, shown));
    controls::register(ui, name.id(), rect);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let body = TextStyle::Body.resolve(ui.style());
    let left = rect.left_center() + Vec2::new(2.0 * SPACE, 0.0);
    painter.text(left, Align2::LEFT_CENTER, shown, body, INK);
    let start = Words::from(t::START_A_SESSION).shown(ui.ctx()).to_owned();
    let button = TextStyle::Button.resolve(ui.style());
    let galley = painter.layout_no_wrap(start, button, INK);
    let width = galley.size().x + 4.0 * SPACE;
    let place = egui::Rect::from_min_size(
        egui::pos2(rect.right() - width, rect.top()),
        Vec2::new(width, TOUCH_TARGET),
    );
    if response.is_pointer_button_down_on() {
        paint_pressed(painter, Outline::pill(place));
    }
    let at = place.center() - galley.size() / 2.0;
    painter.galley(at, galley, INK);
    response
}

/// `tiles` in rows across the column, as many to a row as fit `TILE_CELL`; the one tapped.
fn tiles_of<'a>(ui: &mut egui::Ui, tiles: &[&'a DrillShown]) -> Option<&'a DrillShown> {
    let width = ui.available_width();
    let columns = ((width / TILE_CELL).floor() as usize).max(1);
    let cell = width / columns as f32;
    let mut tapped = None;
    for row in tiles.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for tile in row {
                let shown = KindTile {
                    family: tile.family,
                    name: t::kind_name(tile.family).into(),
                    inked: tile.compared,
                    ringed: tile.open.is_some(),
                };
                if kind_tile(ui, &shown, cell).clicked() {
                    tapped = Some(*tile);
                }
            }
        });
    }
    tapped
}
