//! Colour combinations (`Kind::Crop`, V大's 临摹色彩组合, S101): a pack's crop, its few colour areas
//! copied by eye and read against the areas the pack holds (`colour_areas/`). Its How plays on a
//! generated example (`demo_colour.rs`'s).

pub(crate) mod marks;
use crate::demos::demo::Demo;
use crate::kinds::colours::demo_colour;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;

pub(crate) struct Crops;

const FACTS: [Facts; 1] = [Facts::of(Family::Areas, "combination", Work::Painted)];
/// How many combinations a drill holds.
const DRILLED: usize = 2;

impl Described for Crops {
    fn code(&self) -> u8 {
        18
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Crop(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Crop)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_colour::areas())
    }

    /// A drill: two crops not shown before while any are left.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picker = Picker::new(turn.seed, turn.before, Tier::Simple);
        (0..DRILLED).filter_map(|_| picker.crop()).collect()
    }
}
