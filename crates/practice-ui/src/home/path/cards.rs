//! The path page's quiet cards under a unit's tiles: a drawing from life's (S151, from unit 2: a
//! book and a cup in ink, its name over the subject the unit suggests) and the big homework's
//! (S154, from unit 7: a warm swatch beside a cool one, Big homework over Cold-warm copy), each
//! with New….

use egui::{Align, Layout, Rect, Stroke, Vec2};
use openpaint_practice::Leans;
use openpaint_ui::components::text_button;
use openpaint_ui::design::{CORNER, DIVIDER, INK, SPACE};

use super::super::column::label;
use crate::text::{self as t, Words};

/// A card's width at most and its glyph's size, in points.
const CARD: f32 = 560.0;
const GLYPH: Vec2 = Vec2::new(72.0, 44.0);

/// A drawing from life's card; where New… lies once tapped.
pub(super) fn life_card(ui: &mut egui::Ui, unit: u8) -> Option<Rect> {
    quiet_card(ui, still_life, (t::LIFE_EXERCISE, t::life_subject(unit)))
}

/// The big homework's card, a cold-warm copy; where New… lies once tapped.
pub(super) fn homework_card(ui: &mut egui::Ui) -> Option<Rect> {
    quiet_card(ui, swatches, (t::BIG_HOMEWORK, t::COLD_WARM_EXERCISE))
}

/// A quiet card: `glyph` drawn, a quiet `above` over its `name`, and New…; where New… lies once
/// tapped.
fn quiet_card(
    ui: &mut egui::Ui,
    glyph: fn(&egui::Painter, Rect),
    (above, name): (t::Text, t::Text),
) -> Option<Rect> {
    let width = ui.available_width().min(CARD);
    let margin = egui::Margin::symmetric((2.5 * SPACE) as i8, (2.0 * SPACE) as i8);
    let frame = egui::Frame::NONE
        .stroke(Stroke::new(1.25, DIVIDER))
        .corner_radius(CORNER)
        .inner_margin(margin);
    let shown = frame.show(ui, |ui| {
        ui.set_width(width - 5.0 * SPACE);
        ui.horizontal(|ui| {
            let (place, _) = ui.allocate_exact_size(GLYPH, egui::Sense::hover());
            glyph(ui.painter(), place);
            ui.add_space(2.0 * SPACE);
            ui.vertical(|ui| {
                label(ui, above);
                ui.label(Words::from(name).shown(ui.ctx()));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let new = text_button(ui, t::LIFE_NEW, true);
                new.clicked().then_some(new.rect)
            })
            .inner
        })
        .inner
    });
    shown.inner
}

/// The homework card's glyph in `rect`: the first cold-warm copy's lights' swatch beside its
/// shadows', each outlined in ink.
fn swatches(painter: &egui::Painter, rect: Rect) {
    let leans = Leans::nth(0);
    let side = Vec2::new(30.0, 40.0);
    for (x, [r, g, b]) in [(2.0, leans.lights()), (40.0, leans.shadows())] {
        let swatch = Rect::from_min_size(rect.min + Vec2::new(x, 2.0), side);
        let fill = egui::Color32::from_rgb(r, g, b);
        painter.rect(
            swatch,
            0.0,
            fill,
            Stroke::new(1.0, INK),
            egui::StrokeKind::Inside,
        );
    }
}

/// A book lying and a cup on it, in the glyph's units (`GLYPH`): the book's faces, the cup's sides.
const BOOK: [&[[f32; 2]]; 2] = [
    &[
        [6.0, 40.0],
        [6.0, 20.0],
        [30.0, 14.0],
        [30.0, 34.0],
        [6.0, 40.0],
    ],
    &[[6.0, 20.0], [16.0, 16.0], [40.0, 10.0], [30.0, 14.0]],
];
const CUP: [[[f32; 2]; 2]; 2] = [[[46.0, 14.0], [46.0, 38.0]], [[66.0, 14.0], [66.0, 38.0]]];

/// The life card's glyph in `rect`: the book and the cup, the cup's rim an ellipse and its foot
/// the ellipse's near half.
fn still_life(painter: &egui::Painter, rect: Rect) {
    let ink = Stroke::new(1.5, INK);
    let at = |[x, y]: [f32; 2]| rect.min + Vec2::new(x, y);
    for line in BOOK {
        painter.add(egui::Shape::line(
            line.iter().copied().map(at).collect(),
            ink,
        ));
    }
    for [top, foot] in CUP {
        painter.line_segment([at(top), at(foot)], ink);
    }
    let round = |turn: f32, y: f32| {
        let angle = std::f32::consts::PI * turn;
        at([56.0 + 10.0 * angle.cos(), y + 3.5 * angle.sin()])
    };
    let rim = (0..=24).map(|k| round(k as f32 / 12.0, 14.0)).collect();
    painter.add(egui::Shape::line(rim, ink));
    let foot = (0..=12).map(|k| round(k as f32 / 12.0, 38.0)).collect();
    painter.add(egui::Shape::line(foot, ink));
}
