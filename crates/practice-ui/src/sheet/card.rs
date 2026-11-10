//! What the practice card says: how to practise each family (S92: its title, what it trains and
//! its week, V大's way played on an example with its numbered steps, `demo.rs`, and once
//! what Compare shows, `text/practice_demo*.rs`; Again and Got it); a comparison, which shows
//! rather than tells (D108): its title, then the items its comparison gives whatever its kind
//! (S109, `card_items.rs`): in place or aligned, a line of words, Construction, Grey, the
//! slider, the areas' chips, colours on the wheel and in the colour table, a key shift's disc
//! (`card_disc.rs`) and lightness bar, a division's table; and a session's summary (the
//! colour judgement's radar over the earlier sessions', the colour means beside theirs). The words
//! are `text/practice.rs`'s, in the pass's language.

use openpaint_app::Hsv;
use openpaint_core::Color;
use openpaint_practice::{
    Axis, CELLS, CardItem, Family, HUELESS, KindSummary, Metric, Original, Rgb8, WheelSize, oklch,
};
use openpaint_practice_app::{Finished, PracticeIntent, PracticeProjection};

use openpaint_ui::components::{
    animate_from, card_columns, card_text, choices, color_compare, hint, keep_moving, text_button,
    text_choice_wide,
};
use openpaint_ui::design::motion::OPEN;
use openpaint_ui::design::{DIALOG_WIDTH, INK_SECONDARY, SPACE, TOUCH_TARGET};
use openpaint_ui::taps::push_once;

use super::card_disc::disc;
use super::card_items as items;
use super::demo::{Demonstrations, HowOf};
use crate::components::{RadarAxis, colour_table, paint_seal, paint_stretch, radar, seal_turn};
use crate::palette::CORRECTION;
use crate::text::{self as t, Words, language};

/// The comparison wheel's side on the card, in points: a colour's, a colour copy's point
/// compared, and a colour combination's (smaller, so the card fits under the exercise's tools on
/// a landscape screen).
const COMPARE_WHEEL: f32 = 280.0;
const STUDY_WHEEL: f32 = 240.0;
const AREAS_WHEEL: f32 = 180.0;
/// The summary's radar's side when the colour combinations' and value scales' rows or a new seal
/// share the card with it.
const SUMMARY_RADAR: f32 = 260.0;
/// A finish's road's height and a new seal's side, in points.
const ROAD: f32 = 7.0 * SPACE;
const BIG_SEAL: f32 = 84.0;
/// Seconds after the summary first shows that its dot inks in (the card has grown by then) and
/// that a new seal stamps down.
const INK_AFTER: f64 = 0.4;
const STAMP_AFTER: f64 = 0.75;
/// Under a How card's body: its foot (Again, Got it), the card's margin and a gap to the screen's
/// edge; and the least of its body that shows, in points.
const HOW_FOOT: f32 = TOUCH_TARGET + 4.0 * SPACE;
const HOW_LEAST: f32 = 12.0 * SPACE;

/// The How `practice` shows for `family`: a value scale's of the cells of the row in hand (S97),
/// nine where none is.
pub(crate) fn how_of(practice: &PracticeProjection, family: Family) -> HowOf {
    let shown = practice
        .current
        .and_then(|index| practice.sheet.originals.get(index));
    let cells = shown.and_then(Original::cells).unwrap_or(CELLS);
    HowOf::new(family, cells)
}

/// A family's How (S92): its title, what it trains and its week, its demonstration playing with
/// its numbered steps (`demos`; a value scale's of the row in hand's cells), what Compare's marks
/// mean, and Again beside Got it; returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, of: HowOf, demos: &mut Demonstrations) -> bool {
    let family = of.family;
    let how = t::how(family);
    let steps = t::how_steps(family, of.cells(), language(ui.ctx()));
    ui.heading(Words::from(how.heading).shown(ui.ctx()));
    // On a screen too short for the whole card its body scrolls; Again and Got it stay in reach.
    let room = ui.ctx().content_rect().bottom() - ui.cursor().top() - HOW_FOOT;
    egui::ScrollArea::vertical()
        .id_salt("how")
        .max_height(room.max(HOW_LEAST))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if let Some(trains) = how.trains {
                card_text(ui, trains, true);
            }
            demos.show(ui, of, &steps);
            for line in how.says {
                card_text(ui, *line, false);
            }
        });
    // At the top of its row: centred, the row would take the height the card had before (a
    // summary's) and keep it.
    ui.horizontal_top(|ui| {
        let again = how.trains.is_some() && text_button(ui, t::AGAIN, true).clicked();
        if again {
            demos.again(ui.ctx());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            text_button(ui, t::GOT_IT, true).clicked()
        })
        .inner
    })
    .inner
}

/// The shown exercise as compared: its title (a colour copy's name), then its comparison's items
/// (S109), whatever its kind.
pub(crate) fn compared(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<PracticeIntent>,
) {
    let Some(family) = practice.family else {
        return;
    };
    if !practice.comparing {
        return;
    }
    let language = language(ui.ctx());
    let heading = if practice.copy {
        t::shown_name(ui.ctx(), practice.sheet.name.as_str()).into_owned()
    } else {
        t::exercise(language, (family, practice.warming_up), practice.place.0)
    };
    ui.heading(heading);
    for item in &practice.card {
        item_of(ui, practice, item, taps);
    }
}

/// One of the card's items.
fn item_of(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    item: &CardItem,
    taps: &mut Vec<PracticeIntent>,
) {
    let chosen = practice.area;
    match item {
        CardItem::Reading => card_text(ui, t::READING_COPY, true),
        CardItem::Overlaid => lines(ui, practice, taps),
        CardItem::Says(say) => items::says(ui, say),
        CardItem::Failed(say) => items::failed(ui, say),
        CardItem::Construction => {
            let on = practice.construction;
            items::toggle(
                ui,
                ("construction", t::CONSTRUCTION, on),
                taps,
                PracticeIntent::Construction,
            );
        }
        CardItem::Grey => {
            items::toggle(
                ui,
                ("grey", t::GREY_VIEW, practice.grey),
                taps,
                PracticeIntent::Grey,
            );
        }
        CardItem::Slider => items::shown_slider(ui, practice.shown, taps),
        CardItem::Chips {
            originals,
            copy,
            marked,
        } => items::chips(ui, (originals, copy.as_deref()), (chosen, marked), taps),
        CardItem::Wheel {
            original,
            copy,
            size,
        } => pair_on_wheel(ui, (*original, *copy), wheel_side(*size)),
        CardItem::Disc {
            originals,
            copy,
            tendency,
            mask,
        } => disc(ui, (originals, copy), *tendency, mask, chosen),
        CardItem::Bar { original, copy } => items::lightness(ui, *original, *copy),
        CardItem::Levels(rows) => items::levels(ui, rows),
    }
}

/// The wheel's side on the card, in points.
const fn wheel_side(size: WheelSize) -> f32 {
    match size {
        WheelSize::Large => COMPARE_WHEEL,
        WheelSize::Middle => STUDY_WHEEL,
        WheelSize::Small => AREAS_WHEEL,
    }
}

/// A drawn exercise compared (a line, a group of them, a single shape): in place or aligned
/// (while its fit is found, saying so).
fn lines(ui: &mut egui::Ui, practice: &PracticeProjection, taps: &mut Vec<PracticeIntent>) {
    ui.horizontal(|ui| {
        choices(ui, "overlay", |ui, group| {
            for (aligned, word) in [(false, t::IN_PLACE), (true, t::ALIGNED)] {
                let chosen = (practice.aligned && practice.aligned_offered) == aligned;
                let offered = !aligned || practice.aligned_offered;
                let tapped = text_choice_wide(ui, group, word, (offered, chosen), 0.0).clicked();
                if tapped && !chosen && offered {
                    push_once(taps, PracticeIntent::Aligned(aligned));
                }
            }
        });
        if practice.fitting {
            hint(ui, t::ALIGNING);
        }
    });
}

/// `original` and `mine` on the wheel (`side` points), the way from one to the other, and the
/// colour table under it.
pub(crate) fn pair_on_wheel(ui: &mut egui::Ui, (original, mine): (Rgb8, Rgb8), side: f32) {
    let as_hsv = |rgb: Rgb8| {
        let [r, g, b] = rgb.map(|c| f32::from(c) / 255.0);
        Color::rgb(r, g, b).map_or(
            Hsv {
                h: 0.0,
                s: 0.0,
                v: 0.0,
            },
            Hsv::of,
        )
    };
    // A colour shows on the ring wherever the eye sees a hue: an exact grey's wheel hue is noise.
    let hued = [mine, original].map(|c| oklch(c)[1] >= HUELESS);
    ui.vertical_centered(|ui| {
        let colours = (as_hsv(mine), as_hsv(original));
        color_compare(ui, colours, (hued, CORRECTION), side);
    });
    colour_table(ui, (original, mine));
}

/// The summary: what was compared, the colour judgement over the earlier sessions', and the
/// colour means beside theirs.
pub(crate) fn summary(ui: &mut egui::Ui, practice: &PracticeProjection, kinds: &[KindSummary]) {
    let language = language(ui.ctx());
    ui.heading(done_heading(language, practice));
    if let Some(finished) = &practice.finished {
        finish(ui, finished, (practice.stage, practice.number));
    }
    let sealed = (practice.finished.as_ref()).is_some_and(|f| !f.sealed.is_empty());
    for kind in kinds {
        let line = t::compared_of(language, kind.family, (kind.checked, kind.held));
        card_text(ui, Words::made(&line), false);
    }
    // The kind the summary judges (the colours' radar, D108), its rows under it.
    let Some((colours, [this, before])) =
        (kinds.iter()).find_map(|kind| kind.judgement.map(|judged| (kind, judged)))
    else {
        items::summary_rows(ui, kinds);
        return;
    };
    let axes: Vec<RadarAxis<'_>> = Axis::ALL
        .iter()
        .map(|&axis| {
            (
                t::judgement_axis(axis).get(language),
                share(this.of(axis)),
                share(before.of(axis)),
            )
        })
        .collect();
    // Smaller where the combinations' and scales' rows follow, so the card fits the screen.
    let more =
        sealed || (kinds.iter()).any(|kind| kind.judgement.is_none() && !kind.rows.is_empty());
    let side = if more {
        SUMMARY_RADAR
    } else {
        DIALOG_WIDTH - 4.0 * SPACE
    };
    radar(ui, &axes, side);
    if let Some(head) = t::kind_words(colours.family).rows {
        let heads = [head, t::THIS_SESSION, t::BEFORE].map(|head| head.get(language));
        card_columns(ui, &heads, true);
    }
    for (metric, this, before) in &colours.rows {
        let shown =
            |mean: &Option<f64>| mean.map_or_else(|| "—".to_owned(), |v| metric_text(*metric, v));
        let name = t::metric(*metric).get(language);
        card_columns(ui, &[name, &shown(this), &shown(before)], false);
    }
    items::summary_rows(ui, kinds);
}

/// A finished session's heading ("Session 3 done"), or a drill's ("Centre drill done").
pub(crate) fn done_heading(language: t::Language, practice: &PracticeProjection) -> String {
    match practice.drill {
        Some(family) => t::drill_done(language, family),
        None => t::session_done(language, practice.number),
    }
}

/// A session's finish (S89): its dot on its stage's road inks in, then each seal its work earned
/// stamps down with its name under it, on the interface's spring (D44's acknowledge); a drill's
/// (S91) only its seals, for it has no dot on the road.
fn finish(ui: &mut egui::Ui, finished: &Finished, session: (Option<u8>, u32)) {
    let ctx = ui.ctx().clone();
    let id = egui::Id::new(("practice finish", session));
    // Each motion waits until the card has grown and the one before it is under way.
    let now = ctx.input(|input| input.time);
    let first = ctx.data_mut(|data| *data.get_temp_mut_or_insert_with(id.with("seen"), || now));
    let after = |wait: f64| {
        if now - first < wait {
            keep_moving(&ctx);
            0.0
        } else {
            1.0
        }
    };
    let width = ui.available_width();
    if let Some(stretch) = finished.road {
        let inked = animate_from(&ctx, id.with("dot"), 0.0, after(INK_AFTER), OPEN);
        let (road, _) = ui.allocate_exact_size(egui::vec2(width, ROAD), egui::Sense::hover());
        paint_stretch(ui.painter(), road, stretch, Some(inked));
    }
    let language = language(&ctx);
    for (place, seal) in finished.sealed.iter().enumerate() {
        let wait = STAMP_AFTER + 0.3 * place as f64;
        let stamped = animate_from(&ctx, id.with(place), 0.0, after(wait), OPEN);
        let room = egui::vec2(width, BIG_SEAL * 1.4);
        let (rect, _) = ui.allocate_exact_size(room, egui::Sense::hover());
        let side = BIG_SEAL * (1.4 - 0.4 * stamped);
        let lines = t::seal_lines(language, *seal);
        let turn = seal_turn(place + 1);
        paint_seal(ui.painter(), (rect.center(), side), (&lines, turn), stamped);
        let named = egui::RichText::new(t::new_seal(language, *seal))
            .color(INK_SECONDARY.gamma_multiply(stamped));
        ui.vertical_centered(|ui| ui.label(named));
    }
}

fn share(value: Option<f64>) -> Option<f32> {
    value.map(|v| v as f32)
}

/// A metric's value in its plain unit: colour differences × 100, as ΔE is shown.
pub(crate) fn metric_text(metric: Metric, value: f64) -> String {
    match metric {
        Metric::FirstDelta | Metric::FinalDelta | Metric::AreasDelta => {
            format!("{:.1}", value * 100.0)
        }
        Metric::FirstLightness
        | Metric::FinalLightness
        | Metric::FirstChroma
        | Metric::FinalChroma
        | Metric::FirstHue
        | Metric::FinalHue => format!("{:+.1}", value * 100.0),
        Metric::ScaleOff
        | Metric::SpaceOff
        | Metric::SolidOff
        | Metric::ObjectOff
        | Metric::DivisionLength(_)
        | Metric::MassesMatched => format!("{:.0} %", value * 100.0),
        Metric::DivisionLines(_)
        | Metric::KeyLightness
        | Metric::KeyUnmoved
        | Metric::KeyMerged => format!("{value:.0}"),
        Metric::PointAcross | Metric::PointDown | Metric::ProportionOff => {
            format!("{:+.1} %", value * 100.0)
        }
        Metric::AngleOff => format!("{value:+.1}°"),
        Metric::Corrections
        | Metric::AreasWeak
        | Metric::AreasMadeStrong
        | Metric::ScaleUneven
        | Metric::ScaleReversed => format!("{value:.1}"),
    }
}
