//! Value scales (`Kind::Scale`, S97): a row of cells on Oklab L, read and checked. Its How is the
//! colours' (`demo_colour.rs`), of the row in hand's count.

pub(crate) mod marks;
pub(crate) mod scale;

use crate::demos::demo::Demo;
use crate::kinds::colours::demo_colour;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work, place};
use crate::marks::mark::SheetMark;
use crate::packs::pack::ItemRef;
use crate::sessions::drill::turned;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::FULL;
use scale::{CELLS, FIRST_CELLS, ScaleTone, ValueScale};

pub(crate) struct Scales;

const FACTS: [Facts; 1] = [Facts::of(Family::Scale, "scale", Work::Painted).constructed()];
/// How many scales a drill holds.
const DRILLED: usize = 2;

impl Described for Scales {
    fn code(&self) -> u8 {
        6
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    /// Its tone and twice its cells (S97; 1.5.0 to 1.14.0 stored its tone alone, the cells drawn
    /// from the seed: the store's upgrade turns it into this, `store/src/practice.rs`).
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
        match original {
            Original::Scale(scale) => marks::givens(scale),
            _ => Vec::new(),
        }
    }

    /// Nine cells'.
    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_colour::scale(CELLS))
    }

    /// The row in hand's (S97).
    fn how_of(&self, _family: Family, cells: usize) -> Option<Demo> {
        Some(demo_colour::scale(cells))
    }

    /// A drill: two full scales, their tones in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        turned((&ScaleTone::ALL, DRILLED), turn, |tone| {
            Kind::Scale(tone, FULL)
        })
    }
}
