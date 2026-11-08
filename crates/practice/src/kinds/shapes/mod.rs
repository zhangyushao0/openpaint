//! Single shapes (`Kind::Shape`): a pack picture of a tier copied as a line drawing by its outline,
//! its ground faded in the original's frame (S99, `ground.rs`); compared by the picture and its
//! outline laid over the copy, fitted for Aligned (`marks.rs`).

pub(crate) mod demo;
pub(crate) mod ground;
pub(crate) mod marks;

use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, Turn, Work};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, line};
use crate::packs::pack::{ItemRef, Tier};
use crate::sessions::drill::in_turn;
use crate::sessions::family::Family;
use crate::sessions::plan::Exercise;
use crate::sessions::session::Picker;

pub(crate) struct Shapes;

const FACTS: [Facts; 1] = [Facts::of(Family::Shape, "shape", Work::Lines)
    .needs(Needs::Pictures)
    .grounded()];
/// How many shapes a drill holds.
const DRILLED: usize = 3;
/// A jug's outline, the shape's tile.
const JUG: [[f32; 2]; 10] = [
    [22.0, 8.0],
    [32.0, 8.0],
    [31.0, 12.0],
    [37.0, 17.0],
    [38.0, 25.0],
    [34.0, 32.0],
    [20.0, 32.0],
    [16.0, 25.0],
    [17.0, 17.0],
    [23.0, 12.0],
];

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
        Some(demo::shape())
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

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// A jug's outline.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        vec![line(&closed(&JUG))]
    }
}
