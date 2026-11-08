//! Value masses (`Kind::ValueMasses`, S96): a copy in three greys read against the picture's
//! masses (`compositions/value_masses.rs`), and the How.

pub(crate) mod demo_masses;
pub(crate) mod marks;
pub(crate) mod value_masses_read;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::compositions;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;

pub(crate) struct ValueMasses;

const FACTS: [Facts; 1] =
    [Facts::of(Family::ValueMasses, "value-masses", Work::Painted).composed()];
/// How many value masses a drill holds.
const DRILLED: usize = 3;

impl Described for ValueMasses {
    fn code(&self) -> u8 {
        17
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::ValueMasses(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::ValueMasses)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_masses::value_masses())
    }

    /// A drill: three compositions whose masses read, the simple and medium tiers in turn.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        compositions(DRILLED, turn, Kind::ValueMasses)
    }
}
