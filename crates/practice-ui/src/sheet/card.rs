//! What the practice card says: how to practise each kind (S92: its title, what it trains and
//! its week, V大's way played on an example with its numbered steps, `demo.rs`, then how to
//! practise, never what Compare shows, D118; Again and Got it; the first lesson's own,
//! `first.rs`); a comparison, which shows rather than tells (D108): its title, its result among Accurate, Almost and Again
//! (`result.rs`: the app's ringed first, the eye's three to choose from after the items), then the
//! items its comparison gives whatever its kind (S109, `card_items.rs`): in place or aligned, a
//! line of words, Construction, Grey, the slider, the areas' chips, colours on the wheel and in the
//! colour table, a key shift's disc (`card_disc.rs`) and lightness bar, a division's table; and a
//! lesson's end (`end.rs`). The words are `text/`'s, in the pass's language.

use openpaint_app::Hsv;
use openpaint_core::Color;
use openpaint_practice::{CELLS, CardItem, HUELESS, Kind, Rgb8, Verdict, WheelSize, oklch};
use openpaint_practice_app::{PracticeIntent, PracticeProjection};

use openpaint_ui::components::{
    card_text, choices, color_compare, hint, text_button, text_choice_wide,
};
use openpaint_ui::design::{SPACE, TOUCH_TARGET};
use openpaint_ui::taps::push_once;

use super::card_disc::disc;
use super::card_items as items;
use super::demo::{Demonstrations, HowOf};
use super::lesson_name;
use super::result::result_row;
use crate::components::colour_table;
use crate::palette::CORRECTION;
use crate::text::{self as t, Words, language};

/// The comparison wheel's side on the card, in points: a colour's, a colour copy's point
/// compared, and a colour combination's (smaller, so the card fits under the exercise's tools on
/// a landscape screen).
const COMPARE_WHEEL: f32 = 280.0;
const STUDY_WHEEL: f32 = 240.0;
const AREAS_WHEEL: f32 = 180.0;
/// Under a How card's body: its foot (Again, Got it), the card's margin and a gap to the screen's
/// edge; and the least of its body that shows, in points.
const HOW_FOOT: f32 = TOUCH_TARGET + 4.0 * SPACE;
const HOW_LEAST: f32 = 12.0 * SPACE;

/// The How `practice` shows for `kind`: a value scale's of the cells of the row in hand (S97),
/// nine where none is.
pub(crate) fn how_of(practice: &PracticeProjection, kind: Kind) -> HowOf {
    let cells = (practice.original.as_deref())
        .and_then(|original| original.cells())
        .unwrap_or(CELLS);
    HowOf::new(kind, cells)
}

/// A kind's How (S92): its title, what it trains and its week, its demonstration playing with
/// its numbered steps (`demos`; a value scale's of the row in hand's cells), how to practise, and
/// Again beside Got it; returns whether Got it was tapped.
pub(crate) fn instructions(ui: &mut egui::Ui, of: HowOf, demos: &mut Demonstrations) -> bool {
    let how = t::how(of.kind());
    let steps = t::how_steps(of.kind(), of.cells(), language(ui.ctx()));
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
    // At the top of its row: centred, the row would take the height the card had before and keep
    // it.
    ui.horizontal_top(|ui| {
        let again = how.trains.is_some() && text_button(ui, t::WATCH_AGAIN, true).clicked();
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

/// The shown exercise as compared: its title (a colour copy's name), its result where the app
/// judges it, its comparison's items (S109) whatever its kind, then where the eye judges the three
/// to choose from.
pub(crate) fn compared(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<PracticeIntent>,
) {
    if practice.kind.is_none() || !practice.comparing {
        return;
    }
    heading(ui, practice);
    let judged = practice.judged;
    let eye = judged.is_some_and(|judged| judged.verdict == Verdict::Eye);
    if let Some(judged) = judged.filter(|_| !eye) {
        result_row(ui, judged, taps);
    }
    for item in &practice.card {
        item_of(ui, practice, item, taps);
    }
    if let Some(judged) = judged.filter(|_| eye) {
        result_row(ui, judged, taps);
    }
}

/// The shown exercise while it is made, where its original asks steps of the card (S112): its
/// title, then those steps and the choices they take.
pub(crate) fn task(
    ui: &mut egui::Ui,
    practice: &PracticeProjection,
    taps: &mut Vec<PracticeIntent>,
) {
    heading(ui, practice);
    for item in &practice.task {
        item_of(ui, practice, item, taps);
    }
}

/// The card's title: a colour copy's name, else the lesson's and the exercise's place.
fn heading(ui: &mut egui::Ui, practice: &PracticeProjection) {
    let heading = if practice.copy {
        t::shown_name(ui.ctx(), practice.sheet.name.as_str()).into_owned()
    } else {
        let place = practice.lesson.map_or(1, |lesson| lesson.place.0);
        let name = lesson_name(practice.lesson, practice.kind);
        t::exercise(language(ui.ctx()), name, place)
    };
    ui.heading(heading);
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
        // The choices are fixed from the first Compare on (D99).
        CardItem::Toggles(keys) => {
            let open = practice.result.is_none();
            items::toggles(ui, keys, (practice.choices, open), taps);
        }
        CardItem::Shapes(shapes) => {
            let open = practice.result.is_none();
            items::shapes(ui, shapes, (practice.choices, open), taps);
        }
        CardItem::Views(keys) => items::views(ui, keys, practice.view, taps),
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
