//! A key shift compared on the practice card (S107). It shows rather than tells (D108): under the
//! title one short line for each check that failed, each after a red dot ("1 area darker", "1
//! area not toward the tendency", "2 areas merged into one"); Grey (看黑白); the areas' swatches,
//! the crop's colour over yours, the failing ones outlined red, a tap choosing one; the hue and
//! chroma disc (`practice_card_disc.rs`); and the chosen area's lightness bar, the crop's tick
//! over it and yours under it (red when off), the two in numbers with their difference.

use egui::{Pos2, Sense, Shape, Stroke, Vec2};
use openpaint_app::{Intent, Overlay, PracticeIntent, PracticeProjection};
use openpaint_practice::{KeyCheck, LIGHTNESS_OFF, Rgb8, from_oklab, oklab};

use crate::components::{card_text, choices, text_choice};
use crate::design::{CORRECTION, DIVIDER, INK, SPACE};
use crate::practice_card_colour::chips;
use crate::practice_card_disc::disc;
use crate::practice_marks::colour;
use crate::taps::push_once;
use crate::text::{self as t, Words, language};

/// The lightness bar's height and its ticks' reach past it, in points.
const BAR: f32 = 14.0;
const TICK: f32 = 8.0;
/// A failure's red dot's radius, in points.
const DOT: f32 = 4.0;

/// A key shift compared: its lines, Grey, the swatches, the disc and the chosen area's
/// lightness; until the copy is read, saying so.
pub(crate) fn compared(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<Intent>) {
    let (
        Some(Overlay::KeyShift {
            read, chosen, grey, ..
        }),
        Some(index),
    ) = (&practice.overlay, practice.current)
    else {
        return;
    };
    let (Some((areas, _)), Some(Some(tendency))) = (
        practice.sheet.areas(index),
        practice.sheet.tendencies.get(index),
    ) else {
        return;
    };
    let Some(read) = read else {
        card_text(ui, t::READING_COPY, true);
        return;
    };
    failures(ui, &read.check);
    ui.horizontal(|ui| {
        choices(ui, "grey", |ui, group| {
            if text_choice(ui, group, t::GREY_VIEW, *grey).clicked() {
                push_once(taps, Intent::Practice(PracticeIntent::Grey(!*grey)));
            }
        });
    });
    let marked: Vec<usize> = (0..areas.count())
        .filter(|area| read.check.marked(*area))
        .collect();
    chips(
        ui,
        (&areas.colours, Some(&read.colours)),
        (*chosen, &marked),
        taps,
    );
    disc(ui, (&areas.colours, &read.colours), *tendency, *chosen);
    let original = areas.colours.get(*chosen).copied();
    match (original, read.colours.get(*chosen).copied().flatten()) {
        (Some(original), Some(mine)) => lightness(ui, original, mine),
        _ => card_text(ui, t::NOT_PAINTED, false),
    }
}

/// One line for each check that failed, after a red dot.
fn failures(ui: &mut egui::Ui, check: &KeyCheck) {
    let language = language(ui.ctx());
    let (lighter, darker) = check.lighter_darker();
    let (areas, blocks) = check.merged_blocks();
    let lines = [
        (lighter > 0).then(|| t::lightness_off(language, lighter, true)),
        (darker > 0).then(|| t::lightness_off(language, darker, false)),
        (!check.unmoved.is_empty()).then(|| t::not_toward(language, check.unmoved.len())),
        (blocks > 0).then(|| t::merged(language, areas, blocks)),
    ];
    for line in lines.into_iter().flatten() {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(3.0 * DOT), Sense::hover());
            ui.painter().circle_filled(rect.center(), DOT, CORRECTION);
            card_text(ui, Words::made(&line), false);
        });
    }
}

/// The lightness bar from black to white: the crop's lightness ticked over it, yours under it
/// (red when it left the crop's), and both in numbers (Oklab `L` × 100) with the difference.
fn lightness(ui: &mut egui::Ui, original: Rgb8, mine: Rgb8) {
    let [l0, l1] = [original, mine].map(|c| oklab(c)[0].clamp(0.0, 1.0));
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, BAR + 2.0 * TICK), Sense::hover());
    let bar = egui::Rect::from_min_size(
        rect.min + Vec2::new(0.0, TICK),
        Vec2::new(rect.width(), BAR),
    );
    let painter = ui.painter();
    let mut mesh = egui::Mesh::default();
    for step in 0..=16u8 {
        let l = f64::from(step) / 16.0;
        let x = bar.left() + bar.width() * l as f32;
        let grey = colour(from_oklab([l, 0.0, 0.0]));
        mesh.colored_vertex(Pos2::new(x, bar.top()), grey);
        mesh.colored_vertex(Pos2::new(x, bar.bottom()), grey);
        if step > 0 {
            let i = 2 * u32::from(step);
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i + 1, i);
        }
    }
    painter.add(Shape::mesh(mesh));
    painter.rect_stroke(
        bar,
        0.0,
        Stroke::new(1.0, DIVIDER),
        egui::StrokeKind::Outside,
    );
    let at = |l: f64| bar.left() + bar.width() * l as f32;
    let off = (l1 - l0).abs() > LIGHTNESS_OFF;
    let mine_ink = if off { CORRECTION } else { INK };
    let above = [
        Pos2::new(at(l0), rect.top()),
        Pos2::new(at(l0), bar.bottom()),
    ];
    painter.line_segment(above, Stroke::new(2.5, INK));
    let under = [
        Pos2::new(at(l1), bar.top()),
        Pos2::new(at(l1), rect.bottom()),
    ];
    painter.line_segment(under, Stroke::new(2.5, mine_ink));
    let numbers = format!(
        "{:.0} → {:.0}   {:+.0}",
        l0 * 100.0,
        l1 * 100.0,
        (l1 - l0) * 100.0
    );
    ui.add_space(SPACE / 2.0);
    ui.vertical_centered(|ui| card_text(ui, Words::made(&numbers), false));
}
