//! Picture divisions (`Kind::Division`, S95, V大's 画面分割): a division's levels read and measured,
//! and its How. The levels themselves are the compositions' (`compositions/levels.rs`).

pub(crate) mod demo_division;
pub(crate) mod division;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::compositions;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct Divisions;

const FACTS: [Facts; 1] = [Facts::of(Family::Division, "division", Work::Divided).composed()];
/// How many divisions a drill holds.
const DRILLED: usize = 3;

impl Described for Divisions {
    fn code(&self) -> u8 {
        15
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Division(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Division)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_division::division())
    }

    /// A drill: three compositions, the simple and medium tiers in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        compositions(DRILLED, turn, Kind::Division)
    }
}
