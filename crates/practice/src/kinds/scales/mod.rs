//! Value scales (`Kind::Scale`, S97): a row of cells between given ends, filled in even steps of
//! Oklab lightness; read from the copy's pixels (`marks.rs`). Its How plays a scale of the row in
//! hand's count (`demo.rs`).

pub(crate) mod demo;
pub(crate) mod marks;
pub(crate) mod scale;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Picks, Tools, Turn, Work, place};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, square};
use crate::packs::pack::ItemRef;
use crate::records::record::SCALE_ROWS;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::FULL;
use scale::{CELLS, FIRST_CELLS, ScaleTone, ValueScale};

pub(crate) struct Scales;

const FACTS: [Facts; 1] = [Facts::of(Family::Scale, "scale", Work::Painted)
    .constructed()
    .tools(Tools::Cells)
    .picks(Picks::Ends)
    .rows(&SCALE_ROWS)];
/// How many scales a drill holds.
const DRILLED: usize = 2;

impl Described for Scales {
    fn code(&self) -> u8 {
        6
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    /// Its tone and twice its cells (S97); cells 5 to 9 read (a 1.14.0 scale holds 6 or 7).
    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Scale(tone, cells) => Some(place(&ScaleTone::ALL, &tone) + 2 * cells),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        let cells = variant / 2;
        let kept = (usize::from(cells) >= FIRST_CELLS) && usize::from(cells) <= CELLS;
        let tone = ScaleTone::ALL.get(usize::from(variant) % 2)?;
        kept.then_some(Kind::Scale(*tone, cells))
    }

    fn original(&self, exercise: &Exercise) -> Original {
        let Kind::Scale(tone, cells) = exercise.kind else {
            return exercise.picture();
        };
        Original::Scale(ValueScale::generate(
            tone,
            usize::from(cells),
            exercise.seed,
        ))
    }

    fn givens(&self, _exercise: &Exercise, original: &Original) -> Vec<SheetMark> {
        original.scale().map(marks::givens).unwrap_or_default()
    }

    /// Nine cells'.
    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo::scale(CELLS))
    }

    /// The row in hand's (S97).
    fn how_of(&self, _family: Family, cells: usize) -> Option<Demo> {
        Some(demo::scale(cells))
    }

    /// A drill: two full scales, their tones in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&ScaleTone::ALL, DRILLED), turn, |tone| {
            Kind::Scale(tone, FULL)
        })
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// Five cells, darker to the right.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        (0..5)
            .flat_map(|cell| {
                let left = 8.0 + 7.6 * cell as f32;
                let corners = square([left, 14.0], [left + 7.6, 26.0]);
                let share = 0.08 + 0.18 * cell as f32;
                [
                    TileMark::Tint(corners.to_vec(), share),
                    TileMark::Line(closed(&corners)),
                ]
            })
            .collect()
    }
}
