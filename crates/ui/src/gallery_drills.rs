//! The practice options' Practice section (S91): each ready stage a row, its name at the left and
//! Start a session at the right (the row tapped starts the stage's next session at once, as the
//! stages did before: the next stage early, an earlier one to revisit), and under it the stage's
//! kinds as tiles (`components::kind_tile`), in ink once compared anywhere, in pencil before,
//! ringed while a drill of the kind is left unfinished. A tile tapped starts a drill of its kind,
//! or continues the one left unfinished; the Shape tile without any pack picture, Division and
//! Whole copy without a composition (S95), Value masses without one whose masses read (S96), or
//! Colour combination or Key shift without a crop (S101, S107; Stage 5's row too), say where to
//! import a pack instead, and nothing starts.

use egui::{Align2, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};
use openpaint_app::{GalleryIntent, Language};
use openpaint_practice::{Family, Needs, stage_needs};
use openpaint_practice_app::{DrillShown, OptionsIntent, TodayProjection};

use super::practice::{Tapped, note};
use crate::components::{KindTile, controls, kind_tile, section};
use crate::design::{INK, Outline, SPACE, TOUCH_TARGET, paint_pressed};
use crate::text::{self as t, Words};

/// The least width a tile takes with its name under it, in points.
const TILE_CELL: f32 = 72.0;

/// Whether `today` keeps what `needs` asks for.
fn kept(today: &TodayProjection, needs: Needs) -> bool {
    match needs {
        Needs::Nothing => true,
        Needs::Pictures => !today.pictures.is_empty(),
        Needs::Compositions => today.compositions,
        Needs::Masses => today.masses,
        Needs::Crops => today.crops,
    }
}

/// The note under a stage's tiles saying where to import what `needs` asks for.
fn needs_note(needs: Needs) -> t::Text {
    match needs {
        Needs::Pictures => t::DRILL_NEEDS_PACK,
        Needs::Masses => t::DRILL_NEEDS_MASSES,
        Needs::Crops => t::DRILL_NEEDS_CROPS,
        Needs::Compositions | Needs::Nothing => t::DRILL_NEEDS_COMPOSITIONS,
    }
}

/// The Practice section; `needs_pack` is set to the kind whose tile was tapped without its pack's
/// pictures (its note shows under its stage's tiles). Returns what was tapped.
pub(super) fn practice_section(
    ui: &mut egui::Ui,
    today: &TodayProjection,
    needs_pack: &mut Option<Family>,
) -> Option<Tapped> {
    let language = t::language(ui.ctx());
    let mut chosen = None;
    let asked = *needs_pack;
    section(ui, t::PRACTICE);
    for stage in 1..=today.ready {
        let name = t::stage(language, stage);
        let english = t::stage(Language::English, stage);
        let tiles: Vec<&DrillShown> = (today.drills.iter())
            .filter(|tile| tile.stage == stage)
            .collect();
        if stage_row(ui, Words::named(&english, &name)).clicked() {
            // A stage whose sessions need what is not kept (S107: stage 5's key shifts, crops)
            // starts none: its tile needing the same says where to import it.
            let needs = stage_needs(stage);
            if kept(today, needs) {
                chosen = Some(Tapped::Options(OptionsIntent::Practice(stage), true));
            } else {
                *needs_pack = (tiles.iter())
                    .map(|tile| tile.family)
                    .find(|family| family.facts().needs == needs);
            }
        }
        if let Some(tapped) = tiles_of(ui, &tiles) {
            let needs = tapped.family.facts().needs;
            match tapped.open {
                Some(sheet) => chosen = Some(Tapped::Intent(GalleryIntent::Open(sheet), true)),
                None if !kept(today, needs) => *needs_pack = Some(tapped.family),
                None => {
                    let drill = OptionsIntent::Drill(tapped.family);
                    chosen = Some(Tapped::Options(drill, true));
                }
            }
        }
        let needed = needs_pack.filter(|family| tiles.iter().any(|tile| tile.family == *family));
        if let Some(family) = needed {
            note(ui, needs_note(family.facts().needs).into(), false);
        }
        // The note this tap asked for comes into view (stage 5's lies at the column's end).
        if needed.is_some() && *needs_pack != asked {
            ui.scroll_to_cursor(None);
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
