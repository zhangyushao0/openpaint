//! Single shapes (`Kind::Shape`): a pack picture of a tier copied as a line drawing by its outline,
//! its ground faded in the original's frame (S99, `ground.rs`); compared by the picture and its
//! outline laid over the copy, fitted for Aligned (`marks.rs`).
//!
//! A lesson holds one shape (about 3.5 minutes an easy one, 5.5 a hard one): the first level the
//! easiest of three tiers in turn (entry mostly, as stage 2's first drills), the second a medium
//! picture, the third a hard one; at those two every second lesson the user's own of that tier
//! or an easier one, when they have some.

pub(crate) mod demo;
pub(crate) mod ground;
pub(crate) mod marks;

use crate::course::picker::{Picker, in_turn};
use crate::demos::demo::Demo;
use crate::kinds::{Described, Facts, Kind, Needs, Original, SkillLevel, Tools, Turn};
use crate::marks::asked::{Asked, Compare};
use crate::marks::mark::SheetMark;
use crate::marks::tile::{TileMark, closed, line};
use crate::packs::pack::{ItemRef, Tier};
use crate::sheets::exercise::Exercise;
use crate::sheets::family::Family;

pub(crate) struct Shapes;

const FACTS: [Facts; 1] = [Facts::of(Family::Shape, "shape", Tools::Drawing, 240)
    .aligned()
    .needs(Needs::Pictures)
    .grounded()];
/// The tiers the first level's shape is the easiest of, three in turn (stage 2's drills).
const TURNED: usize = 3;
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

    /// None where no picture is left.
    fn lesson(&self, level: SkillLevel, turn: &Turn<'_>) -> Vec<(Kind, Option<ItemRef>)> {
        let tier = match level {
            SkillLevel::First => {
                let mut tiers = in_turn(&Tier::ALL, turn.turn, TURNED);
                tiers.sort();
                let mut picker = Picker::new(turn, tiers[tiers.len() - 1]);
                let mut shapes = picker.shapes(&tiers);
                shapes.truncate(1);
                return shapes;
            }
            SkillLevel::Second => Tier::Medium,
            SkillLevel::Third => Tier::Hard,
        };
        let mut picker = Picker::new(turn, tier);
        let own = picker.has_own() && turn.turn % 2 == 1;
        picker.shape(tier, own).into_iter().collect()
    }

    fn window(&self) -> usize {
        3
    }

    fn compare(&self, asked: &Asked<'_>) -> Option<Compare> {
        marks::compare(asked)
    }

    /// A jug's outline.
    fn tile(&self, _family: Family) -> Vec<TileMark> {
        vec![line(&closed(&JUG))]
    }
}
