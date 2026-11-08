//! Single shapes (`Kind::Shape`): a pack's picture copied by its outline (`outlines/`, aligned by
//! `fits/align_shape.rs`), its ground faded (S99). Its How shows a generated vase
//! (`demo_drawn.rs`'s).

pub(crate) mod ground;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::lines::demo_drawn;
use crate::kinds::{Described, Facts, Kind, Original, Turn, Work};
use crate::marks::mark::SheetMark;
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::in_turn;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;

pub(crate) struct Shapes;

const FACTS: [Facts; 1] = [Facts::of(Family::Shape, "shape", Work::Lines)];
/// How many shapes a drill holds.
const DRILLED: usize = 3;

impl Described for Shapes {
    fn code(&self) -> u8 {
        4
    }

    fn facts(&self) -> &'static [Facts] {
        &FACTS
    }

    fn variant(&self, kind: Kind) -> Option<u8> {
        match kind {
            Kind::Shape(tier) => Some(tier.code()),
            _ => None,
        }
    }

    fn of_variant(&self, variant: u8) -> Option<Kind> {
        Tier::of_code(variant).map(Kind::Shape)
    }

    fn givens(&self, _exercise: &Exercise, _original: &Original) -> Vec<SheetMark> {
        crate::marks::drawing::pictured()
    }

    fn original(&self, exercise: &Exercise) -> Original {
        exercise.picture()
    }

    fn how(&self, _family: Family) -> Option<Demo> {
        Some(demo_drawn::shape())
    }

    /// A drill: three tiers in turn from entry to hard, easiest first, the last the user's own of
    /// the hardest or an easier one when they have one (as stage 2's); fewer, or none, where no
    /// picture of a tier is left.
    fn drill(&self, _family: Family, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let mut tiers = in_turn(&Tier::ALL, turn.turn, DRILLED);
        tiers.sort();
        let mut picker = Picker::new(turn.seed, turn.before, tiers[tiers.len() - 1]);
        picker.shapes(&tiers)
    }
}
